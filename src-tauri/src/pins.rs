use std::sync::{Arc, OnceLock, RwLock};
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use thiserror::Error;

use crate::flash::{flash_window, FlashKind};
use crate::host::{self, is_session_ending};
use crate::logfile::{write_entry, Level};
use crate::shortcuts::{HOTKEY_OPACITY_DOWN, HOTKEY_OPACITY_UP, HOTKEY_PIN};
use crate::store::{
    self, alpha_to_percent, percent_to_alpha, AppStateStore, SavedPin, SavedSettings,
};
use crate::tray::update_tray_menu;
use crate::win::{
    self, get_foreground_window, is_pinnable, is_topmost, is_valid_window,
    set_window_click_through, set_window_opacity, unpin_window, WinError, WindowInitialStyles,
};

#[derive(Error, Debug)]
pub enum PinManagerError {
    #[error("Win32 error: {0}")]
    Win(#[from] WinError),
    #[error("Window not found")]
    NotFound,
    #[error("Storage error: {0}")]
    Storage(String),
}

/// A currently active pinned window in memory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LivePin {
    pub hwnd: isize,
    pub title: String,
    pub process_name: String,
    pub opacity: u8, // 20 to 100 percentage
    pub click_through: bool,
    pub was_layered: bool,
    pub was_topmost: bool,
    pub was_click_through: bool,
    pub opacity_changed: bool,
}

/// Flattened representation sent to the React frontend.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrontendPinItem {
    pub hwnd: Option<isize>,
    pub process_name: String,
    pub title: String,
    pub opacity: u8,
    pub click_through: bool,
    pub was_layered: Option<bool>,
    pub was_topmost: Option<bool>,
    pub was_click_through: Option<bool>,
    pub is_pending: bool,
}

pub struct PinManager {
    pub live_pins: Vec<LivePin>,
    pub pending_pins: Vec<SavedPin>,
    pub settings: SavedSettings,
    pub app_handle: Option<AppHandle>,
}

static MANAGER: OnceLock<Arc<RwLock<PinManager>>> = OnceLock::new();

pub fn get_manager() -> &'static Arc<RwLock<PinManager>> {
    MANAGER.get_or_init(|| {
        Arc::new(RwLock::new(PinManager {
            live_pins: Vec::new(),
            pending_pins: Vec::new(),
            settings: SavedSettings::default(),
            app_handle: None,
        }))
    })
}

impl PinManager {
    /// Builds the list of `FrontendPinItem`s combining live and pending pins.
    pub fn to_frontend_items(&self) -> Vec<FrontendPinItem> {
        let mut items = Vec::new();

        // 1. Live pins
        for p in &self.live_pins {
            items.push(FrontendPinItem {
                hwnd: Some(p.hwnd),
                process_name: p.process_name.clone(),
                title: p.title.clone(),
                opacity: p.opacity,
                click_through: p.click_through,
                was_layered: Some(p.was_layered),
                was_topmost: Some(p.was_topmost),
                was_click_through: Some(p.was_click_through),
                is_pending: false,
            });
        }

        // 2. Pending pins
        for p in &self.pending_pins {
            items.push(FrontendPinItem {
                hwnd: None,
                process_name: p.process_name.clone(),
                title: p.title.clone(),
                opacity: alpha_to_percent(p.opacity),
                click_through: p.click_through,
                was_layered: Some(p.was_layered),
                was_topmost: Some(p.was_topmost),
                was_click_through: Some(p.was_click_through),
                is_pending: true,
            });
        }

        items
    }

    /// Emits `pins-changed` event to the frontend and updates the system tray.
    pub fn notify_change(&self) {
        if let Some(ref app) = self.app_handle {
            let items = self.to_frontend_items();
            let _ = app.emit("pins-changed", &items);

            let tray_pins: Vec<(isize, String)> = self
                .live_pins
                .iter()
                .map(|p| (p.hwnd, p.title.clone()))
                .collect();
            update_tray_menu(app, &tray_pins);
        }
    }

