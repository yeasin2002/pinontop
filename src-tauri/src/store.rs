use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::logfile::{get_app_data_dir, Level, write_entry};

/// Lossless integer conversion: percent (20–100) -> 8-bit alpha (0–255).
/// Formula: `alpha = (percent * 255 + 50) / 100` [integer division, rounded]
pub fn percent_to_alpha(percent: u8) -> u8 {
    let p = percent.clamp(20, 100) as u32;
    ((p * 255 + 50) / 100) as u8
}

/// Lossless integer conversion: 8-bit alpha (0–255) -> percent (20–100).
/// Formula: `percent = (alpha * 100 + 127) / 255` [integer division, rounded]
pub fn alpha_to_percent(alpha: u8) -> u8 {
    let a = alpha as u32;
    (((a * 100 + 127) / 255) as u8).clamp(20, 100)
}

/// Representation of a single pin as persisted in `pinned.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedPin {
    #[serde(default)]
    pub process_name: String,
    #[serde(default)]
    pub title: String,
    #[serde(default = "default_opacity")]
    pub opacity: u8, // 0–255 alpha value (default: 255)
    #[serde(default)]
    pub click_through: bool,
    #[serde(default = "default_true")]
    pub was_layered: bool,
    #[serde(default = "default_true")]
    pub was_topmost: bool,
    #[serde(default = "default_true")]
    pub was_click_through: bool,

    /// Preserves unknown legacy and future fields verbatim
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

fn default_opacity() -> u8 {
    255
}

fn default_true() -> bool {
    true
}

impl Default for SavedPin {
    fn default() -> Self {
        Self {
            process_name: String::new(),
            title: String::new(),
            opacity: 255,
            click_through: false,
            was_layered: true,
            was_topmost: true,
            was_click_through: true,
            extra: HashMap::new(),
        }
    }
}

/// Saved keyboard shortcut strings as persisted in `pinned.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedShortcuts {
    #[serde(default = "default_shortcut_toggle_pin")]
    pub toggle_pin: String,
    #[serde(default = "default_shortcut_opacity_up")]
    pub opacity_up: String,
    #[serde(default = "default_shortcut_opacity_down")]
    pub opacity_down: String,
    #[serde(default = "default_shortcut_toggle_window")]
    pub toggle_window: String,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

fn default_shortcut_toggle_pin() -> String {
    "super+ctrl+KeyT".to_string()
}
fn default_shortcut_opacity_up() -> String {
    "super+ctrl+Equal".to_string()
}
fn default_shortcut_opacity_down() -> String {
    "super+ctrl+Minus".to_string()
}
fn default_shortcut_toggle_window() -> String {
    "super+ctrl+KeyP".to_string()
}

impl Default for SavedShortcuts {
    fn default() -> Self {
        Self {
            toggle_pin: default_shortcut_toggle_pin(),
            opacity_up: default_shortcut_opacity_up(),
            opacity_down: default_shortcut_opacity_down(),
            toggle_window: default_shortcut_toggle_window(),
            extra: HashMap::new(),
        }
    }
}

/// Application settings as persisted in `pinned.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSettings {
    #[serde(default = "default_true")]
    pub enable_sound: bool,
    #[serde(default)]
    pub show_notifications: bool,
    #[serde(default)]
    pub has_seen_tray_notice: bool,
    #[serde(default)]
    pub start_with_windows: bool,
    #[serde(default)]
    pub shortcuts: SavedShortcuts,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Default for SavedSettings {
    fn default() -> Self {
        Self {
            enable_sound: true,
            show_notifications: false,
            has_seen_tray_notice: false,
            start_with_windows: false,
            shortcuts: SavedShortcuts::default(),
            extra: HashMap::new(),
        }
    }
}

/// Top-level representation of `pinned.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub struct ConfigFile {
    #[serde(default)]
    pub pins: HashMap<String, SavedPin>,
    #[serde(default)]
    pub settings: SavedSettings,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}


/// Full application persisted state ready for consumption by pins/commands layer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppStateStore {
    pub pins: Vec<SavedPin>,
    pub settings: SavedSettings,
    pub top_level_extra: HashMap<String, serde_json::Value>,
}

pub fn get_storage_path() -> PathBuf {
    get_app_data_dir().join("pinned.json")
}

pub fn get_temp_storage_path() -> PathBuf {
    get_app_data_dir().join("pinned.json.tmp")
}

