use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter};

use crate::logfile::{write_entry, Level};

static TRAY_INSTANCE: RwLock<Option<TrayIcon>> = RwLock::new(None);
static HAS_SHOWN_TRAY_BALLOON: AtomicBool = AtomicBool::new(false);

/// Truncates title to 48 characters with an ellipsis if longer, per specification.
pub fn truncate_menu_title(title: &str) -> String {
    let char_count = title.chars().count();
    if char_count <= 48 {
        title.to_string()
    } else {
        let truncated: String = title.chars().take(45).collect();
        format!("{}...", truncated)
    }
}

/// Builds the tray context menu dynamically from the current active pinned window list.
pub fn build_tray_menu(
    app: &AppHandle,
    pins: &[(isize, String)], // (hwnd, title)
) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let menu = Menu::new(app)?;

    // 1. "Show pinontop"
    let show_item = MenuItem::with_id(app, "show", "Show pinontop", true, None::<&str>)?;
    menu.append(&show_item)?;

    // 2. Active Pins (checked, clicking unpins)
    if !pins.is_empty() {
        let sep1 = PredefinedMenuItem::separator(app)?;
        menu.append(&sep1)?;

        for (hwnd, title) in pins {
            let label = truncate_menu_title(title);
            let item_id = format!("unpin:{}", hwnd);
            let check_item = CheckMenuItem::with_id(
                app,
                &item_id,
                &label,
                true, // enabled
                true, // checked
                None::<&str>,
            )?;
            menu.append(&check_item)?;
        }
    }

    let sep2 = PredefinedMenuItem::separator(app)?;
    menu.append(&sep2)?;

    // 3. Actions: "Pin a window...", "Unpin all"
    let pin_item = MenuItem::with_id(app, "pin_picker", "Pin a window...", true, None::<&str>)?;
    menu.append(&pin_item)?;

    let has_pins = !pins.is_empty();
    let unpin_all = MenuItem::with_id(app, "unpin_all", "Unpin all", has_pins, None::<&str>)?;
    menu.append(&unpin_all)?;

    let sep3 = PredefinedMenuItem::separator(app)?;
    menu.append(&sep3)?;

    // 4. "Settings" and "Quit"
    let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    menu.append(&settings_item)?;

    let quit_item = MenuItem::with_id(app, "quit", "Quit pinontop", true, None::<&str>)?;
    menu.append(&quit_item)?;

    Ok(menu)
}

/// Updates the tray context menu dynamically when pinned windows change.
pub fn update_tray_menu(app: &AppHandle, pins: &[(isize, String)]) {
    if let Ok(menu) = build_tray_menu(app, pins)
        && let Ok(guard) = TRAY_INSTANCE.read()
            && let Some(ref tray) = *guard {
                let _ = tray.set_menu(Some(menu));
            }
}

/// Sets up the initial system tray icon and attaches event handlers.
pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_tray_menu(app, &[])?;
    let icon = app.default_window_icon().cloned().ok_or("No default window icon found")?;

    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("pinontop")
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = tray;
                crate::host::toggle_main_window();
            }
        })
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if let Some(hwnd_str) = id.strip_prefix("unpin:") {
                if let Ok(hwnd) = hwnd_str.parse::<isize>() {
                    let _ = app.emit("trigger-unpin-hwnd", hwnd);
                }
            } else {
                match id {
                    "show" => {
                        crate::host::show_main_window();
                    }
                    "pin_picker" => {
                        crate::host::show_main_window();
                        let _ = app.emit("open-sheet", "window_picker");
                    }
                    "unpin_all" => {
                        let _ = app.emit("trigger-unpin-all", ());
                    }
                    "settings" => {
                        crate::host::show_main_window();
                        let _ = app.emit("open-sheet", "settings");
                    }
                    "quit" => {
                        crate::host::perform_manual_quit();
                    }
                    _ => {}
                }
            }
        })
        .build(app)?;

    if let Ok(mut guard) = TRAY_INSTANCE.write() {
        *guard = Some(tray);
    }

    // Attach TaskbarCreated recovery handler: re-sets tray if Windows Explorer restarts
    let app_handle = app.clone();
    crate::host::set_taskbar_created_handler(move || {
        write_entry(Level::Info, "Restoring system tray icon after TaskbarCreated event");
        let _ = setup_tray(&app_handle);
    });

    Ok(())
}

/// Checks whether the user has seen the first-time tray notice; if not, marks it shown.
pub fn check_first_time_tray_notice() -> bool {
    let already_shown = HAS_SHOWN_TRAY_BALLOON.swap(true, Ordering::SeqCst);
    !already_shown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_menu_title() {
        let short = "Notepad - notes.txt";
        assert_eq!(truncate_menu_title(short), short);

        let long = "Very Long Document Title That Exceeds Forty Eight Characters In Length";
        let truncated = truncate_menu_title(long);
        assert_eq!(truncated.chars().count(), 48);
        assert!(truncated.ends_with("..."));
    }
}
