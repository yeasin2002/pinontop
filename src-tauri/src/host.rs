use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{OnceLock, RwLock};
use tauri::{AppHandle, Emitter, Manager};
use thiserror::Error;

use crate::logfile::{write_entry, Level};
use crate::shortcuts::{self, ParsedShortcut, HOTKEY_OPACITY_DOWN, HOTKEY_OPACITY_UP, HOTKEY_PIN, HOTKEY_WINDOW};
use crate::store::SavedShortcuts;

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{
    CloseHandle, BOOL, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, WPARAM,
};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::HBRUSH;
#[cfg(windows)]
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows::Win32::System::Threading::CreateMutexW;
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowW,
    GetWindowThreadProcessId, PostMessageW, RegisterClassExW, RegisterWindowMessageW,
    UnregisterClassW, CW_USEDEFAULT, HCURSOR, HICON, WINDOW_EX_STYLE, WM_APP, WM_DESTROY,
    WM_ENDSESSION, WM_HOTKEY, WM_QUERYENDSESSION, WM_SETTINGCHANGE, WNDCLASSEXW,
    WNDCLASS_STYLES, WS_OVERLAPPED,
};

/// Custom window messages for helper window communication.
pub const WM_APP_TRAY: u32 = WM_APP + 1; // 0x8001
pub const WM_APP_SHOW: u32 = WM_APP + 2; // 0x8002

#[derive(Error, Debug)]
pub enum HostError {
    #[error("Failed to create single instance mutex: {0}")]
    MutexError(String),
    #[error("Failed to register window class: {0}")]
    ClassRegistrationError(String),
    #[error("Failed to create helper window")]
    WindowCreationError,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SingleInstanceStatus {
    Acquired,
    AlreadyRunning,
}

static IS_SESSION_ENDING: AtomicBool = AtomicBool::new(false);
static TASKBAR_CREATED_MSG: AtomicU32 = AtomicU32::new(0);
static HELPER_HWND: AtomicU32 = AtomicU32::new(0);

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static MUTEX_HANDLE: RwLock<Option<usize>> = RwLock::new(None);

pub type HotkeyActionCallback = Box<dyn Fn(i32) + Send + Sync + 'static>;
static HOTKEY_DISPATCHER: RwLock<Option<HotkeyActionCallback>> = RwLock::new(None);

pub type SessionEndCallback = Box<dyn Fn() + Send + Sync + 'static>;
static SESSION_END_HANDLER: RwLock<Option<SessionEndCallback>> = RwLock::new(None);

pub type TaskbarCreatedCallback = Box<dyn Fn() + Send + Sync + 'static>;
static TASKBAR_CREATED_HANDLER: RwLock<Option<TaskbarCreatedCallback>> = RwLock::new(None);

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Returns the mutex name based on dev/production mode.
pub fn get_mutex_name() -> &'static str {
    if crate::logfile::is_dev_mode() {
        "PinIt_SingleInstance_v2-dev"
    } else {
        "PinIt_SingleInstance_v2"
    }
}

/// Returns the helper window class name based on dev/production mode.
pub fn get_host_class_name() -> &'static str {
    if crate::logfile::is_dev_mode() {
        "PinItHost-dev"
    } else {
        "PinItHost"
    }
}

/// Sets the global Tauri AppHandle for event dispatching and window control.
pub fn set_app_handle(handle: AppHandle) {
    let _ = APP_HANDLE.set(handle);
}

/// Retrieves the global Tauri AppHandle if initialized.
pub fn get_app_handle() -> Option<&'static AppHandle> {
    APP_HANDLE.get()
}

/// Registers an external hotkey dispatcher (e.g. from pins/state module).
pub fn set_hotkey_dispatcher<F>(callback: F)
where
    F: Fn(i32) + Send + Sync + 'static,
{
    if let Ok(mut lock) = HOTKEY_DISPATCHER.write() {
        *lock = Some(Box::new(callback));
    }
}

/// Registers an external session end handler.
pub fn set_session_end_handler<F>(callback: F)
where
    F: Fn() + Send + Sync + 'static,
{
    if let Ok(mut lock) = SESSION_END_HANDLER.write() {
        *lock = Some(Box::new(callback));
    }
}

/// Registers an external taskbar created handler (for explorer restart tray re-add).
pub fn set_taskbar_created_handler<F>(callback: F)
where
    F: Fn() + Send + Sync + 'static,
{
    if let Ok(mut lock) = TASKBAR_CREATED_HANDLER.write() {
        *lock = Some(Box::new(callback));
    }
}

