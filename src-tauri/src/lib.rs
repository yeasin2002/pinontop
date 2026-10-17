pub mod autostart;
pub mod commands;
pub mod flash;
pub mod host;
pub mod logfile;
pub mod pins;
pub mod shortcuts;
pub mod store;
pub mod tray;
pub mod win;

use tauri::{Emitter, Manager};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logfile::init();
    log_info!("pinontop {} starting", env!("CARGO_PKG_VERSION"));

    #[cfg(windows)]
    match host::check_single_instance() {
        host::SingleInstanceStatus::AlreadyRunning => {
            log_info!("Another instance is already running; signaled existing instance and exiting.");
            return;
        }
        host::SingleInstanceStatus::Acquired => {
            log_info!("Single instance mutex acquired.");
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            host::set_app_handle(app.handle().clone());

            #[cfg(windows)]
            {
                if let Err(e) = host::init_helper_window() {
                    log_warn!("Failed to initialize helper window: {}", e);
                } else if let Some(hwnd) = host::get_helper_hwnd() {
                    let state = store::load();
                    let _ = host::register_all_hotkeys(hwnd, &state.settings.shortcuts);
                }
            }

            #[cfg(desktop)]
            {
                if let Err(e) = tray::setup_tray(app.handle()) {
                    log_warn!("Failed to initialize system tray: {}", e);
                }
                pins::init_pin_manager(app.handle().clone());
            }

            if let Some(window) = app.get_webview_window("main")
                && let Some(icon) = app.default_window_icon() {
                    let _ = window.set_icon(icon.clone());
                }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // When closing the main window, hide to tray instead of destroying
                if !host::is_session_ending() {
                    api.prevent_close();
                    let _ = window.hide();
                    if tray::check_first_time_tray_notice() {
                        let _ = window.emit("toast", "pinontop is running in the system tray");
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::get_state,
            commands::toggle_pin,
            commands::unpin,
            commands::set_opacity,
            commands::set_click_through,
            commands::unpin_all,
            commands::forget_pending,
            commands::get_windows,
            commands::save_shortcuts,
            commands::save_settings,
            commands::set_autostart,
            commands::get_autostart,
            commands::row_hovered,
            commands::open_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    #[cfg(windows)]
    host::cleanup_helper_window();
}
