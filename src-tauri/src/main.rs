#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ai_light::aggregator::StateAggregator;
use ai_light::app_lock::AppLock;
use ai_light::config::load_app_config;
use ai_light::http_server::{existing_instance_is_healthy, start_http_server};
use std::sync::Arc;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

mod ipc;

fn main() {
    let app_lock = match AppLock::acquire() {
        Ok(Some(lock)) => lock,
        Ok(None) => return,
        Err(error) => {
            eprintln!("failed to acquire app lock: {error}");
            return;
        }
    };

    let app_config = load_app_config();
    let aggregator = Arc::new(StateAggregator::new());
    let server_aggregator = Arc::clone(&aggregator);

    tauri::Builder::default()
        .manage(Arc::clone(&aggregator))
        .manage(app_lock)
        .invoke_handler(tauri::generate_handler![
            ipc::confirm_light,
            ipc::remove_light,
            ipc::get_lights,
            ipc::get_diagnostics,
            ipc::open_project,
            ipc::open_session_logs,
            ipc::open_app_log,
            ipc::get_app_config,
            ipc::save_app_config_command,
            ipc::copy_path,
            ipc::pause_monitoring,
            ipc::resume_monitoring,
            ipc::open_settings,
            ipc::resize_main_window,
            ipc::check_hooks,
            ipc::install_hooks_command,
            ipc::remove_hooks_command,
            ipc::preview_hook_config_command,
            ipc::quit_app
        ])
        .setup(move |app| {
            if existing_instance_is_healthy() {
                app.handle().exit(0);
                return Ok(());
            }

            let window = app
                .get_webview_window("main")
                .expect("main window should exist");
            if let Some(settings_window) = app.get_webview_window("settings") {
                let window_to_hide = settings_window.clone();
                settings_window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_to_hide.hide();
                    }
                });
            }

            let emit_aggregator = Arc::clone(&aggregator);
            let emit_window = window.clone();

            aggregator.set_on_change(move || {
                let _ = emit_window.emit("state-changed", emit_aggregator.get_lights());
            });

            start_http_server(Arc::clone(&server_aggregator), &app_config)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            ai_light::codex_watcher::start_codex_watcher(Arc::clone(&aggregator))?;

            window.emit("state-changed", aggregator.get_lights())?;

            if let Ok(resource_dir) = app.path().resource_dir() {
                let _ = ai_light::hook_installer::install_hook_binary_from_resource(&resource_dir);
            }

            if !ai_light::hook_installer::check_hooks_installed() {
                WebviewWindowBuilder::new(
                    app,
                    "install-hooks",
                    WebviewUrl::App("install-hooks.html".into()),
                )
                .title("Claude Code Integration")
                .inner_size(560.0, 340.0)
                .resizable(false)
                .center()
                .build()?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
