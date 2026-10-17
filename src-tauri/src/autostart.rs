use std::path::Path;
use thiserror::Error;

use crate::logfile::{write_entry, Level};

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
#[cfg(windows)]
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_SZ, HKEY,
};

#[derive(Error, Debug)]
pub enum AutostartError {
    #[error("Failed to determine current executable path: {0}")]
    CurrentExeError(String),
    #[error("Registry operation failed with code: {0}")]
    RegistryError(u32),
}

const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Returns the registry value name: "PinIt-dev" in dev mode, "PinIt" in production.
pub fn get_value_name() -> &'static str {
    if crate::logfile::is_dev_mode() {
        "PinIt-dev"
    } else {
        "PinIt"
    }
}

/// Checks whether autostart is currently configured in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
pub fn is_autostart_enabled() -> bool {
    #[cfg(windows)]
    unsafe {
        let subkey_w = to_wide(RUN_SUBKEY);
        let value_w = to_wide(get_value_name());

        let mut hkey = HKEY::default();
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey_w.as_ptr()),
            None,
            KEY_READ,
            &mut hkey,
        );

        if status != ERROR_SUCCESS {
            return false;
        }

        let mut buffer = [0u16; 1024];
        let mut buffer_size: u32 = (buffer.len() * std::mem::size_of::<u16>()) as u32;

        let query_status = RegQueryValueExW(
            hkey,
            PCWSTR(value_w.as_ptr()),
            None,
            None,
            Some(buffer.as_mut_ptr() as *mut u8),
            Some(&mut buffer_size),
        );

        let _ = RegCloseKey(hkey);

        if query_status == ERROR_SUCCESS && buffer_size > 0 {
            let len = (buffer_size as usize / std::mem::size_of::<u16>()).saturating_sub(1);
            let val = String::from_utf16_lossy(&buffer[..len]);
            !val.trim().is_empty()
        } else {
            false
        }
    }

    #[cfg(not(windows))]
    false
}

/// Enables or disables autostart in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
pub fn set_autostart(enabled: bool) -> Result<(), AutostartError> {
    #[cfg(windows)]
    unsafe {
        let subkey_w = to_wide(RUN_SUBKEY);
        let value_w = to_wide(get_value_name());

        let mut hkey = HKEY::default();
        let open_status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey_w.as_ptr()),
            None,
            KEY_SET_VALUE | KEY_READ,
            &mut hkey,
        );

        if open_status != ERROR_SUCCESS {
            return Err(AutostartError::RegistryError(open_status.0));
        }

        let result = if enabled {
            let current_exe = std::env::current_exe()
                .map_err(|e| AutostartError::CurrentExeError(e.to_string()))?;
            let command = format!("\"{}\" --minimized", current_exe.to_string_lossy());
            let command_w = to_wide(&command);
            let bytes_len = (command_w.len() * std::mem::size_of::<u16>()) as u32;

            let set_res = RegSetValueExW(
                hkey,
                PCWSTR(value_w.as_ptr()),
                None,
                REG_SZ,
                Some(std::slice::from_raw_parts(command_w.as_ptr() as *const u8, bytes_len as usize)),
            );

            if set_res == ERROR_SUCCESS {
                write_entry(Level::Info, &format!("Autostart enabled with command: {}", command));
                Ok(())
            } else {
                Err(AutostartError::RegistryError(set_res.0))
            }
        } else {
            let del_res = RegDeleteValueW(hkey, PCWSTR(value_w.as_ptr()));
            if del_res == ERROR_SUCCESS || del_res == ERROR_FILE_NOT_FOUND {
                write_entry(Level::Info, "Autostart disabled in registry");
                Ok(())
            } else {
                Err(AutostartError::RegistryError(del_res.0))
            }
        };

        let _ = RegCloseKey(hkey);
        result
    }

    #[cfg(not(windows))]
    {
        let _ = enabled;
        Ok(())
    }
}

/// Parses an executable path from a registry command string formatted like:
/// `"C:\Path\To\pinontop.exe" --minimized`
pub fn extract_exe_path(command: &str) -> String {
    let trimmed = command.trim();
    if let Some(stripped) = trimmed.strip_prefix('"')
        && let Some(end_quote) = stripped.find('"') {
            return stripped[..end_quote].to_string();
        }

    // Unquoted fallback: split by space
    trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
}

/// Self-repairs the registry autostart entry if the executable was moved to a new location.
/// Returns Ok(true) if a repair was performed.
pub fn repair_autostart_path() -> Result<bool, AutostartError> {
    #[cfg(windows)]
    unsafe {
        let subkey_w = to_wide(RUN_SUBKEY);
        let value_w = to_wide(get_value_name());

        let mut hkey = HKEY::default();
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey_w.as_ptr()),
            None,
            KEY_READ,
            &mut hkey,
        );

        if status != ERROR_SUCCESS {
            return Ok(false);
        }

        let mut buffer = [0u16; 1024];
        let mut buffer_size: u32 = (buffer.len() * std::mem::size_of::<u16>()) as u32;

        let query_status = RegQueryValueExW(
            hkey,
            PCWSTR(value_w.as_ptr()),
            None,
            None,
            Some(buffer.as_mut_ptr() as *mut u8),
            Some(&mut buffer_size),
        );

        let _ = RegCloseKey(hkey);

        if query_status != ERROR_SUCCESS || buffer_size == 0 {
            return Ok(false);
        }

        let len = (buffer_size as usize / std::mem::size_of::<u16>()).saturating_sub(1);
        let registered_cmd = String::from_utf16_lossy(&buffer[..len]);
        let registered_exe = extract_exe_path(&registered_cmd);

        let current_exe = std::env::current_exe()
            .map_err(|e| AutostartError::CurrentExeError(e.to_string()))?;
        let current_exe_str = current_exe.to_string_lossy();

        if registered_exe.eq_ignore_ascii_case(&current_exe_str) {
            return Ok(false); // Already matches current location
        }

        // If registered path exists on disk, another installation owns it; do not touch
        if Path::new(&registered_exe).exists() {
            return Ok(false);
        }

        // Path is stale (file no longer exists); repair by updating to current path
        write_entry(
            Level::Info,
            &format!(
                "Repairing stale autostart path from '{}' to '{}'",
                registered_exe, current_exe_str
            ),
        );
        set_autostart(true)?;
        Ok(true)
    }

    #[cfg(not(windows))]
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_exe_path() {
        let quoted = r#""C:\Program Files\pinontop\pinontop.exe" --minimized"#;
        assert_eq!(
            extract_exe_path(quoted),
            r#"C:\Program Files\pinontop\pinontop.exe"#
        );

        let unquoted = r#"C:\pinontop\pinontop.exe --minimized"#;
        assert_eq!(extract_exe_path(unquoted), r#"C:\pinontop\pinontop.exe"#);
    }

    #[test]
    fn test_get_value_name() {
        let name = get_value_name();
        assert!(name.starts_with("PinIt"));
    }
}
