use std::path::Path;
use thiserror::Error;

#[cfg(windows)]
use windows::core::{BOOL, PWSTR};
#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, COLORREF, HWND, LPARAM, RECT};
#[cfg(windows)]
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetForegroundWindow, GetWindowLongW, GetWindowRect,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow,
    IsWindowVisible, SetLayeredWindowAttributes, SetWindowLongW, SetWindowPos,
    SystemParametersInfoW, GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, LWA_ALPHA,
    SPI_GETCLIENTAREAANIMATION, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT,
};

#[derive(Error, Debug, PartialEq, Eq)]
pub enum WinError {
    #[error("Invalid or closed window handle")]
    InvalidWindow,
    #[error("Window is not pinnable: {0}")]
    NotPinnable(String),
    #[error("Pin failed for process '{0}' (likely elevated/UIPI block)")]
    UipiBlocked(String),
    #[error("Win32 error: {0}")]
    SystemError(String),
}

/// Initial styles recorded prior to pinning a window, used for clean restoration on unpin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowInitialStyles {
    pub was_topmost: bool,
    pub was_layered: bool,
    pub was_click_through: bool,
}

/// Discovered top-level window suitable for display in the Window Picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredWindow {
    pub hwnd: isize,
    pub title: String,
    pub process_name: String,
}

/// The blacklist of Windows shell classes that must NEVER be pinned.
pub const SHELL_WINDOW_CLASSES: &[&str] = &[
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "NotifyIconOverflowWindow",
    "TopLevelWindowForOverflowXamlIsland",
    "Windows.UI.Core.CoreWindow",
    "XamlExplorerHostIslandWindow",
    "MultitaskingViewFrame",
    "TaskListThumbnailWnd",
    "ForegroundStaging",
];

/// Checks if a window class name belongs to the Windows shell.
pub fn is_shell_window_class(class_name: &str) -> bool {
    let trimmed = class_name.trim();
    SHELL_WINDOW_CLASSES
        .iter()
        .any(|&shell_class| shell_class.eq_ignore_ascii_case(trimmed))
}

#[cfg(windows)]
#[inline]
pub fn to_hwnd(hwnd: isize) -> HWND {
    HWND(hwnd as *mut std::ffi::c_void)
}

/// Retrieves the current process ID of this application.
pub fn get_current_process_id() -> u32 {
    #[cfg(windows)]
    unsafe {
        GetCurrentProcessId()
    }
    #[cfg(not(windows))]
    0
}

/// Checks whether an HWND is valid and still exists.
pub fn is_valid_window(hwnd: isize) -> bool {
    #[cfg(windows)]
    unsafe {
        IsWindow(Some(to_hwnd(hwnd))).as_bool()
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        true
    }
}

/// Retrieves the extended window style bitmask (`GWL_EXSTYLE`).
pub fn get_extended_style(hwnd: isize) -> u32 {
    #[cfg(windows)]
    unsafe {
        GetWindowLongW(to_hwnd(hwnd), GWL_EXSTYLE) as u32
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        0
    }
}

