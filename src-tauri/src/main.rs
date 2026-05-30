#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ai_light::aggregator::StateAggregator;
use ai_light::http_server::{existing_instance_is_healthy, start_http_server};
use std::sync::Arc;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

mod ipc;

fn main() {
    let aggregator = Arc::new(StateAggregator::new());
    let server_aggregator = Arc::clone(&aggregator);

    tauri::Builder::default()
        .manage(Arc::clone(&aggregator))
        .invoke_handler(tauri::generate_handler![
            ipc::confirm_light,
            ipc::remove_light,
            ipc::get_lights,
            ipc::open_project,
            ipc::open_session_logs,
            ipc::copy_path,
            ipc::pause_monitoring,
            ipc::resume_monitoring,
            ipc::open_settings,
            ipc::check_hooks,
            ipc::install_hooks_command,
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
            let emit_aggregator = Arc::clone(&aggregator);
            let emit_window = window.clone();

            aggregator.set_on_change(move || {
                let _ = emit_window.emit("state-changed", emit_aggregator.get_lights());
            });

            start_http_server(Arc::clone(&server_aggregator))
                .map_err(|error| std::io::Error::other(error.to_string()))?;

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
