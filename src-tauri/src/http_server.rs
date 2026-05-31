use crate::aggregator::StateAggregator;
use crate::config::{load_runtime_config, save_runtime_config, AppConfig, RuntimeConfig};
use crate::types::{Status, Tool};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookEvent {
    pub event_type: String,
    #[serde(default = "default_session_id", alias = "sessionId")]
    pub session_id: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default, alias = "tool", alias = "toolName", alias = "tool_name")]
    pub tool_call: Option<String>,
}

impl HookEvent {
    pub fn event_type_to_status(event_type: &str) -> Option<Status> {
        match event_type {
            "session-start" => Some(Status::Idle),
            "prompt-submit" => Some(Status::Working),
            "pre-tool-use" => Some(Status::Working),
            "permission-request" => Some(Status::Error),
            "post-tool-use" => Some(Status::Working),
            "notification" => Some(Status::Error),
            "stop" => Some(Status::Done),
            "session-end" => None,
            _ => None,
        }
    }
}

pub fn parse_hook_event(payload: &str) -> Result<HookEvent, serde_json::Error> {
    serde_json::from_str(payload)
}

pub fn existing_instance_is_healthy() -> bool {
    let Some(runtime) = load_runtime_config() else {
        return false;
    };

    let address = SocketAddr::from(([127, 0, 0, 1], runtime.http_port));

    TcpStream::connect_timeout(&address, Duration::from_millis(250))
        .and_then(|mut stream| {
            stream.set_read_timeout(Some(Duration::from_millis(250)))?;
            stream.set_write_timeout(Some(Duration::from_millis(250)))?;
            stream.write_all(
                b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
            )?;

            let mut response = String::new();
            stream.read_to_string(&mut response)?;
            Ok(response.starts_with("HTTP/1.1 200 OK"))
        })
        .unwrap_or(false)
}

pub fn start_http_server(
    aggregator: Arc<StateAggregator>,
    app_config: &AppConfig,
) -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
    let bind_address = format!(
        "{}:{}",
        app_config.http_bind,
        app_config.http_port.unwrap_or(0)
    );
    let listener = TcpListener::bind(bind_address)?;
    let port = listener.local_addr()?.port();

    save_runtime_config(&RuntimeConfig { http_port: port })?;

    thread::Builder::new()
        .name("ai-light-http-server".to_string())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else {
                    continue;
                };

                let aggregator = Arc::clone(&aggregator);
                thread::spawn(move || {
                    let _ = handle_connection(stream, aggregator);
                });
            }
        })?;

    Ok(port)
}

fn default_session_id() -> String {
    "unknown".to_string()
}

fn handle_connection(mut stream: TcpStream, aggregator: Arc<StateAggregator>) -> io::Result<()> {
    let request = read_http_request(&mut stream)?;
    let Some((request_line, body)) = request else {
        return write_response(&mut stream, 400, "Bad Request", "missing request");
    };

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();

    match (method, path) {
        ("GET", "/health") => write_response(&mut stream, 200, "OK", "ok"),
        ("GET", "/state") => {
            let body = serde_json::to_string(&aggregator.get_lights())
                .unwrap_or_else(|_| "[]".to_string());
            write_json_response(&mut stream, 200, "OK", &body)
        }
        ("POST", "/events") => match parse_hook_event(&body) {
            Ok(event) => {
                apply_hook_event(&aggregator, event);
                write_response(&mut stream, 200, "OK", "ok")
            }
            Err(_) => write_response(&mut stream, 400, "Bad Request", "invalid json"),
        },
        _ => write_response(&mut stream, 404, "Not Found", "not found"),
    }
}

fn read_http_request(stream: &mut TcpStream) -> io::Result<Option<(String, String)>> {
    let mut buffer = Vec::new();
    let mut chunk = [0; 1024];

    loop {
        let bytes_read = stream.read(&mut chunk)?;
        if bytes_read == 0 {
            break;
        }

        buffer.extend_from_slice(&chunk[..bytes_read]);

        if let Some((header_end, separator_len)) = find_header_split(&buffer) {
            let headers = String::from_utf8_lossy(&buffer[..header_end]).to_string();
            let request_line = headers.lines().next().unwrap_or_default().to_string();
            let content_length = parse_content_length(&headers);
            let body_start = header_end + separator_len;

            while buffer.len() < body_start + content_length {
                let bytes_read = stream.read(&mut chunk)?;
                if bytes_read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..bytes_read]);
            }

            let body_end = (body_start + content_length).min(buffer.len());
            let body = String::from_utf8_lossy(&buffer[body_start..body_end]).to_string();

            return Ok(Some((request_line, body)));
        }

        if buffer.len() > 64 * 1024 {
            return Ok(None);
        }
    }

    Ok(None)
}

fn apply_hook_event(aggregator: &StateAggregator, event: HookEvent) {
    match event.event_type.as_str() {
        "session-start" => {
            let cwd = event
                .cwd
                .as_deref()
                .map(PathBuf::from)
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from("."));

            aggregator.add_session(event.session_id, Tool::ClaudeCode, &cwd, Status::Idle);
        }
        "session-end" => {
            aggregator.remove_session(&event.session_id);
        }
        _ => {
            if should_ignore_late_event_after_done(aggregator, &event) {
                return;
            }

            if let Some(status) = HookEvent::event_type_to_status(&event.event_type) {
                aggregator.update_session_status(&event.session_id, status);
            }

            if let Some(tool_call) = event.tool_call {
                aggregator.set_last_tool_call(&event.session_id, tool_call);
            }
        }
    }
}

fn should_ignore_late_event_after_done(aggregator: &StateAggregator, event: &HookEvent) -> bool {
    if aggregator.session_status(&event.session_id) != Some(Status::Done) {
        return false;
    }

    matches!(
        event.event_type.as_str(),
        "pre-tool-use" | "permission-request" | "post-tool-use" | "notification"
    )
}

fn find_header_split(buffer: &[u8]) -> Option<(usize, usize)> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| (position, 4))
        .or_else(|| {
            buffer
                .windows(2)
                .position(|window| window == b"\n\n")
                .map(|position| (position, 2))
        })
}

fn parse_content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0)
}

fn write_response(stream: &mut TcpStream, code: u16, reason: &str, body: &str) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {code} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn write_json_response(
    stream: &mut TcpStream,
    code: u16,
    reason: &str,
    body: &str,
) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {code} {reason}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_content_length_case_insensitively() {
        let headers = "POST /events HTTP/1.1\r\ncontent-length: 42\r\n";

        assert_eq!(parse_content_length(headers), 42);
    }
}
