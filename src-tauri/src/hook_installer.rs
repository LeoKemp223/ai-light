use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const HOOK_EVENTS: [(&str, &str); 8] = [
    ("SessionStart", "session-start"),
    ("UserPromptSubmit", "prompt-submit"),
    ("PreToolUse", "pre-tool-use"),
    ("PermissionRequest", "permission-request"),
    ("PostToolUse", "post-tool-use"),
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

    let command_path = hook_path.to_string_lossy().to_string();

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
                "command": command_path.clone(),
                "args": [hook_event]
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
        read_settings_json(&settings_path)?
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

pub fn remove_hooks() -> Result<(), Box<dyn std::error::Error>> {
    let settings_path = get_claude_settings_path();
    if settings_path.exists() {
        let existing = read_settings_json(&settings_path)?;
        let cleaned = remove_ai_light_hooks(existing)?;

        fs::copy(
            &settings_path,
            settings_path.with_extension("json.ai-light-remove.bak"),
        )?;
        fs::write(&settings_path, serde_json::to_string_pretty(&cleaned)?)?;
    }

    let hook_path = get_hook_binary_path();
    if hook_path.exists() {
        fs::remove_file(hook_path)?;
    }

    Ok(())
}

pub fn preview_hook_config() -> Result<String, String> {
    let existing = if get_claude_settings_path().exists() {
        read_settings_json(&get_claude_settings_path()).map_err(|e| e.to_string())?
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

    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
    let Ok(settings) = serde_json::from_str::<Value>(content) else {
        return false;
    };

    let Some(hooks) = settings.get("hooks").and_then(Value::as_object) else {
        return false;
    };

    HOOK_EVENTS.iter().all(|(claude_event, hook_event)| {
        hooks
            .get(*claude_event)
            .and_then(Value::as_array)
            .is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| contains_ai_light_hook_for_event(entry, hook_event))
            })
    })
}

pub fn remove_ai_light_hooks(mut existing: Value) -> Result<Value, String> {
    if !existing.is_object() {
        return Err("settings root must be a JSON object".to_string());
    }

    let Some(hooks) = existing.get_mut("hooks") else {
        return Ok(existing);
    };

    let hooks = hooks
        .as_object_mut()
        .ok_or_else(|| "settings hooks field must be a JSON object".to_string())?;
    let event_names: Vec<_> = hooks.keys().cloned().collect();

    for event_name in event_names {
        let Some(event_hooks) = hooks.get_mut(&event_name).and_then(Value::as_array_mut) else {
            continue;
        };

        remove_existing_ai_light_hooks(event_hooks);

        if event_hooks.is_empty() {
            hooks.remove(&event_name);
        }
    }

    Ok(existing)
}

// --- opencode integration ---

pub fn get_opencode_config_path() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("opencode")
        .join("opencode.json")
}

pub fn get_opencode_plugin_source_path() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_light")
        .join("opencode-plugin.js")
}

pub fn install_opencode_integration() -> Result<(), Box<dyn std::error::Error>> {
    let plugin_source = bundled_opencode_plugin_path()?;
    let plugin_dest = get_opencode_plugin_source_path();

    if let Some(parent) = plugin_dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&plugin_source, &plugin_dest)?;

    let config_path = get_opencode_config_path();
    let existing = if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    let plugin_path = plugin_dest.to_string_lossy().to_string();
    let mut config = existing;
    if !config.is_object() {
        config = json!({});
    }
    if config.get("plugins").is_none() {
        config["plugins"] = json!({});
    }
    config["plugins"][&plugin_path] = json!({});

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if config_path.exists() {
        fs::copy(&config_path, config_path.with_extension("json.ai-light.bak"))?;
    }
    fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;

    Ok(())
}