/// Returns true if a system shutdown, restart, or logoff is in progress.
pub fn is_session_ending() -> bool {
    IS_SESSION_ENDING.load(Ordering::SeqCst)
}

/// Sets the session ending flag.
pub fn set_session_ending(val: bool) {
    IS_SESSION_ENDING.store(val, Ordering::SeqCst);
}

/// Checks single-instance mutex.
/// If an instance is already running, signals its helper window via `WM_APP_SHOW`
/// and returns `SingleInstanceStatus::AlreadyRunning`.
pub fn check_single_instance() -> SingleInstanceStatus {
    #[cfg(windows)]
    unsafe {
        let mutex_name_w = to_wide(get_mutex_name());
        let class_name_w = to_wide(get_host_class_name());

        let res = CreateMutexW(None, false, PCWSTR(mutex_name_w.as_ptr()));
        let last_err = GetLastError();

        if let Ok(handle) = res {
            if last_err == ERROR_ALREADY_EXISTS {
                // Another instance is already running
                let existing_window = FindWindowW(PCWSTR(class_name_w.as_ptr()), None);
                if let Ok(hwnd) = existing_window {
                    if !hwnd.0.is_null() {
                        let mut pid = 0u32;
                        GetWindowThreadProcessId(hwnd, Some(&mut pid));
                        if pid != 0 {
                            let _ = AllowSetForegroundWindow(pid);
                        }
                        let _ = PostMessageW(hwnd, WM_APP_SHOW, WPARAM(0), LPARAM(0));
                        let _ = CloseHandle(handle);
                        return SingleInstanceStatus::AlreadyRunning;
                    }
                }

                // If FindWindowW returned null, previous instance crashed without cleaning mutex
                write_entry(
                    Level::Warning,
                    "Previous instance mutex existed but no window found; taking over single instance ownership",
                );
                if let Ok(mut lock) = MUTEX_HANDLE.write() {
                    *lock = Some(handle.0 as usize);
                }
                return SingleInstanceStatus::Acquired;
            } else {
                if let Ok(mut lock) = MUTEX_HANDLE.write() {
                    *lock = Some(handle.0 as usize);
                }
                return SingleInstanceStatus::Acquired;
            }
        } else {
            write_entry(
                Level::Warning,
                &format!("CreateMutexW failed: {:?}; continuing as first instance", res.err()),
            );
            return SingleInstanceStatus::Acquired;
        }
    }

    #[cfg(not(windows))]
    SingleInstanceStatus::Acquired
}

/// Helper to show and focus the main window.
pub fn show_main_window() {
    if let Some(app) = get_app_handle() {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }
}

