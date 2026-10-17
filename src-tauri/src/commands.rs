use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::autostart;
use crate::flash::{flash_window, hide_flash, FlashKind};
use crate::host;
use crate::pins::{get_manager, FrontendPinItem};
use crate::store::{SavedSettings, SavedShortcuts};
use crate::win;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppState {
    pub pins: Vec<FrontendPinItem>,
    pub settings: SavedSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowInfo {
    pub hwnd: isize,
    pub title: String,
    pub process_name: String,
    pub is_pinned: bool,
}

/// Retrieves the complete application state (pins + settings).
#[tauri::command]
pub fn get_state() -> Result<AppState, String> {
    let mgr = get_manager().read().map_err(|e| e.to_string())?;
    Ok(AppState {
        pins: mgr.to_frontend_items(),
        settings: mgr.settings.clone(),
    })
}

/// Toggles pin status on the specified window.
#[tauri::command]
pub fn toggle_pin(hwnd: isize) -> Result<bool, String> {
    let mut mgr = get_manager().write().map_err(|e| e.to_string())?;
    mgr.toggle_pin(hwnd).map_err(|e| e.to_string())
}

/// Explicitly unpins a window.
#[tauri::command]
pub fn unpin(hwnd: isize) -> Result<(), String> {
    let mut mgr = get_manager().write().map_err(|e| e.to_string())?;
    if mgr.live_pins.iter().any(|p| p.hwnd == hwnd) {
        let _ = mgr.toggle_pin(hwnd).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Adjusts the opacity (20–100%) of a pinned window.
#[tauri::command]
pub fn set_opacity(hwnd: isize, percent: u8) -> Result<(), String> {
    let mut mgr = get_manager().write().map_err(|e| e.to_string())?;
    mgr.set_opacity(hwnd, percent).map_err(|e| e.to_string())
}

/// Enables or disables click-through on a pinned window.
#[tauri::command]
pub fn set_click_through(hwnd: isize, enabled: bool) -> Result<(), String> {
    let mut mgr = get_manager().write().map_err(|e| e.to_string())?;
    mgr.set_click_through(hwnd, enabled).map_err(|e| e.to_string())
}

/// Unpins all pinned windows and clears the pending list.
#[tauri::command]
pub fn unpin_all() -> Result<(), String> {
    let mut mgr = get_manager().write().map_err(|e| e.to_string())?;
    mgr.unpin_all().map_err(|e| e.to_string())
}

/// Forgets an orphaned pending pin by index.
#[tauri::command]
pub fn forget_pending(index: usize) -> Result<(), String> {
    let mut mgr = get_manager().write().map_err(|e| e.to_string())?;
    if mgr.forget_pending(index) {
        Ok(())
    } else {
        Err("Pending pin index out of bounds".to_string())
    }
}

/// Enumerates all pinnable windows currently open on the system for WindowPicker.
#[tauri::command]
pub fn get_windows() -> Result<Vec<WindowInfo>, String> {
    let own_pid = win::get_current_process_id();
    let discovered = win::get_pinnable_windows(own_pid);

    let mgr = get_manager().read().map_err(|e| e.to_string())?;
    let windows = discovered
        .into_iter()
        .map(|w| {
            let is_pinned = mgr.live_pins.iter().any(|p| p.hwnd == w.hwnd);
            WindowInfo {
                hwnd: w.hwnd,
                title: w.title,
                process_name: w.process_name,
                is_pinned,
            }
        })
        .collect();

    Ok(windows)
}

/// Saves shortcut configurations and re-registers global hotkeys.
/// Returns any shortcut collision names encountered.
#[tauri::command]
pub fn save_shortcuts(app: AppHandle, shortcuts: SavedShortcuts) -> Result<Vec<String>, String> {
    let mut mgr = get_manager().write().map_err(|e| e.to_string())?;
    mgr.settings.shortcuts = shortcuts.clone();
    let _ = mgr.save_to_disk();

    let conflicts = if let Some(hwnd) = host::get_helper_hwnd() {
        host::register_all_hotkeys(hwnd, &shortcuts)
    } else {
        Vec::new()
    };

    let _ = app.emit("settings-changed", &mgr.settings);
    Ok(conflicts)
}

/// Persists application preferences.
#[tauri::command]
pub fn save_settings(app: AppHandle, settings: SavedSettings) -> Result<(), String> {
    let mut mgr = get_manager().write().map_err(|e| e.to_string())?;

    // If start_with_windows changed, update the Windows registry Run key
    if mgr.settings.start_with_windows != settings.start_with_windows {
        let _ = autostart::set_autostart(settings.start_with_windows);
    }

    mgr.settings = settings;
    let _ = mgr.save_to_disk();

    let _ = app.emit("settings-changed", &mgr.settings);
    Ok(())
}

/// Configures Windows registry Run key autostart.
#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    autostart::set_autostart(enabled).map_err(|e| e.to_string())?;

    let mut mgr = get_manager().write().map_err(|e| e.to_string())?;
    mgr.settings.start_with_windows = enabled;
    let _ = mgr.save_to_disk();

    if let Some(ref app) = mgr.app_handle {
        let _ = app.emit("settings-changed", &mgr.settings);
    }

    Ok(())
}

/// Queries Windows registry to determine if autostart is active.
#[tauri::command]
pub fn get_autostart() -> Result<bool, String> {
    Ok(autostart::is_autostart_enabled())
}

/// Shows or hides the hover flash overlay outline when hovering a pin row.
#[tauri::command]
pub fn row_hovered(hwnd: isize, hovered: bool) -> Result<(), String> {
    if hovered {
        flash_window(hwnd, FlashKind::Hover);
    } else {
        hide_flash();
    }
    Ok(())
}

/// Opens an external URL in the system's default browser.
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", &url])
            .spawn();
    }

    #[cfg(not(windows))]
    {
        let _ = url;
    }

    Ok(())
}
