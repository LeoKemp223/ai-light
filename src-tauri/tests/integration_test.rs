use ai_light::aggregator::StateAggregator;
use ai_light::config::AppConfig;
use ai_light::http_server::start_http_server;
use ai_light::types::Status;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn hook_http_server_drives_session_lifecycle() {
    let config_dir = std::env::temp_dir().join(unique_name("ai-light-config"));
    std::fs::create_dir_all(&config_dir).unwrap();
    std::env::set_var("AI_LIGHT_CONFIG_DIR", &config_dir);

    let project_dir = std::env::temp_dir().join(unique_name("ai-light-project"));
    std::fs::create_dir_all(&project_dir).unwrap();

    let aggregator = Arc::new(StateAggregator::new());
    let port = start_http_server(Arc::clone(&aggregator), &AppConfig::default()).unwrap();

    post_event(
        port,
        &format!(
            r#"{{"event_type":"session-start","session_id":"s1","cwd":"{}"}}"#,
            json_path(&project_dir)
        ),
    );

    eventually(|| {
        let lights = aggregator.get_lights();
        lights.len() == 1 && lights[0].status == Status::Idle
    });

    post_event(
        port,
        r#"{"event_type":"prompt-submit","session_id":"s1","tool_call":"shell"}"#,
    );

    eventually(|| {
        let lights = aggregator.get_lights();
        lights.len() == 1
            && lights[0].status == Status::Working
            && lights[0].last_tool_call.as_deref() == Some("shell")
    });

    post_event(port, r#"{"event_type":"stop","session_id":"s1"}"#);

    eventually(|| {
        let lights = aggregator.get_lights();
        lights.len() == 1 && lights[0].status == Status::Done
    });

    post_event(port, r#"{"event_type":"notification","session_id":"s1"}"#);
    post_event(port, r#"{"event_type":"post-tool-use","session_id":"s1"}"#);
    post_event(
        port,
        r#"{"event_type":"permission-request","session_id":"s1"}"#,
    );

    eventually(|| {
        let lights = aggregator.get_lights();
        lights.len() == 1 && lights[0].status == Status::Done
    });

    post_event(port, r#"{"event_type":"prompt-submit","session_id":"s1"}"#);

    eventually(|| {
        let lights = aggregator.get_lights();
        lights.len() == 1 && lights[0].status == Status::Working
    });

    post_event(port, r#"{"event_type":"session-end","session_id":"s1"}"#);

    eventually(|| {
        let lights = aggregator.get_lights();
        lights.is_empty()
    });

    let _ = std::fs::remove_dir_all(project_dir);
    let _ = std::fs::remove_dir_all(config_dir);
    std::env::remove_var("AI_LIGHT_CONFIG_DIR");
}

#[test]
fn hook_http_server_respects_fixed_port_config() {
    let config_dir = std::env::temp_dir().join(unique_name("ai-light-fixed-port-config"));
    std::fs::create_dir_all(&config_dir).unwrap();
    std::env::set_var("AI_LIGHT_CONFIG_DIR", &config_dir);

    let probe = TcpListener::bind("127.0.0.1:0").unwrap();
    let fixed_port = probe.local_addr().unwrap().port();
    drop(probe);

    let aggregator = Arc::new(StateAggregator::new());
    let config = AppConfig {
        http_port: Some(fixed_port),
        ..AppConfig::default()
    };

    let port = start_http_server(aggregator, &config).unwrap();
    assert_eq!(port, fixed_port);

    let _ = std::fs::remove_dir_all(config_dir);
    std::env::remove_var("AI_LIGHT_CONFIG_DIR");
}

fn post_event(port: u16, body: &str) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let request = format!(
        "POST /events HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    stream.write_all(request.as_bytes()).unwrap();

    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
}

fn eventually(mut predicate: impl FnMut() -> bool) {
    for _ in 0..20 {
        if predicate() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    assert!(predicate());
}

fn unique_name(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    format!("{prefix}-{nanos}")
}

fn json_path(path: &PathBuf) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}