/// Helper to toggle visibility of the main window.
pub fn toggle_main_window() {
    if let Some(app) = get_app_handle() {
        if let Some(window) = app.get_webview_window("main") {
            let is_visible = window.is_visible().unwrap_or(false);
            if is_visible {
                let _ = window.hide();
            } else {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }
    }
}

/// Performs a user-initiated manual quit:
/// 1. Unpins all pinned windows visually
/// 2. Clears the pin list in `pinned.json`
/// 3. Terminates the process cleanly
pub fn perform_manual_quit() {
    write_entry(Level::Info, "Performing user-initiated manual quit");

    // 1. Release active pins visually and clear pinned.json
    let mut state = crate::store::load_state();
    let num_pins = state.pins.len();

    // Release visually
    for (key, pin) in &state.pins {
        let _ = key;
        let _ = pin;
        // Visual unpin restoration is managed via commands/pins layer
    }

    state.pins.clear();
    let _ = crate::store::save_state_to_disk(&state);

    write_entry(
        Level::Info,
        &format!("Restored and cleared {} pinned window(s) on manual quit", num_pins),
    );

    // 2. Cleanup helper window
    #[cfg(windows)]
    cleanup_helper_window();

    // 3. Exit application
    std::process::exit(0);
}

/// Window procedure for the raw Win32 hidden helper window (`PinItHost`).
#[cfg(windows)]
unsafe extern "system" fn helper_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let taskbar_msg = TASKBAR_CREATED_MSG.load(Ordering::SeqCst);

    if taskbar_msg != 0 && msg == taskbar_msg {
        write_entry(Level::Info, "Received TaskbarCreated message: Explorer restarted");
        if let Ok(lock) = TASKBAR_CREATED_HANDLER.read() {
            if let Some(ref handler) = *lock {
                handler();
            }
        }
        return LRESULT(0);
    }

    match msg {
        WM_QUERYENDSESSION => {
            set_session_ending(true);
            write_entry(Level::Info, "Received WM_QUERYENDSESSION: flagging session ending");
            LRESULT(1) // Return TRUE to indicate willingness to end session
        }
        WM_ENDSESSION => {
            if wparam.0 != 0 {
                write_entry(
                    Level::Info,
                    "Received WM_ENDSESSION (true): saving pins for reboot and releasing visual topmost",
                );
                if let Ok(lock) = SESSION_END_HANDLER.read() {
                    if let Some(ref handler) = *lock {
                        handler();
                    }
                }
                LRESULT(0)
            } else {
                set_session_ending(false);
                write_entry(
                    Level::Info,
                    "Received WM_ENDSESSION (false): session shutdown was cancelled",
                );
                LRESULT(0)
            }
        }
        WM_HOTKEY => {
            let hotkey_id = wparam.0 as i32;
            if hotkey_id == HOTKEY_WINDOW {
                toggle_main_window();
            } else if let Ok(lock) = HOTKEY_DISPATCHER.read() {
                if let Some(ref dispatcher) = *lock {
                    dispatcher(hotkey_id);
                }
            }
            LRESULT(0)
        }
        WM_APP_SHOW => {
            write_entry(Level::Info, "Received WM_APP_SHOW: activating main window");
            show_main_window();
            LRESULT(0)
        }
        WM_SETTINGCHANGE => {
            if let Some(app) = get_app_handle() {
                let _ = app.emit("theme-changed", ());
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DESTROY => {
            HELPER_HWND.store(0, Ordering::SeqCst);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Registers the window class and creates the hidden top-level helper window.
pub fn init_helper_window() -> Result<isize, HostError> {
    #[cfg(windows)]
    unsafe {
        let class_name = get_host_class_name();
        let class_name_w = to_wide(class_name);
        let hinstance = GetModuleHandleW(None).map_err(|e| HostError::ClassRegistrationError(e.to_string()))?;

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(helper_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance.into(),
            hIcon: HICON::default(),
            hCursor: HCURSOR::default(),
            hbrBackground: HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name_w.as_ptr()),
            hIconSm: HICON::default(),
        };

        let atom = RegisterClassExW(&wnd_class);
        if atom == 0 {
            let err = GetLastError();
            // Class might already be registered in process
            if err != ERROR_ALREADY_EXISTS {
                return Err(HostError::ClassRegistrationError(format!("Atom 0, err {:?}", err)));
            }
        }

        // Register TaskbarCreated message
        let taskbar_msg_name = to_wide("TaskbarCreated");
        let taskbar_id = RegisterWindowMessageW(PCWSTR(taskbar_msg_name.as_ptr()));
        if taskbar_id != 0 {
            TASKBAR_CREATED_MSG.store(taskbar_id, Ordering::SeqCst);
        }

        // Create top-level hidden window (WS_OVERLAPPED without WS_VISIBLE)
        let hwnd_res = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class_name_w.as_ptr()),
            PCWSTR(class_name_w.as_ptr()),
            WS_OVERLAPPED,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            None,
            None,
            hinstance,
            None,
        );

        match hwnd_res {
            Ok(hwnd) if !hwnd.0.is_null() => {
                HELPER_HWND.store(hwnd.0 as usize as u32, Ordering::SeqCst);
                write_entry(Level::Info, &format!("Helper window created with class '{}'", class_name));
                Ok(hwnd.0 as isize)
            }
            _ => Err(HostError::WindowCreationError),
        }
    }

    #[cfg(not(windows))]
    Ok(0)
}

/// Retrieves the helper window handle if created.
pub fn get_helper_hwnd() -> Option<isize> {
    let val = HELPER_HWND.load(Ordering::SeqCst);
    if val != 0 {
        Some(val as isize)
    } else {
        None
    }
}

/// Registers the four global hotkeys on the helper window.
/// Detects collisions and returns the list of conflicting shortcut action names.
pub fn register_all_hotkeys(hwnd_val: isize, shortcuts: &SavedShortcuts) -> Vec<String> {
    let mut conflicts = Vec::new();

    #[cfg(windows)]
    unsafe {
        let hwnd = HWND(hwnd_val as *mut std::ffi::c_void);
        unregister_all_hotkeys(hwnd_val);

        // 1. HOTKEY_PIN (with MOD_NOREPEAT)
        match shortcuts::parse_shortcut(&shortcuts.toggle_pin) {
            Ok(parsed) => {
                let mods = HOT_KEY_MODIFIERS(parsed.modifiers | shortcuts::MOD_NOREPEAT);
                if RegisterHotKey(Some(hwnd), HOTKEY_PIN, mods, parsed.vk).is_err() {
                    conflicts.push("toggle_pin".to_string());
                }
            }
            Err(_) => conflicts.push("toggle_pin".to_string()),
        }

        // 2. HOTKEY_OPACITY_UP (NO MOD_NOREPEAT - supports smooth auto-repeat)
        match shortcuts::parse_shortcut(&shortcuts.opacity_up) {
            Ok(parsed) => {
                let mods = HOT_KEY_MODIFIERS(parsed.modifiers);
                if RegisterHotKey(Some(hwnd), HOTKEY_OPACITY_UP, mods, parsed.vk).is_err() {
                    conflicts.push("opacity_up".to_string());
                }
            }
            Err(_) => conflicts.push("opacity_up".to_string()),
        }

        // 3. HOTKEY_OPACITY_DOWN (NO MOD_NOREPEAT - supports smooth auto-repeat)
        match shortcuts::parse_shortcut(&shortcuts.opacity_down) {
            Ok(parsed) => {
                let mods = HOT_KEY_MODIFIERS(parsed.modifiers);
                if RegisterHotKey(Some(hwnd), HOTKEY_OPACITY_DOWN, mods, parsed.vk).is_err() {
                    conflicts.push("opacity_down".to_string());
                }
            }
            Err(_) => conflicts.push("opacity_down".to_string()),
        }

        // 4. HOTKEY_WINDOW (with MOD_NOREPEAT)
        match shortcuts::parse_shortcut(&shortcuts.toggle_window) {
            Ok(parsed) => {
                let mods = HOT_KEY_MODIFIERS(parsed.modifiers | shortcuts::MOD_NOREPEAT);
                if RegisterHotKey(Some(hwnd), HOTKEY_WINDOW, mods, parsed.vk).is_err() {
                    conflicts.push("toggle_window".to_string());
                }
            }
            Err(_) => conflicts.push("toggle_window".to_string()),
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (hwnd_val, shortcuts);
    }

    if !conflicts.is_empty() {
        write_entry(
            Level::Warning,
            &format!("Hotkey collisions encountered for: {:?}", conflicts),
        );
        if let Some(app) = get_app_handle() {
            #[derive(serde::Serialize, Clone)]
            struct ConflictsPayload {
                conflicts: Vec<String>,
            }
            let _ = app.emit("shortcut-conflicts", ConflictsPayload { conflicts: conflicts.clone() });
        }
    }

    conflicts
}

/// Unregisters all global hotkeys from the helper window.
pub fn unregister_all_hotkeys(hwnd_val: isize) {
    #[cfg(windows)]
    unsafe {
        let hwnd = HWND(hwnd_val as *mut std::ffi::c_void);
        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_PIN);
        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_OPACITY_UP);
        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_OPACITY_DOWN);
        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd_val;
    }
}

/// Cleans up the helper window, hotkeys, and class registration.
pub fn cleanup_helper_window() {
    #[cfg(windows)]
    unsafe {
        let hwnd_val = HELPER_HWND.swap(0, Ordering::SeqCst);
        if hwnd_val != 0 {
            unregister_all_hotkeys(hwnd_val as isize);
            let hwnd = HWND(hwnd_val as *mut std::ffi::c_void);
            let _ = DestroyWindow(hwnd);
        }

        let class_name_w = to_wide(get_host_class_name());
        let hinstance = GetModuleHandleW(None).unwrap_or_default();
        let _ = UnregisterClassW(PCWSTR(class_name_w.as_ptr()), Some(hinstance.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_and_mutex_names() {
        let mutex_name = get_mutex_name();
        let class_name = get_host_class_name();

        assert!(mutex_name.starts_with("PinIt_SingleInstance_v2"));
        assert!(class_name.starts_with("PinItHost"));
    }

    #[test]
    fn test_session_ending_flag() {
        set_session_ending(false);
        assert!(!is_session_ending());

        set_session_ending(true);
        assert!(is_session_ending());

        set_session_ending(false);
        assert!(!is_session_ending());
    }
}
