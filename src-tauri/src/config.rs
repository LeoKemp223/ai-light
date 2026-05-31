use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppConfig {
    pub window_x: i32,
    pub window_y: i32,
    pub monitoring_paused: bool,
    pub hooks_installed: bool,
    pub http_bind: String,
    pub http_port: Option<u16>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window_x: 100,
            window_y: 100,
            monitoring_paused: false,
            hooks_installed: false,
            http_bind: "127.0.0.1".to_string(),
            http_port: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub http_port: u16,
}

pub fn get_config_dir() -> PathBuf {
    if let Some(config_dir) = std::env::var_os("AI_LIGHT_CONFIG_DIR") {
        return PathBuf::from(config_dir);
    }

    home_dir()
        .expect("failed to resolve home directory")
        .join(".ai_light")
}

pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

pub fn get_runtime_path() -> PathBuf {
    get_config_dir().join("runtime.json")
}

pub fn get_lock_path() -> PathBuf {
    get_config_dir().join("ai-light.lock")
}

pub fn get_log_path() -> PathBuf {
    get_config_dir().join("ai-light.log")
}

pub fn load_app_config() -> AppConfig {
    let path = get_config_path();
    let Ok(content) = fs::read_to_string(path) else {
        return AppConfig::default();
    };

    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save_app_config(config: &AppConfig) -> io::Result<()> {
    fs::create_dir_all(get_config_dir())?;
    let content = serde_json::to_string_pretty(config).map_err(io::Error::other)?;
    fs::write(get_config_path(), content)
}

pub fn load_runtime_config() -> Option<RuntimeConfig> {
    let content = fs::read_to_string(get_runtime_path()).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_runtime_config(config: &RuntimeConfig) -> io::Result<()> {
    fs::create_dir_all(get_config_dir())?;
    let content = serde_json::to_string_pretty(config).map_err(io::Error::other)?;
    fs::write(get_runtime_path(), content)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}