/// Sets the extended window style bitmask (`GWL_EXSTYLE`).
pub fn set_extended_style(hwnd: isize, style: u32) -> Result<(), WinError> {
    #[cfg(windows)]
    unsafe {
        let res = SetWindowLongW(to_hwnd(hwnd), GWL_EXSTYLE, style as i32);
        if res == 0 && !is_valid_window(hwnd) {
            return Err(WinError::InvalidWindow);
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (hwnd, style);
        Ok(())
    }
}

/// Returns whether `WS_EX_TOPMOST` is set on the window.
pub fn is_topmost(hwnd: isize) -> bool {
    #[cfg(windows)]
    {
        (get_extended_style(hwnd) & WS_EX_TOPMOST.0) != 0
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        false
    }
}

/// Returns whether `WS_EX_LAYERED` is set on the window.
pub fn is_layered(hwnd: isize) -> bool {
    #[cfg(windows)]
    {
        (get_extended_style(hwnd) & WS_EX_LAYERED.0) != 0
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        false
    }
}

/// Returns whether `WS_EX_TRANSPARENT` is set on the window.
pub fn is_click_through(hwnd: isize) -> bool {
    #[cfg(windows)]
    {
        (get_extended_style(hwnd) & WS_EX_TRANSPARENT.0) != 0
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        false
    }
}

/// Reads the window class name via `GetClassNameW`.
pub fn get_window_class_name(hwnd: isize) -> String {
    #[cfg(windows)]
    unsafe {
        let mut buffer = [0u16; 256];
        let len = GetClassNameW(to_hwnd(hwnd), &mut buffer);
        if len > 0 {
            String::from_utf16_lossy(&buffer[..len as usize])
        } else {
            String::new()
        }
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        String::new()
    }
}

/// Reads the window title via `GetWindowTextW`.
pub fn get_window_title(hwnd: isize) -> String {
    #[cfg(windows)]
    unsafe {
        let h = to_hwnd(hwnd);
        let len = GetWindowTextLengthW(h);
        if len <= 0 {
            return String::new();
        }
        let mut buffer = vec![0u16; (len + 1) as usize];
        let read = GetWindowTextW(h, &mut buffer);
        if read > 0 {
            String::from_utf16_lossy(&buffer[..read as usize])
        } else {
            String::new()
        }
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        String::new()
    }
}

/// Retrieves the PID and executable name owning the window using `QueryFullProcessImageNameW`.
pub fn get_window_process_name_and_pid(hwnd: isize) -> Result<(String, u32), WinError> {
    #[cfg(windows)]
    unsafe {
        let mut pid: u32 = 0;
        let h = to_hwnd(hwnd);
        GetWindowThreadProcessId(h, Some(&mut pid));
        if pid == 0 {
            return Err(WinError::InvalidWindow);
        }

        let proc_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
        if let Ok(handle) = proc_handle {
            let mut buffer = [0u16; 1024];
            let mut len: u32 = buffer.len() as u32;
            let res = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_FORMAT(0),
                PWSTR(buffer.as_mut_ptr()),
                &mut len,
            );
            let _ = CloseHandle(handle);

            if res.is_ok() && len > 0 {
                let full_path = String::from_utf16_lossy(&buffer[..len as usize]);
                let file_name = Path::new(&full_path)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("unknown.exe")
                    .to_string();
                return Ok((file_name, pid));
            }
        }

        Ok(("unknown.exe".to_string(), pid))
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        Ok(("notepad.exe".to_string(), 1234))
    }
}

/// Core validation rule: check if a window is eligible to be pinned.
pub fn is_pinnable(hwnd: isize, own_pid: u32) -> bool {
    if !is_valid_window(hwnd) {
        return false;
    }

    let (proc_name, pid) = match get_window_process_name_and_pid(hwnd) {
        Ok(res) => res,
        Err(_) => return false,
    };

    if pid == own_pid {
        return false;
    }

    let class_name = get_window_class_name(hwnd);
    if is_shell_window_class(&class_name) {
        return false;
    }

    // Reject known shell executables hosting system desktop/shell surfaces
    if proc_name.eq_ignore_ascii_case("explorer.exe") && is_shell_window_class(&class_name) {
        return false;
    }

    true
}

/// Pins a window to `HWND_TOPMOST` with full UIPI detection.
pub fn pin_window(hwnd: isize, own_pid: u32) -> Result<WindowInitialStyles, WinError> {
    if !is_pinnable(hwnd, own_pid) {
        return Err(WinError::NotPinnable("Window is not pinnable".to_string()));
    }

    let initial = WindowInitialStyles {
        was_topmost: is_topmost(hwnd),
        was_layered: is_layered(hwnd),
        was_click_through: is_click_through(hwnd),
    };

    #[cfg(windows)]
    unsafe {
        let h = to_hwnd(hwnd);
        let _ = SetWindowPos(
            h,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );

        // UIPI verification: verify WS_EX_TOPMOST actually took effect
        if !is_topmost(hwnd) {
            let (proc_name, _) = get_window_process_name_and_pid(hwnd)
                .unwrap_or_else(|_| ("unknown".to_string(), 0));
            return Err(WinError::UipiBlocked(proc_name));
        }
    }

    Ok(initial)
}