pub fn get_corrupt_storage_path() -> PathBuf {
    get_app_data_dir().join("pinned.json.corrupt")
}

/// Parses the index integer from keys formatted as `"<process_name>:<index>"`.
fn parse_index_from_key(key: &str) -> Option<usize> {
    key.rsplit_once(':')
        .and_then(|(_, idx_str)| idx_str.parse::<usize>().ok())
}

/// Reads and parses `pinned.json` from the specified path.
///
/// Behavior on:
/// - Missing / empty file: returns defaults without error.
/// - Corrupt file: backs up to `pinned.json.corrupt` and returns defaults without crashing.
pub fn load_from_path(path: &Path) -> AppStateStore {
    if !path.exists() {
        return AppStateStore::default();
    }

    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            write_entry(Level::Warning, &format!("Could not read pinned.json: {}", e));
            return AppStateStore::default();
        }
    };

    if contents.trim().is_empty() {
        return AppStateStore::default();
    }

    match serde_json::from_str::<ConfigFile>(&contents) {
        Ok(config) => {
            // Sort pins by index parsed from key
            let mut indexed_pins: Vec<(usize, SavedPin)> = Vec::new();
            for (key, pin) in config.pins {
                if pin.process_name.trim().is_empty() {
                    continue;
                }
                let index = parse_index_from_key(&key).unwrap_or(usize::MAX);
                indexed_pins.push((index, pin));
            }

            indexed_pins.sort_by_key(|(idx, _)| *idx);
            let pins: Vec<SavedPin> = indexed_pins.into_iter().map(|(_, pin)| pin).collect();

            AppStateStore {
                pins,
                settings: config.settings,
                top_level_extra: config.extra,
            }
        }
        Err(err) => {
            write_entry(
                Level::Warning,
                &format!("pinned.json is corrupt ({}); backing it up to pinned.json.corrupt", err),
            );

            // Rename to .corrupt if in standard storage location
            let corrupt_path = path.with_extension("json.corrupt");
            let _ = fs::rename(path, corrupt_path);

            AppStateStore::default()
        }
    }
}

/// Loads application state from the standard `%LOCALAPPDATA%\PinIt\pinned.json`.
pub fn load() -> AppStateStore {
    load_from_path(&get_storage_path())
}

/// Saves application state atomically to the specified path:
/// writes to `<path>.tmp` and renames over `<path>`.
pub fn save_to_path(path: &Path, state: &AppStateStore) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

    // Convert ordered pin vector back into `<process_name>:<i>` map
    let mut pins_map = HashMap::new();
    for (i, pin) in state.pins.iter().enumerate() {
        let key = format!("{}:{}", pin.process_name, i);
        pins_map.insert(key, pin.clone());
    }

    let config = ConfigFile {
        pins: pins_map,
        settings: state.settings.clone(),
        extra: state.top_level_extra.clone(),
    };

    let serialized = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;

    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, serialized.as_bytes()).map_err(|e| {
        write_entry(Level::Warning, &format!("Could not save tmp file: {}", e));
        e.to_string()
    })?;

    // Atomic replace
    fs::rename(&tmp_path, path).map_err(|e| {
        write_entry(Level::Warning, &format!("Could not rename tmp file over pinned.json: {}", e));
        e.to_string()
    })?;

    Ok(())
}

/// Saves application state atomically to standard `%LOCALAPPDATA%\PinIt\pinned.json`.
pub fn save(state: &AppStateStore) -> Result<(), String> {
    save_to_path(&get_storage_path(), state)
}

