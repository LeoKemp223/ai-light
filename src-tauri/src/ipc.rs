use ai_light::aggregator::StateAggregator;
use ai_light::config::{
    get_config_dir, get_config_path, get_lock_path, get_log_path, get_runtime_path,
    load_app_config, load_runtime_config, save_app_config,
};
use ai_light::hook_installer::{
    check_hooks_installed, check_opencode_integration, check_reasonix_integration,
    install_hooks, install_opencode_integration, install_reasonix_integration, preview_hook_config,
    remove_hooks, remove_opencode_integration, remove_reasonix_integration,
};
use ai_light::types::LightState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, Position, Size, State};

#[derive(Debug, Serialize)]
pub struct Diagnostics {
    pub config_dir: String,
    pub runtime_path: String,
    pub lock_path: String,
    pub log_path: String,
    pub claude_settings_path: String,
    pub hook_binary_path: String,
    pub codex_sessions_path: String,
    pub hooks_installed: bool,
    pub hook_binary_exists: bool,
    pub runtime_exists: bool,
    pub light_count: usize,
    pub recent_log: String,
    pub opencode_integration: bool,
    pub reasonix_integration: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfigView {
    pub config_path: String,
    pub http_bind: String,
    pub http_port: Option<u16>,
    pub runtime_port: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfigUpdate {
    pub http_bind: String,
    pub http_port: Option<u16>,
}

#[tauri::command]
pub fn confirm_light(project_id: String, aggregator: State<Arc<StateAggregator>>) {
    aggregator.confirm_light(&project_id);
}

#[tauri::command]
pub fn remove_light(project_id: String, aggregator: State<Arc<StateAggregator>>) {
    aggregator.remove_light(&project_id);
}

#[tauri::command]
pub fn get_lights(aggregator: State<Arc<StateAggregator>>) -> Vec<LightState> {
    aggregator.get_lights()
}

#[tauri::command]
pub fn open_project(project_id: String) -> Result<(), String> {
    open_path(&project_id)
}

#[tauri::command]
pub fn open_session_logs(project_id: String) -> Result<(), String> {
    let path = claude_project_log_dir(&project_id)?;
    open_path(&path.to_string_lossy())
}

#[tauri::command]
pub fn open_app_log() -> Result<(), String> {
    let log_path = get_log_path();
    if !log_path.exists() {
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(&log_path, "").map_err(|error| error.to_string())?;
    }

    open_path(&log_path.to_string_lossy())
}

#[tauri::command]
pub fn get_app_config() -> AppConfigView {
    let config = load_app_config();
    AppConfigView {
        config_path: get_config_path().to_string_lossy().to_string(),
        http_bind: config.http_bind,
        http_port: config.http_port,
        runtime_port: load_runtime_config().map(|runtime| runtime.http_port),
    }
}

#[tauri::command]
pub fn save_app_config_command(update: AppConfigUpdate) -> Result<(), String> {
    validate_http_bind(&update.http_bind)?;
    validate_http_port(update.http_port)?;

    let mut config = load_app_config();
    config.http_bind = update.http_bind;
    config.http_port = update.http_port;

    save_app_config(&config).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_diagnostics(aggregator: State<Arc<StateAggregator>>) -> Diagnostics {
    let log_path = get_log_path();
    let hook_binary_path = ai_light::hook_installer::get_hook_binary_path();
    Diagnostics {
        config_dir: get_config_dir().to_string_lossy().to_string(),
        runtime_path: get_runtime_path().to_string_lossy().to_string(),
        lock_path: get_lock_path().to_string_lossy().to_string(),
        log_path: log_path.to_string_lossy().to_string(),
        claude_settings_path: ai_light::hook_installer::get_claude_settings_path()
            .to_string_lossy()
            .to_string(),
        hook_binary_path: hook_binary_path.to_string_lossy().to_string(),
        codex_sessions_path: codex_sessions_dir().to_string_lossy().to_string(),
        hooks_installed: check_hooks_installed(),
        hook_binary_exists: hook_binary_path.exists(),
        runtime_exists: get_runtime_path().exists(),
        light_count: aggregator.get_lights().len(),
        recent_log: recent_log(&log_path),
        opencode_integration: check_opencode_integration().unwrap_or(false),
        reasonix_integration: check_reasonix_integration().unwrap_or(false),
    }
}

#[tauri::command]
pub fn copy_path(project_id: String) -> String {
    project_id
}

#[tauri::command]
pub fn pause_monitoring() {}

#[tauri::command]
pub fn resume_monitoring() {}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| "settings window is not available".to_string())?;

    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn resize_main_window(app: AppHandle, width: f64, height: f64) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is not available".to_string())?;

    let width = width.clamp(54.0, 1200.0);
    let height = height.clamp(64.0, 900.0);

    window
        .set_size(Size::Logical(LogicalSize::new(width, height)))
        .map_err(|error| error.to_string())?;

    keep_window_on_current_monitor(&window)?;
    Ok(())
}

fn keep_window_on_current_monitor(window: &tauri::WebviewWindow) -> Result<(), String> {
    let Some(monitor) = window
        .current_monitor()
        .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };

