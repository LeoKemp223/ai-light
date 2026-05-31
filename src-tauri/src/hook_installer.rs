use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const HOOK_EVENTS: [(&str, &str); 5] = [
    ("SessionStart", "session-start"),
    ("UserPromptSubmit", "prompt-submit"),
    ("Notification", "notification"),
    ("Stop", "stop"),
    ("SessionEnd", "session-end"),
];

pub fn get_claude_settings_path() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join("settings.json")
}

pub fn get_hook_binary_path() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_light")
        .join("bin")
        .join(hook_binary_name())
}

pub fn install_hook_binary_from_resource(resource_dir: &Path) -> Result<bool, std::io::Error> {
    let Some(source) = bundled_hook_candidates(resource_dir)
        .into_iter()
        .find(|path| path.exists())
    else {
        return Ok(false);
    };

    let destination = get_hook_binary_path();
    if hook_binary_is_current(&source, &destination)? {
        return Ok(false);
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(source, destination)?;
    Ok(true)
}

pub fn merge_hooks(mut existing: Value, hook_path: &Path) -> Result<Value, String> {
    if !existing.is_object() {
        return Err("settings root must be a JSON object".to_string());
    }

    if existing.get("hooks").is_none() {
        existing["hooks"] = json!({});
    }

    let hooks = existing
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "settings hooks field must be a JSON object".to_string())?;

    let command_path = shell_command_path(hook_path);

    for (claude_event, hook_event) in HOOK_EVENTS {
        let event_hooks = hooks
            .entry(claude_event.to_string())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| format!("settings hooks.{claude_event} field must be an array"))?;

        remove_existing_ai_light_hooks(event_hooks);
        event_hooks.push(json!({
            "matcher": "",
            "hooks": [{
                "type": "command",
                "command": format!("{command_path} {hook_event}")
            }]
        }));
    }

    Ok(existing)
}

pub fn install_hooks() -> Result<(), Box<dyn std::error::Error>> {
    let settings_path = get_claude_settings_path();
    let hook_path = get_hook_binary_path();

    if !hook_path.exists() {
        return Err(format!("hook binary not found: {}", hook_path.display()).into());
    }

    let existing = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content)?
    } else {
        json!({})
    };

    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if settings_path.exists() {
        fs::copy(&settings_path, settings_path.with_extension("json.bak"))?;
    }

    let merged = merge_hooks(existing, &hook_path)?;
    fs::write(settings_path, serde_json::to_string_pretty(&merged)?)?;

    Ok(())
}

pub fn preview_hook_config() -> Result<String, String> {
    let existing = if get_claude_settings_path().exists() {
        let content = fs::read_to_string(get_claude_settings_path()).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())?
    } else {
        json!({})
    };

    let merged = merge_hooks(existing, &get_hook_binary_path())?;
    serde_json::to_string_pretty(&merged).map_err(|e| e.to_string())
}

pub fn check_hooks_installed() -> bool {
    let Ok(content) = fs::read_to_string(get_claude_settings_path()) else {
        return false;
    };

    content.contains(hook_binary_name())
}

fn hook_binary_name() -> &'static str {
    if cfg!(windows) {
        "ai-light-hook.exe"
    } else {
        "ai-light-hook"
    }
}

fn bundled_hook_candidates(resource_dir: &Path) -> Vec<PathBuf> {
    vec![
        resource_dir.join(hook_binary_name()),
        resource_dir.join("ai-light-hook.exe"),
        resource_dir.join("ai-light-hook"),
    ]
}

fn remove_existing_ai_light_hooks(event_hooks: &mut Vec<Value>) {
    event_hooks.retain(|entry| {
        let Some(commands) = entry.get("hooks").and_then(Value::as_array) else {
            return true;
        };

        !commands.iter().any(|command| {
            command
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|command| command.contains("ai-light-hook"))
        })
    });
}

pub fn hook_binary_is_current(source: &Path, destination: &Path) -> Result<bool, std::io::Error> {
    if !destination.exists() {
        return Ok(false);
    }

    Ok(fs::read(source)? == fs::read(destination)?)
}

fn shell_command_path(path: &Path) -> String {
    let path = path.to_string_lossy();

    if path.contains(' ') {
        format!("\"{}\"", path.replace('"', "\\\""))
    } else {
        path.to_string()
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}