    /// Persists the current in-memory pins and settings to disk.
    pub fn save_to_disk(&self) -> Result<(), String> {
        let mut saved_pins = Vec::new();

        // Include live pins
        for p in &self.live_pins {
            saved_pins.push(SavedPin {
                process_name: p.process_name.clone(),
                title: p.title.clone(),
                opacity: percent_to_alpha(p.opacity),
                click_through: p.click_through,
                was_layered: p.was_layered,
                was_topmost: p.was_topmost,
                was_click_through: p.was_click_through,
                extra: std::collections::HashMap::new(),
            });
        }

        // Include pending pins
        for p in &self.pending_pins {
            saved_pins.push(p.clone());
        }

        let store_state = AppStateStore {
            pins: saved_pins,
            settings: self.settings.clone(),
            top_level_extra: std::collections::HashMap::new(),
        };

        store::save(&store_state)
    }

    /// Toggles the pin status of an arbitrary window handle.
    pub fn toggle_pin(&mut self, hwnd: isize) -> Result<bool, WinError> {
        // If already in live pins, unpin it
        if let Some(pos) = self.live_pins.iter().position(|p| p.hwnd == hwnd) {
            let pin = self.live_pins.remove(pos);

            let initial = WindowInitialStyles {
                was_topmost: pin.was_topmost,
                was_layered: pin.was_layered,
                was_click_through: pin.was_click_through,
            };

            let _ = unpin_window(hwnd, &initial, pin.opacity_changed, pin.click_through);
            flash_window(hwnd, FlashKind::Unpin);

            let _ = self.save_to_disk();
            self.notify_change();
            return Ok(false);
        }

        // Otherwise, attempt to pin it
        let own_pid = win::get_current_process_id();
        let initial = win::pin_window(hwnd, own_pid)?;

        let (proc_name, _) = win::get_window_process_name_and_pid(hwnd)
            .unwrap_or_else(|_| ("unknown.exe".to_string(), 0));
        let title = win::get_window_title(hwnd);

        // Remove from pending pins if previously matched
        self.pending_pins
            .retain(|p| !(p.process_name.eq_ignore_ascii_case(&proc_name) && p.title == title));

        self.live_pins.push(LivePin {
            hwnd,
            title,
            process_name: proc_name,
            opacity: 100,
            click_through: initial.was_click_through,
            was_layered: initial.was_layered,
            was_topmost: initial.was_topmost,
            was_click_through: initial.was_click_through,
            opacity_changed: false,
        });

        flash_window(hwnd, FlashKind::Pin);

        let _ = self.save_to_disk();
        self.notify_change();
        Ok(true)
    }

    /// Sets the opacity (20–100%) on a pinned window.
    pub fn set_opacity(&mut self, hwnd: isize, percent: u8) -> Result<(), WinError> {
        let pin = match self.live_pins.iter_mut().find(|p| p.hwnd == hwnd) {
            Some(p) => p,
            None => return Err(WinError::NotPinnable("Window is not pinned".into())),
        };

        let clamped = percent.clamp(20, 100);
        let alpha = percent_to_alpha(clamped);

        set_window_opacity(hwnd, alpha)?;

        pin.opacity = clamped;
        pin.opacity_changed = true;

        let _ = self.save_to_disk();
        self.notify_change();
        Ok(())
    }

    /// Sets or removes mouse click passthrough on a pinned window.
    pub fn set_click_through(&mut self, hwnd: isize, enabled: bool) -> Result<(), WinError> {
        let pin = match self.live_pins.iter_mut().find(|p| p.hwnd == hwnd) {
            Some(p) => p,
            None => return Err(WinError::NotPinnable("Window is not pinned".into())),
        };

        let alpha = percent_to_alpha(pin.opacity);
        set_window_click_through(hwnd, enabled, alpha)?;

        pin.click_through = enabled;

        let _ = self.save_to_disk();
        self.notify_change();
        Ok(())
    }

    /// Unpins all live pinned windows visually and removes all pending pins.
    pub fn unpin_all(&mut self) -> Result<(), WinError> {
        for pin in self.live_pins.drain(..) {
            let initial = WindowInitialStyles {
                was_topmost: pin.was_topmost,
                was_layered: pin.was_layered,
                was_click_through: pin.was_click_through,
            };
            let _ = unpin_window(
                pin.hwnd,
                &initial,
                pin.opacity_changed,
                pin.click_through,
            );
            flash_window(pin.hwnd, FlashKind::Unpin);
        }

        self.pending_pins.clear();
        let _ = self.save_to_disk();
        self.notify_change();
        Ok(())
    }