    let position = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();

    let monitor_left = monitor_position.x;
    let monitor_top = monitor_position.y;
    let monitor_right = monitor_left + monitor_size.width as i32;
    let monitor_bottom = monitor_top + monitor_size.height as i32;

    let mut next_x = position.x;
    let mut next_y = position.y;

    if next_x + size.width as i32 > monitor_right {
        next_x = monitor_right - size.width as i32;
    }
    if next_y + size.height as i32 > monitor_bottom {
        next_y = monitor_bottom - size.height as i32;
    }

    next_x = next_x.max(monitor_left);
    next_y = next_y.max(monitor_top);

    if next_x != position.x || next_y != position.y {
        window
            .set_position(Position::Physical(PhysicalPosition::new(next_x, next_y)))
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn validate_http_bind(bind: &str) -> Result<(), String> {
    bind.parse::<IpAddr>().map(|_| ()).map_err(|_| {
        "HTTP bind must be an IP address, for example 127.0.0.1 or 0.0.0.0".to_string()
    })
}

fn validate_http_port(port: Option<u16>) -> Result<(), String> {
    if matches!(port, Some(0)) {
        return Err("HTTP port must be blank or between 1 and 65535".to_string());
    }

    Ok(())
}

#[tauri::command]
pub fn check_hooks() -> bool {
    check_hooks_installed()
}

#[tauri::command]
pub fn install_hooks_command() -> Result<(), String> {
    install_hooks().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn remove_hooks_command() -> Result<(), String> {
    remove_hooks().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn preview_hook_config_command() -> Result<String, String> {
    preview_hook_config()
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

// --- opencode integration commands ---

#[tauri::command]
pub fn check_opencode() -> bool {
    check_opencode_integration().unwrap_or(false)
}

#[tauri::command]
pub fn install_opencode_command() -> Result<(), String> {
    install_opencode_integration().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn remove_opencode_command() -> Result<(), String> {
    remove_opencode_integration().map_err(|error| error.to_string())
}

// --- reasonix integration commands ---

#[tauri::command]
pub fn check_reasonix() -> bool {
    check_reasonix_integration().unwrap_or(false)
}

#[tauri::command]
pub fn install_reasonix_command() -> Result<(), String> {
    install_reasonix_integration().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn remove_reasonix_command() -> Result<(), String> {
    remove_reasonix_integration().map_err(|error| error.to_string())
}

fn open_path(path: &str) -> Result<(), String> {
    let mut command = platform_open_command(path)?;

    command.spawn().map_err(|error| error.to_string())?;
    Ok(())
}

fn claude_project_log_dir(project_id: &str) -> Result<PathBuf, String> {
    let home = home_dir().ok_or_else(|| "failed to resolve home directory".to_string())?;
    Ok(home
        .join(".claude")
        .join("projects")
        .join(encode_claude_project_dir(project_id)))
}

fn encode_claude_project_dir(project_id: &str) -> String {
    project_id
        .replace("\\\\?\\", "")
        .replace(':', "")
        .replace(['\\', '/'], "-")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

fn codex_sessions_dir() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("sessions")
}

fn recent_log(log_path: &PathBuf) -> String {
    let Ok(content) = fs::read_to_string(log_path) else {
        return String::new();
    };

    let lines: Vec<_> = content.lines().rev().take(20).collect();
    lines.into_iter().rev().collect::<Vec<_>>().join("\n")
}

fn platform_open_command(path: &str) -> Result<std::process::Command, String> {
    #[cfg(target_os = "windows")]
    {
        let mut command = std::process::Command::new("explorer");
        command.arg(path);
        return Ok(command);
    }

    #[cfg(target_os = "macos")]
    {
        let mut command = std::process::Command::new("open");
        command.arg(path);
        return Ok(command);
    }

    #[cfg(target_os = "linux")]
    {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(path);
        return Ok(command);
    }

    #[allow(unreachable_code)]
    Err("opening paths is not supported on this platform".to_string())
}