/// Determines whether a live window matches a saved pending pin:
/// - Process name match is case-insensitive.
/// - Title match is exact and case-sensitive.
/// - Empty saved titles never match.
pub fn matches_pending(saved: &SavedPin, live_process: &str, live_title: &str) -> bool {
    if saved.title.trim().is_empty() {
        return false;
    }
    saved.process_name.eq_ignore_ascii_case(live_process) && saved.title == live_title
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opacity_integer_roundtrip() {
        // Every percentage between 20 and 100 must round-trip with 0% drift
        for pct in 20..=100 {
            let alpha = percent_to_alpha(pct);
            let back = alpha_to_percent(alpha);
            assert_eq!(
                pct, back,
                "Drift detected at {}%: alpha={}, back={}%",
                pct, alpha, back
            );
        }

        // Test known anchor points from specification
        assert_eq!(percent_to_alpha(100), 255);
        assert_eq!(percent_to_alpha(90), 230);
        assert_eq!(percent_to_alpha(50), 128);
        assert_eq!(percent_to_alpha(20), 51);
    }

    #[test]
    fn test_matching_logic() {
        let pin = SavedPin {
            process_name: "notepad.exe".to_string(),
            title: "notes.txt - Notepad".to_string(),
            ..Default::default()
        };

        // Case-insensitive process, exact title
        assert!(matches_pending(&pin, "NOTEPAD.EXE", "notes.txt - Notepad"));
        assert!(matches_pending(&pin, "notepad.exe", "notes.txt - Notepad"));

        // Mismatched title
        assert!(!matches_pending(&pin, "notepad.exe", "other.txt - Notepad"));
        assert!(!matches_pending(&pin, "notepad.exe", "Notes.txt - Notepad"));

        // Empty title matches nothing
        let empty_title_pin = SavedPin {
            process_name: "notepad.exe".to_string(),
            title: "".to_string(),
            ..Default::default()
        };
        assert!(!matches_pending(&empty_title_pin, "notepad.exe", "any"));
    }

    #[test]
    fn test_schema_roundtrip_and_unknown_fields() {
        let json_input = r#"{
            "pins": {
                "chrome.exe:1": {
                    "process_name": "chrome.exe",
                    "title": "Google",
                    "opacity": 230,
                    "click_through": true,
                    "was_layered": false,
                    "was_topmost": true,
                    "was_click_through": false,
                    "future_pin_field": 42
                },
                "notepad.exe:0": {
                    "process_name": "notepad.exe",
                    "title": "notes.txt",
                    "opacity": 255,
                    "click_through": false,
                    "was_layered": true,
                    "was_topmost": true,
                    "was_click_through": true
                }
            },
            "settings": {
                "enable_sound": false,
                "show_notifications": true,
                "has_seen_tray_notice": true,
                "start_with_windows": false,
                "shortcuts": {
                    "toggle_pin": "super+ctrl+KeyT",
                    "opacity_up": "super+ctrl+Equal",
                    "opacity_down": "super+ctrl+Minus",
                    "toggle_window": "super+ctrl+KeyP",
                    "custom_hotkey": "alt+KeyF"
                },
                "future_setting": "preserved"
            },
            "top_level_future": true
        }"#;

        let temp_dir = std::env::temp_dir().join(format!("pinontop_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let test_file = temp_dir.join("test_pinned.json");

        fs::write(&test_file, json_input).unwrap();

        let state = load_from_path(&test_file);

        // Verify sorted ordering by index (0, then 1)
        assert_eq!(state.pins.len(), 2);
        assert_eq!(state.pins[0].process_name, "notepad.exe");
        assert_eq!(state.pins[1].process_name, "chrome.exe");
        assert_eq!(state.pins[1].opacity, 230);
        assert_eq!(state.pins[1].extra.get("future_pin_field").unwrap(), 42);

        // Verify settings and unknown fields preserved
        assert!(!state.settings.enable_sound);
        assert!(state.settings.show_notifications);
        assert_eq!(
            state.settings.extra.get("future_setting").unwrap(),
            "preserved"
        );
        assert_eq!(
            state.settings.shortcuts.extra.get("custom_hotkey").unwrap(),
            "alt+KeyF"
        );
        assert_eq!(state.top_level_extra.get("top_level_future").unwrap(), true);

        // Test saving back
        save_to_path(&test_file, &state).unwrap();
        let reloaded = load_from_path(&test_file);

        assert_eq!(state.pins, reloaded.pins);
        assert_eq!(state.settings, reloaded.settings);
        assert_eq!(state.top_level_extra, reloaded.top_level_extra);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_corrupt_file_handling() {
        let temp_dir = std::env::temp_dir().join(format!("pinontop_corrupt_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let test_file = temp_dir.join("pinned.json");

        fs::write(&test_file, "{ this is not valid json").unwrap();

        let state = load_from_path(&test_file);

        // Must return default state
        assert_eq!(state.pins.len(), 0);
        assert!(state.settings.enable_sound);

        // Must rename corrupt file to .corrupt
        let corrupt_path = temp_dir.join("pinned.json.corrupt");
        assert!(corrupt_path.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