    /// Forgets a pending pin by index.
    pub fn forget_pending(&mut self, index: usize) -> bool {
        if index < self.pending_pins.len() {
            self.pending_pins.remove(index);
            let _ = self.save_to_disk();
            self.notify_change();
            true
        } else {
            false
        }
    }
}

/// Initializes PinManager on application startup:
/// 1. Loads `pinned.json`
/// 2. Restores saved pins against currently open windows
/// 3. Registers hotkey and session end dispatchers with `host.rs`
/// 4. Launches the background re-enforcement thread
pub fn init_pin_manager(app: AppHandle) {
    let mgr_arc = get_manager();

    {
        let mut mgr = mgr_arc.write().unwrap();
        mgr.app_handle = Some(app.clone());

        let state = store::load();
        mgr.settings = state.settings;

        let own_pid = win::get_current_process_id();
        let discovered = win::get_pinnable_windows(own_pid);

        // Partition saved pins into live matches and pending
        for saved in state.pins {
            let matched_window = discovered
                .iter()
                .find(|w| store::matches_pending(&saved, &w.process_name, &w.title));

            if let Some(matched) = matched_window {
                // Pin the live window
                if let Ok(initial) = win::pin_window(matched.hwnd, own_pid) {
                    let percent = alpha_to_percent(saved.opacity);
                    let opacity_changed = saved.opacity < 255;

                    if opacity_changed {
                        let _ = set_window_opacity(matched.hwnd, saved.opacity);
                    }
                    if saved.click_through {
                        let _ = set_window_click_through(matched.hwnd, true, saved.opacity);
                    }

                    // Protect user's original prior state per spec section 2.2
                    mgr.live_pins.push(LivePin {
                        hwnd: matched.hwnd,
                        title: matched.title.clone(),
                        process_name: matched.process_name.clone(),
                        opacity: percent,
                        click_through: saved.click_through,
                        was_layered: initial.was_layered && saved.was_layered,
                        was_topmost: initial.was_topmost && saved.was_topmost,
                        was_click_through: initial.was_click_through && saved.was_click_through,
                        opacity_changed,
                    });
                    continue;
                }
            }

            // Not matched or pin failed: keep as pending
            mgr.pending_pins.push(saved);
        }

        let _ = mgr.save_to_disk();
        mgr.notify_change();
    }

    // Connect hotkey dispatcher to host
    let mgr_for_hotkeys = mgr_arc.clone();
    host::set_hotkey_dispatcher(move |hotkey_id| {
        let mut mgr = mgr_for_hotkeys.write().unwrap();
        let fg = get_foreground_window();

        match hotkey_id {
            HOTKEY_PIN => {
                if let Some(hwnd) = fg {
                    let _ = mgr.toggle_pin(hwnd);
                }
            }
            HOTKEY_OPACITY_UP => {
                if let Some(hwnd) = fg
                    && let Some(pin) = mgr.live_pins.iter().find(|p| p.hwnd == hwnd) {
                        let next = (pin.opacity + 10).min(100);
                        let _ = mgr.set_opacity(hwnd, next);
                    }
            }
            HOTKEY_OPACITY_DOWN => {
                if let Some(hwnd) = fg
                    && let Some(pin) = mgr.live_pins.iter().find(|p| p.hwnd == hwnd) {
                        let next = pin.opacity.saturating_sub(10).max(20);
                        let _ = mgr.set_opacity(hwnd, next);
                    }
            }
            _ => {}
        }
    });

    // Connect session end handler to host (reboot survival vs manual quit)
    let mgr_for_session = mgr_arc.clone();
    host::set_session_end_handler(move || {
        let mgr = mgr_for_session.write().unwrap();
        // Save current pins for reboot survival
        let _ = mgr.save_to_disk();

        // Release visual topmost courtesy styles before shutdown
        for pin in &mgr.live_pins {
            let initial = WindowInitialStyles {
                was_topmost: pin.was_topmost,
                was_layered: pin.was_layered,
                was_click_through: pin.was_click_through,
            };
            let _ = unpin_window(
                pin.hwnd,
                &initial,
                pin.opacity_changed,
                pin.click_through,
            );
        }
    });

    // Launch background re-enforcement loop (every 1.5s)
    let mgr_for_loop = mgr_arc.clone();
    thread::spawn(move || {
        let own_pid = win::get_current_process_id();

        loop {
            thread::sleep(Duration::from_millis(1500));

            if is_session_ending() {
                continue;
            }

            let mut mgr = mgr_for_loop.write().unwrap();

            if mgr.live_pins.is_empty() && mgr.pending_pins.is_empty() {
                continue; // Zero CPU when idle
            }

            let mut dirty = false;

            // 1. Re-enforce live pins
            let mut surviving = Vec::new();
            for mut pin in mgr.live_pins.drain(..) {
                if !is_valid_window(pin.hwnd) {
                    // Window was closed by user
                    dirty = true;
                    continue;
                }

                // Check if topmost was displaced
                if !is_topmost(pin.hwnd) {
                    let _ = win::pin_window(pin.hwnd, own_pid);
                }

                // Check title changes
                let current_title = win::get_window_title(pin.hwnd);
                if !current_title.trim().is_empty() && current_title != pin.title {
                    pin.title = current_title;
                    dirty = true;
                }

                surviving.push(pin);
            }
            mgr.live_pins = surviving;

            // 2. Check pending pins against foreground window
            if let Some(fg_hwnd) = get_foreground_window()
                && is_pinnable(fg_hwnd, own_pid)
                    && !mgr.live_pins.iter().any(|p| p.hwnd == fg_hwnd)
                    && let Ok((proc_name, _)) = win::get_window_process_name_and_pid(fg_hwnd) {
                        let title = win::get_window_title(fg_hwnd);

                        let pending_idx = mgr
                            .pending_pins
                            .iter()
                            .position(|p| store::matches_pending(p, &proc_name, &title));

                        if let Some(idx) = pending_idx {
                            let saved = mgr.pending_pins.remove(idx);
                            if let Ok(initial) = win::pin_window(fg_hwnd, own_pid) {
                                let percent = alpha_to_percent(saved.opacity);
                                let opacity_changed = saved.opacity < 255;

                                if opacity_changed {
                                    let _ = set_window_opacity(fg_hwnd, saved.opacity);
                                }
                                if saved.click_through {
                                    let _ = set_window_click_through(fg_hwnd, true, saved.opacity);
                                }

                                mgr.live_pins.push(LivePin {
                                    hwnd: fg_hwnd,
                                    title: title.clone(),
                                    process_name: proc_name,
                                    opacity: percent,
                                    click_through: saved.click_through,
                                    was_layered: initial.was_layered && saved.was_layered,
                                    was_topmost: initial.was_topmost && saved.was_topmost,
                                    was_click_through: initial.was_click_through
                                        && saved.was_click_through,
                                    opacity_changed,
                                });

                                flash_window(fg_hwnd, FlashKind::Pin);
                                dirty = true;
                            }
                        }
                    }

            if dirty {
                let _ = mgr.save_to_disk();
                mgr.notify_change();
            }
        }
    });

    write_entry(Level::Info, "PinManager and re-enforcement engine started");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontend_item_conversion() {
        let mgr = PinManager {
            live_pins: vec![LivePin {
                hwnd: 1001,
                title: "Test Window".into(),
                process_name: "test.exe".into(),
                opacity: 80,
                click_through: false,
                was_layered: false,
                was_topmost: false,
                was_click_through: false,
                opacity_changed: true,
            }],
            pending_pins: vec![SavedPin {
                process_name: "pending.exe".into(),
                title: "Pending Doc".into(),
                opacity: 200,
                click_through: true,
                was_layered: true,
                was_topmost: true,
                was_click_through: true,
                extra: Default::default(),
            }],
            settings: Default::default(),
            app_handle: None,
        };

        let items = mgr.to_frontend_items();
        assert_eq!(items.len(), 2);
        assert!(!items[0].is_pending);
        assert_eq!(items[0].hwnd, Some(1001));
        assert!(items[1].is_pending);
        assert_eq!(items[1].hwnd, None);
    }
}