pub fn remove_opencode_integration() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_opencode_config_path();
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        let mut config: Value = serde_json::from_str(&content).unwrap_or(json!({}));

        if let Some(plugins) = config.get_mut("plugins").and_then(Value::as_object_mut) {
            let plugin_path = get_opencode_plugin_source_path();
            let plugin_key = plugin_path.to_string_lossy().to_string();
            plugins.remove(&plugin_key);
        }

        fs::copy(&config_path, config_path.with_extension("json.ai-light.bak"))?;
        fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    }

    let plugin_path = get_opencode_plugin_source_path();
    if plugin_path.exists() {
        fs::remove_file(plugin_path)?;
    }

    Ok(())
}

pub fn check_opencode_integration() -> Result<bool, Box<dyn std::error::Error>> {
    let plugin_path = get_opencode_plugin_source_path();
    if !plugin_path.exists() {
        return Ok(false);
    }

    let config_path = get_opencode_config_path();
    if !config_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&config_path)?;
    let config: Value = serde_json::from_str(&content)?;
    let plugin_key = plugin_path.to_string_lossy().to_string();

    Ok(config
        .get("plugins")
        .and_then(Value::as_object)
        .is_some_and(|plugins| plugins.contains_key(&plugin_key)))
}

// --- reasonix integration ---

pub fn get_reasonix_settings_path() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".reasonix")
        .join("settings.json")
}

pub fn install_reasonix_integration() -> Result<(), Box<dyn std::error::Error>> {
    let hook_path = get_hook_binary_path();
    if !hook_path.exists() {
        return Err(format!("hook binary not found: {}", hook_path.display()).into());
    }

    let settings_path = get_reasonix_settings_path();
    let existing = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    let hook_cmd = hook_path.to_string_lossy().to_string();
    let mut config = existing;
    if !config.is_object() {
        config = json!({});
    }

    let reasonix_hooks = json!({
        "SessionStart": [
            {
                "match": "",
                "command": [hook_cmd.clone(), "--source", "reasonix", "session-start"].join(" ")
            }
        ],
        "PreToolUse": [
            {
                "match": "",
                "command": [hook_cmd.clone(), "--source", "reasonix", "pre-tool-use"].join(" ")
            }
        ],
        "PostToolUse": [
            {
                "match": "",
                "command": [hook_cmd.clone(), "--source", "reasonix", "post-tool-use"].join(" ")
            }
        ],
        "UserPromptSubmit": [
            {
                "match": "",
                "command": [hook_cmd.clone(), "--source", "reasonix", "prompt-submit"].join(" ")
            }
        ],
        "Stop": [
            {
                "match": "",
                "command": [hook_cmd.clone(), "--source", "reasonix", "stop"].join(" ")
            }
        ],
        "TurnStart": [
            {
                "match": "",
                "command": [hook_cmd.clone(), "--source", "reasonix", "pre-tool-use"].join(" ")
            }
        ],
        "TurnEnd": [
            {
                "match": "",
                "command": [hook_cmd.clone(), "--source", "reasonix", "stop"].join(" ")
            }
        ],
        "SessionEnd": [
            {
                "match": "",
                "command": [hook_cmd, "--source", "reasonix", "session-end"].join(" ")
            }
        ]
    });

    config["hooks"] = reasonix_hooks;

    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if settings_path.exists() {
        fs::copy(&settings_path, settings_path.with_extension("json.ai-light.bak"))?;
    }
    fs::write(&settings_path, serde_json::to_string_pretty(&config)?)?;

    Ok(())
}

pub fn remove_reasonix_integration() -> Result<(), Box<dyn std::error::Error>> {
    let settings_path = get_reasonix_settings_path();
    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        let mut config: Value = serde_json::from_str(&content).unwrap_or(json!({}));

        if config.is_object() {
            config["hooks"] = json!({});
        }

        fs::copy(&settings_path, settings_path.with_extension("json.ai-light.bak"))?;
        fs::write(&settings_path, serde_json::to_string_pretty(&config)?)?;
    }

    Ok(())
}

pub fn check_reasonix_integration() -> Result<bool, Box<dyn std::error::Error>> {
    let settings_path = get_reasonix_settings_path();
    if !settings_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&settings_path)?;
    let config: Value = serde_json::from_str(&content)?;

    Ok(config
        .get("hooks")
        .and_then(Value::as_object)
        .is_some_and(|hooks| hooks.contains_key("SessionStart")))
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

