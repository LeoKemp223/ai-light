use ai_light::hook_installer::{hook_binary_is_current, merge_hooks};
use serde_json::json;
use std::path::Path;

#[test]
fn merge_hooks_creates_hooks_object_when_missing() {
    let merged = merge_hooks(json!({}), Path::new("/path/to/ai-light-hook")).unwrap();

    let hooks = merged.get("hooks").unwrap();
    assert!(hooks.get("SessionStart").is_some());
    assert!(hooks.get("UserPromptSubmit").is_some());
    assert!(hooks.get("PreToolUse").is_some());
    assert!(hooks.get("PermissionRequest").is_some());
    assert!(hooks.get("PostToolUse").is_some());
    assert!(hooks.get("Notification").is_some());
    assert!(hooks.get("Stop").is_some());
    assert!(hooks.get("SessionEnd").is_some());
}

#[test]
fn merge_hooks_preserves_existing_settings_and_hooks() {
    let existing = json!({
        "hooks": {
            "PreToolUse": [{
                "matcher": "",
                "hooks": [{"type": "command", "command": "echo test"}]
            }]
        },
        "theme": "dark"
    });

    let merged = merge_hooks(existing, Path::new("/path/to/ai-light-hook")).unwrap();

    assert!(merged["hooks"].get("PreToolUse").is_some());
    assert!(merged["hooks"].get("SessionStart").is_some());
    assert_eq!(merged["theme"], "dark");

    let pre_tool_use = merged["hooks"]["PreToolUse"].as_array().unwrap();
    assert!(pre_tool_use
        .iter()
        .any(|entry| entry["hooks"][0]["command"] == "echo test"));
    assert!(pre_tool_use.iter().any(|entry| entry["hooks"][0]["command"]
        .as_str()
        .unwrap()
        .contains("ai-light-hook")));
}

#[test]
fn merge_hooks_preserves_existing_hooks_for_same_event() {
    let existing = json!({
        "hooks": {
            "SessionStart": [{
                "matcher": "",
                "hooks": [{"type": "command", "command": "echo existing"}]
            }]
        }
    });

    let merged = merge_hooks(existing, Path::new("/path/to/ai-light-hook")).unwrap();
    let session_start = merged["hooks"]["SessionStart"].as_array().unwrap();

    assert_eq!(session_start.len(), 2);
    assert!(session_start
        .iter()
        .any(|entry| entry["hooks"][0]["command"] == "echo existing"));
    assert!(session_start
        .iter()
        .any(|entry| entry["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .contains("ai-light-hook")));
}

#[test]
fn merge_hooks_replaces_existing_ai_light_hooks_for_same_event() {
    let existing = json!({
        "hooks": {
            "SessionStart": [{
                "matcher": "",
                "hooks": [{"type": "command", "command": "/old/ai-light-hook session-start"}]
            }]
        }
    });

    let merged = merge_hooks(existing, Path::new("/new/ai-light-hook")).unwrap();
    let session_start = merged["hooks"]["SessionStart"].as_array().unwrap();

    assert_eq!(session_start.len(), 1);
    assert_eq!(
        session_start[0]["hooks"][0]["command"],
        "/new/ai-light-hook session-start"
    );
}

#[test]
fn merge_hooks_rejects_non_object_hooks_field() {
    let result = merge_hooks(json!({"hooks": []}), Path::new("/path/to/ai-light-hook"));

    assert!(result.is_err());
}

#[test]
fn merge_hooks_rejects_non_array_event_field() {
    let result = merge_hooks(
        json!({"hooks": {"SessionStart": {}}}),
        Path::new("/path/to/ai-light-hook"),
    );

    assert!(result.is_err());
}

#[test]
fn hook_binary_current_compares_file_content() {
    let dir = std::env::temp_dir().join(unique_name("ai-light-hook-current"));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("source-hook");
    let destination = dir.join("destination-hook");

    std::fs::write(&source, "same-size-a").unwrap();
    std::fs::write(&destination, "same-size-b").unwrap();
    assert!(!hook_binary_is_current(&source, &destination).unwrap());

    std::fs::write(&destination, "same-size-a").unwrap();
    assert!(hook_binary_is_current(&source, &destination).unwrap());

    std::fs::remove_dir_all(dir).unwrap();
}

fn unique_name(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    format!("{prefix}-{nanos}")
}
