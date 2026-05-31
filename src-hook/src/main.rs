use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct RuntimeConfig {
    http_port: u16,
}

#[derive(Debug, Serialize)]
struct HookEvent {
    event_type: String,
    session_id: String,
    cwd: Option<String>,
    tool_call: Option<String>,
}

fn main() {
    let Some(event_type) = env::args().nth(1).map(|event| normalize_event_type(&event)) else {
        return;
    };

    let Some(payload) = read_stdin_payload() else {
        return;
    };

    let Some(target_url) = resolve_event_url() else {
        return;
    };

    let event = HookEvent {
        event_type,
        session_id: extract_string(&payload, &["session_id", "sessionId"])
            .unwrap_or_else(|| "unknown".to_string()),
        cwd: extract_string(&payload, &["cwd"]),
        tool_call: extract_string(&payload, &["tool_name", "tool", "toolName"]),
    };

    post_event(&target_url, &event);
}

fn read_stdin_payload() -> Option<serde_json::Value> {
    let mut stdin_content = String::new();
    io::stdin().read_to_string(&mut stdin_content).ok()?;

    if stdin_content.trim().is_empty() {
        return Some(serde_json::Value::Object(serde_json::Map::new()));
    }

    serde_json::from_str(&stdin_content).ok()
}

fn resolve_event_url() -> Option<String> {
    if let Some(url) = env::var_os("AI_LIGHT_URL").and_then(|value| {
        let value = value.to_string_lossy().trim().to_string();
        (!value.is_empty()).then_some(value)
    }) {
        return Some(normalize_event_url(&url));
    }

    let config = load_runtime_config()?;
    Some(format!("http://127.0.0.1:{}/events", config.http_port))
}

fn load_runtime_config() -> Option<RuntimeConfig> {
    let content = fs::read_to_string(runtime_config_path()).ok()?;
    serde_json::from_str(&content).ok()
}

fn runtime_config_path() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_light")
        .join("runtime.json")
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
}

fn extract_string(payload: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        payload
            .get(key)
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
    })
}

fn normalize_event_type(event_type: &str) -> String {
    match event_type {
        "SessionStart" | "session_start" | "sessionstart" => "session-start",
        "UserPromptSubmit" | "prompt_submit" | "user-prompt-submit" | "userpromptsubmit" => {
            "prompt-submit"
        }
        "Notification" | "notification" => "notification",
        "Stop" | "stop" => "stop",
        "SessionEnd" | "session_end" | "sessionend" => "session-end",
        other => other,
    }
    .to_string()
}

fn post_event(url: &str, event: &HookEvent) {
    let client = reqwest::blocking::Client::new();

    let _ = client.post(url).json(event).send();
}

fn normalize_event_url(url: &str) -> String {
    if url.ends_with("/events") {
        url.to_string()
    } else {
        format!("{}/events", url.trim_end_matches('/'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_claude_hook_names() {
        assert_eq!(normalize_event_type("SessionStart"), "session-start");
        assert_eq!(normalize_event_type("UserPromptSubmit"), "prompt-submit");
        assert_eq!(normalize_event_type("SessionEnd"), "session-end");
    }

    #[test]
    fn extracts_first_present_string_key() {
        let payload = serde_json::json!({
            "sessionId": "abc123",
            "cwd": "N:/AI/ai_light"
        });

        assert_eq!(
            extract_string(&payload, &["session_id", "sessionId"]),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn prefers_explicit_event_url_environment_variable() {
        let previous = env::var_os("AI_LIGHT_URL");
        env::set_var("AI_LIGHT_URL", "http://127.0.0.1:32123");

        assert_eq!(
            resolve_event_url(),
            Some("http://127.0.0.1:32123/events".to_string())
        );

        match previous {
            Some(value) => env::set_var("AI_LIGHT_URL", value),
            None => env::remove_var("AI_LIGHT_URL"),
        }
    }
}