/// Unpins a window, safely restoring initial styles as recorded at pin time.
pub fn unpin_window(
    hwnd: isize,
    initial: &WindowInitialStyles,
    had_opacity_change: bool,
    current_click_through: bool,
) -> Result<(), WinError> {
    if !is_valid_window(hwnd) {
        return Ok(()); // Window closed, nothing to restore visually
    }

    #[cfg(windows)]
    unsafe {
        let h = to_hwnd(hwnd);

        // 1. Restore click-through
        if current_click_through && !initial.was_click_through {
            let current = get_extended_style(hwnd);
            let _ = set_extended_style(hwnd, current & !WS_EX_TRANSPARENT.0);
        }

        // 2. Restore opacity
        if had_opacity_change {
            let _ = SetLayeredWindowAttributes(h, COLORREF(0), 255, LWA_ALPHA);
            if !initial.was_layered {
                let current = get_extended_style(hwnd);
                let _ = set_extended_style(hwnd, current & !WS_EX_LAYERED.0);
                let _ = SetWindowPos(
                    h,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }
        }

        // 3. Restore topmost
        if !initial.was_topmost {
            let _ = SetWindowPos(
                h,
                Some(HWND_NOTOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    Ok(())
}

/// Applies 8-bit alpha (0–255) to a window using the layered model.
pub fn set_window_opacity(hwnd: isize, alpha: u8) -> Result<(), WinError> {
    if !is_valid_window(hwnd) {
        return Err(WinError::InvalidWindow);
    }

    #[cfg(windows)]
    unsafe {
        let h = to_hwnd(hwnd);
        if !is_layered(hwnd) {
            let current = get_extended_style(hwnd);
            set_extended_style(hwnd, current | WS_EX_LAYERED.0)?;
        }
        SetLayeredWindowAttributes(h, COLORREF(0), alpha, LWA_ALPHA)
            .map_err(|e| WinError::SystemError(e.to_string()))?;
    }

    Ok(())
}

/// Sets or removes mouse click passthrough (`WS_EX_TRANSPARENT`).
pub fn set_window_click_through(
    hwnd: isize,
    enabled: bool,
    current_alpha: u8,
) -> Result<(), WinError> {
    if !is_valid_window(hwnd) {
        return Err(WinError::InvalidWindow);
    }

    #[cfg(windows)]
    unsafe {
        let h = to_hwnd(hwnd);

        // Windows requires WS_EX_LAYERED with an alpha for WS_EX_TRANSPARENT to take effect
        if enabled && !is_layered(hwnd) {
            let current = get_extended_style(hwnd);
            set_extended_style(hwnd, current | WS_EX_LAYERED.0)?;
            let _ = SetLayeredWindowAttributes(h, COLORREF(0), current_alpha, LWA_ALPHA);
        }

        let current = get_extended_style(hwnd);
        let next_style = if enabled {
            current | WS_EX_TRANSPARENT.0
        } else {
            current & !WS_EX_TRANSPARENT.0
        };

        set_extended_style(hwnd, next_style)?;
    }

    Ok(())
}

/// Retrieves the current foreground window handle if available.
pub fn get_foreground_window() -> Option<isize> {
    #[cfg(windows)]
    unsafe {
        let h = GetForegroundWindow();
        if h.0.is_null() {
            None
        } else {
            Some(h.0 as isize)
        }
    }
    #[cfg(not(windows))]
    None
}

/// Checks whether Windows UI animations are enabled globally.
pub fn are_animations_enabled() -> bool {
    #[cfg(windows)]
    unsafe {
        let mut enabled: BOOL = BOOL(0);
        let res = SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some(&mut enabled as *mut _ as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        if res.is_ok() {
            enabled.as_bool()
        } else {
            true
        }
    }
    #[cfg(not(windows))]
    true
}

/// Enumerates all pinnable windows currently open on the system for WindowPicker.
pub fn get_pinnable_windows(own_pid: u32) -> Vec<DiscoveredWindow> {
    let mut discovered: Vec<DiscoveredWindow> = Vec::new();

    #[cfg(windows)]
    unsafe {
        struct EnumContext {
            list: *mut Vec<DiscoveredWindow>,
            own_pid: u32,
        }

        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            unsafe {
                let ctx = &mut *(lparam.0 as *mut EnumContext);
                let h_val = hwnd.0 as isize;

                if !IsWindowVisible(hwnd).as_bool() {
                    return BOOL(1);
                }

                if IsIconic(hwnd).as_bool() {
                    return BOOL(1);
                }

                let mut rect = RECT::default();
                if GetWindowRect(hwnd, &mut rect).is_ok() {
                    let width = rect.right - rect.left;
                    let height = rect.bottom - rect.top;
                    if width <= 0 || height <= 0 {
                        return BOOL(1);
                    }
                }

                let ex_style = get_extended_style(h_val);
                if (ex_style & WS_EX_TOOLWINDOW.0) != 0 {
                    return BOOL(1);
                }

                // Check if window is cloaked (e.g. background UWP application)
                let mut cloaked: u32 = 0;
                let dwm_res = DwmGetWindowAttribute(
                    hwnd,
                    DWMWA_CLOAKED,
                    &mut cloaked as *mut _ as *mut _,
                    std::mem::size_of::<u32>() as u32,
                );
                if dwm_res.is_ok() && cloaked != 0 {
                    return BOOL(1);
                }

                if !is_pinnable(h_val, ctx.own_pid) {
                    return BOOL(1);
                }

                let title = get_window_title(h_val);
                if title.trim().is_empty() {
                    return BOOL(1);
                }

                let (process_name, _) = get_window_process_name_and_pid(h_val)
                    .unwrap_or_else(|_| ("unknown.exe".to_string(), 0));

                (*ctx.list).push(DiscoveredWindow {
                    hwnd: h_val,
                    title,
                    process_name,
                });

                BOOL(1)
            }
        }

        let mut context = EnumContext {
            list: &mut discovered,
            own_pid,
        };

        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut context as *mut _ as isize));
    }

    discovered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_blacklist() {
        assert!(is_shell_window_class("Progman"));
        assert!(is_shell_window_class("progman"));
        assert!(is_shell_window_class("WorkerW"));
        assert!(is_shell_window_class("Shell_TrayWnd"));
        assert!(is_shell_window_class("Shell_SecondaryTrayWnd"));
        assert!(is_shell_window_class("NotifyIconOverflowWindow"));
        assert!(is_shell_window_class("TopLevelWindowForOverflowXamlIsland"));
        assert!(is_shell_window_class("Windows.UI.Core.CoreWindow"));
        assert!(is_shell_window_class("XamlExplorerHostIslandWindow"));
        assert!(is_shell_window_class("MultitaskingViewFrame"));
        assert!(is_shell_window_class("TaskListThumbnailWnd"));
        assert!(is_shell_window_class("ForegroundStaging"));

        // Regular application windows must NOT be blacklisted
        assert!(!is_shell_window_class("Notepad"));
        assert!(!is_shell_window_class("Chrome_WidgetWin_1"));
        assert!(!is_shell_window_class("MozillaWindowClass"));
        assert!(!is_shell_window_class("CabinetWClass")); // Normal Windows Explorer folder
    }

    #[test]
    fn test_initial_styles_struct() {
        let initial = WindowInitialStyles {
            was_topmost: false,
            was_layered: true,
            was_click_through: false,
        };
        assert!(!initial.was_topmost);
        assert!(initial.was_layered);
        assert!(!initial.was_click_through);
    }

    #[test]
    fn test_is_pinnable_invalid_hwnd() {
        // Handle 0 is never valid
        assert!(!is_pinnable(0, 999999));
    }
}
