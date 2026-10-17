use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_LOG_SIZE_BYTES: u64 = 512 * 1024; // 512 KB
static LOG_LOCK: Mutex<()> = Mutex::new(());

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Warning,
}

impl Level {
    pub fn as_tag(&self) -> &'static str {
        match self {
            Level::Info => "I",
            Level::Warning => "W",
        }
    }
}

/// Returns true if running in developer mode (`PINIT_DEV=1` or debug build).
pub fn is_dev_mode() -> bool {
    std::env::var("PINIT_DEV").map(|v| v == "1").unwrap_or(false) || cfg!(debug_assertions)
}

/// Resolves the application data directory based on `PINIT_DEV` environment variable:
/// - Normal: `%LOCALAPPDATA%\PinIt`
/// - Dev: `%LOCALAPPDATA%\PinIt-dev`
pub fn get_app_data_dir() -> PathBuf {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
    let folder_name = if is_dev_mode() {
        "PinIt-dev"
    } else {
        "PinIt"
    };
    PathBuf::from(local_app_data).join(folder_name)
}

/// Returns the full path to `pinit.log`.
pub fn get_log_path() -> PathBuf {
    get_app_data_dir().join("pinit.log")
}

/// Returns the full path to `pinit.log.old`.
pub fn get_old_log_path() -> PathBuf {
    get_app_data_dir().join("pinit.log.old")
}

/// Formats current local system time as `YYYY-MM-DD HH:MM:SS.mmm`.
pub fn current_local_timestamp() -> String {
    #[cfg(windows)]
    {
        use windows::Win32::System::SystemInformation::GetLocalTime;
        let st = unsafe { GetLocalTime() };
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
            st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond, st.wMilliseconds
        )
    }
    #[cfg(not(windows))]
    {
        // Fallback for non-windows tests
        "2026-01-01 00:00:00.000".to_string()
    }
}

/// Formats a log line according to specification:
/// `YYYY-MM-DD HH:MM:SS.mmm [L] message\r\n`
pub fn format_log_line(timestamp: &str, level: Level, message: &str) -> String {
    format!("{} [{}] {}\r\n", timestamp, level.as_tag(), message)
}

/// Appends a message to the diagnostic log file.
///
/// PRIVACY HARD RULE:
/// Window titles must NEVER be passed to this function. Only process names,
/// error codes, and system events are permitted in logs.
pub fn write_entry(level: Level, message: &str) {
    let _guard = match LOG_LOCK.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    let dir = get_app_data_dir();
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }

    let log_path = get_log_path();
    let timestamp = current_local_timestamp();
    let formatted_line = format_log_line(&timestamp, level, message);

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = file.write_all(formatted_line.as_bytes());
        let _ = file.flush();
    }

    // Check size limit and rotate if needed
    if let Ok(metadata) = fs::metadata(&log_path) {
        if metadata.len() >= MAX_LOG_SIZE_BYTES {
            let old_path = get_old_log_path();
            let _ = fs::rename(&log_path, &old_path);

            // Write rotation notice into fresh log
            let rot_time = current_local_timestamp();
            let rot_line = format_log_line(&rot_time, Level::Warning, "Log rotated");
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
                let _ = file.write_all(rot_line.as_bytes());
                let _ = file.flush();
            }
        }
    }
}

/// Convenience macro for info-level log messages.
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logfile::write_entry($crate::logfile::Level::Info, &format!($($arg)*))
    };
}

/// Convenience macro for warning-level log messages.
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logfile::write_entry($crate::logfile::Level::Warning, &format!($($arg)*))
    };
}

/// Installs a panic hook that writes panic details to `pinit.log` before aborting.
pub fn init() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "Unknown panic"
        };

        let location = if let Some(loc) = panic_info.location() {
            format!("{}:{}", loc.file(), loc.line())
        } else {
            "unknown location".to_string()
        };

        write_entry(Level::Warning, &format!("PANIC at {}: {}", location, msg));

        default_hook(panic_info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_log_line() {
        let ts = "2026-10-17 14:23:01.042";
        let line = format_log_line(ts, Level::Info, "pinontop starting");
        assert_eq!(line, "2026-10-17 14:23:01.042 [I] pinontop starting\r\n");

        let warn_line = format_log_line(ts, Level::Warning, "Hotkeys unavailable");
        assert_eq!(warn_line, "2026-10-17 14:23:01.042 [W] Hotkeys unavailable\r\n");
    }

    #[test]
    fn test_level_tags() {
        assert_eq!(Level::Info.as_tag(), "I");
        assert_eq!(Level::Warning.as_tag(), "W");
    }

    #[test]
    fn test_app_data_dir_resolution() {
        // Without PINIT_DEV
        unsafe { std::env::remove_var("PINIT_DEV") };
        let normal_dir = get_app_data_dir();
        assert!(normal_dir.ends_with("PinIt"));

        // With PINIT_DEV=1
        unsafe { std::env::set_var("PINIT_DEV", "1") };
        let dev_dir = get_app_data_dir();
        assert!(dev_dir.ends_with("PinIt-dev"));

        unsafe { std::env::remove_var("PINIT_DEV") };
    }
}