fn bundled_opencode_plugin_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dest = get_opencode_plugin_source_path();
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&dest, OPENCODE_PLUGIN_SOURCE)?;
    Ok(dest)
}

const OPENCODE_PLUGIN_SOURCE: &str = r#"/**
 * AI Light OpenCode Plugin
 *
 * Sends OpenCode session events to AI Light's local HTTP server.
 *
 * ## Installation
 *
 * Add to ~/.config/opencode/opencode.json:
 * {
 *   "plugins": {
 *     "https://raw.githubusercontent.com/lcy05/ai-light/main/scripts/opencode-plugin.js": {}
 *   }
 * }
 *
 * Or copy this file to ~/.config/opencode/plugins/ai-light.js
 */

/// <reference types="@opencode-ai/plugin" />

/**
 * @type {import("@opencode-ai/plugin").Plugin}
 */
export default async function aiLightPlugin(ctx) {
  const AI_LIGHT_URL =
    process.env.AI_LIGHT_URL ||
    "http://127.0.0.1:17321";

  function baseUrl() {
    return AI_LIGHT_URL.replace(/\/+$/, "");
  }

  async function sendEvent(eventType, payload) {
    try {
      const url = `${baseUrl()}/events`;
      const body = JSON.stringify({
        event_type: eventType,
        session_id: payload.session?.id || "unknown",
        sessionId: payload.session?.id || "unknown",
        cwd: payload.session?.cwd || payload.cwd || process.cwd(),
        tool_call: payload.tool?.name || payload.toolName || null,
        toolName: payload.tool?.name || payload.toolName || null,
        tool_source: "opencode",
        source: "opencode",
      });
      const response = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body,
      });
      if (!response.ok) {
        console.error(`AI Light: event ${eventType} failed (${response.status})`);
      }
    } catch (error) {
      // AI Light not running -- silently ignore
    }
  }

  return {
    "session.created": async (input) => {
      await sendEvent("session-start", input);
    },
    "session.deleted": async (input) => {
      await sendEvent("session-end", input);
    },
    "session.idle": async (input) => {
      await sendEvent("stop", input);
    },
    "tool.execute.before": async (input) => {
      await sendEvent("pre-tool-use", input);
    },
    "tool.execute.after": async (input) => {
      await sendEvent("post-tool-use", input);
    },
    "message.updated": async (input) => {
      if (input.role === "user") {
        await sendEvent("prompt-submit", input);
      }
    },
  };
}
"#;

fn remove_existing_ai_light_hooks(event_hooks: &mut Vec<Value>) {
    event_hooks.retain(|entry| !entry_contains_ai_light_hook(entry));
}

fn entry_contains_ai_light_hook(entry: &Value) -> bool {
    let Some(commands) = entry.get("hooks").and_then(Value::as_array) else {
        return false;
    };

    commands.iter().any(|command| {
        command
            .get("command")
            .and_then(Value::as_str)
            .is_some_and(|command| command.contains("ai-light-hook"))
    })
}

fn contains_ai_light_hook_for_event(entry: &Value, hook_event: &str) -> bool {
    let Some(commands) = entry.get("hooks").and_then(Value::as_array) else {
        return false;
    };

    commands.iter().any(|command| {
        let command_matches = command
            .get("command")
            .and_then(Value::as_str)
            .is_some_and(|command| command.contains(hook_binary_name()));

        if !command_matches {
            return false;
        }

        command
            .get("args")
            .and_then(Value::as_array)
            .is_some_and(|args| args.iter().any(|arg| arg.as_str() == Some(hook_event)))
            || command
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|command| command.contains(hook_event))
    })
}

pub fn hook_binary_is_current(source: &Path, destination: &Path) -> Result<bool, std::io::Error> {
    if !destination.exists() {
        return Ok(false);
    }

    Ok(fs::read(source)? == fs::read(destination)?)
}

fn read_settings_json(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
    Ok(serde_json::from_str(content)?)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}
