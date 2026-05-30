use ai_light::config::{get_config_dir, AppConfig, RuntimeConfig};

#[test]
fn config_dir_points_to_ai_light_home_directory() {
    let dir = get_config_dir();

    assert_eq!(dir.file_name().unwrap().to_string_lossy(), ".ai_light");
    assert!(dir.parent().is_some());
}

#[test]
fn app_config_defaults_match_mvp_startup_state() {
    let config = AppConfig::default();

    assert_eq!(config.window_x, 100);
    assert_eq!(config.window_y, 100);
    assert!(!config.monitoring_paused);
    assert!(!config.hooks_installed);
}

#[test]
fn runtime_config_serializes_http_port() {
    let runtime = RuntimeConfig { http_port: 12345 };

    let json = serde_json::to_string(&runtime).unwrap();
    let parsed: RuntimeConfig = serde_json::from_str(&json).unwrap();

    assert!(json.contains("12345"));
    assert_eq!(parsed, runtime);
}
