import { useState, useEffect, useCallback } from "react";
import { PinItem, Settings, WindowInfo, ToastMessage } from "../types";

// Check if running inside Tauri runtime
const isTauri =
  typeof window !== "undefined" &&
  ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

const INITIAL_SETTINGS: Settings = {
  enable_sound: true,
  show_notifications: false,
  has_seen_tray_notice: false,
  start_with_windows: false,
  shortcuts: {
    toggle_pin: "super+ctrl+KeyT",
    opacity_up: "super+ctrl+Equal",
    opacity_down: "super+ctrl+Minus",
    toggle_window: "super+ctrl+KeyP",
  },
};

const INITIAL_MOCK_WINDOWS: WindowInfo[] = [
  { hwnd: 101, title: "Visual Studio Code - pinontop", process_name: "Code.exe", is_pinned: false },
  { hwnd: 102, title: "Google Chrome - GitHub", process_name: "chrome.exe", is_pinned: false },
  { hwnd: 103, title: "Windows Terminal (pwsh)", process_name: "WindowsTerminal.exe", is_pinned: false },
  { hwnd: 104, title: "Spotify Free", process_name: "Spotify.exe", is_pinned: false },
  { hwnd: 105, title: "Task Manager", process_name: "Taskmgr.exe", is_pinned: false },
  { hwnd: 106, title: "Notepad - notes.txt", process_name: "notepad.exe", is_pinned: false },
];

export function usePins() {
  const [pins, setPins] = useState<PinItem[]>([]);
  const [settings, setSettings] = useState<Settings>(INITIAL_SETTINGS);
  const [windows, setWindows] = useState<WindowInfo[]>(INITIAL_MOCK_WINDOWS);
  const [toast, setToast] = useState<ToastMessage | null>(null);
  const [conflicts, setConflicts] = useState<string[]>([]);

  const showToast = useCallback(
    (message: string, type: ToastMessage["type"] = "info") => {
      setToast({ id: Math.random().toString(), message, type });
    },
    []
  );

  // Initialize and subscribe to Tauri events if available
  useEffect(() => {
    let unlistenPins: (() => void) | undefined;
    let unlistenSettings: (() => void) | undefined;
    let unlistenToast: (() => void) | undefined;
    let unlistenConflicts: (() => void) | undefined;

    if (isTauri) {
      import("@tauri-apps/api/core").then(({ invoke }) => {
        invoke<any>("get_state")
          .then((state) => {
            if (state) {
              if (state.pins) setPins(state.pins);
              if (state.settings) setSettings(state.settings);
            }
          })
          .catch(() => {
            // Backend might still be setting up handlers
          });
      });

      import("@tauri-apps/api/event").then(({ listen }) => {
        listen<PinItem[]>("pins-changed", (event) => {
          setPins(event.payload);
        }).then((fn) => {
          unlistenPins = fn;
        });

        listen<Settings>("settings-changed", (event) => {
          setSettings(event.payload);
        }).then((fn) => {
          unlistenSettings = fn;
        });

        listen<{ message: string }>("toast", (event) => {
          showToast(event.payload.message);
        }).then((fn) => {
          unlistenToast = fn;
        });

        listen<{ conflicts: string[] }>("shortcut-conflicts", (event) => {
          setConflicts(event.payload.conflicts || []);
        }).then((fn) => {
          unlistenConflicts = fn;
        });
      });
    }

    return () => {
      unlistenPins?.();
      unlistenSettings?.();
      unlistenToast?.();
      unlistenConflicts?.();
    };
  }, [showToast]);

  // Load open windows for picker
  const fetchWindows = useCallback(async () => {
    if (isTauri) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const list = await invoke<WindowInfo[]>("get_windows");
        setWindows(list);
      } catch (err) {
        console.error("Failed to get windows:", err);
      }
    } else {
      // Refresh mock state with current pin statuses
      setWindows((prev) =>
        prev.map((w) => ({
          ...w,
          is_pinned: pins.some((p) => p.hwnd === w.hwnd),
        }))
      );
    }
  }, [pins]);

  // Toggle Pin
  const togglePin = useCallback(
    async (hwnd: number) => {
      if (isTauri) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("toggle_pin", { hwnd });
        } catch (err) {
          showToast(`Error: ${err}`, "error");
        }
      } else {
        // Mock toggle
        const targetWin = windows.find((w) => w.hwnd === hwnd);
        if (!targetWin) return;

        const isCurrentlyPinned = pins.some((p) => p.hwnd === hwnd);
        if (isCurrentlyPinned) {
          setPins((prev) => prev.filter((p) => p.hwnd !== hwnd));
          setWindows((prev) =>
            prev.map((w) => (w.hwnd === hwnd ? { ...w, is_pinned: false } : w))
          );
          showToast(`Unpinned ${targetWin.process_name}`);
        } else {
          const newPin: PinItem = {
            hwnd,
            title: targetWin.title,
            process_name: targetWin.process_name,
            opacity: 100,
            click_through: false,
          };
          setPins((prev) => [...prev, newPin]);
          setWindows((prev) =>
            prev.map((w) => (w.hwnd === hwnd ? { ...w, is_pinned: true } : w))
          );
          showToast(`Pinned ${targetWin.process_name}`);
        }
      }
    },
    [windows, pins, showToast]
  );

  // Set Opacity
  const setOpacity = useCallback(
    async (index: number, percent: number) => {
      const pin = pins[index];
      if (!pin) return;

      // Optimistic update
      setPins((prev) =>
        prev.map((p, i) => (i === index ? { ...p, opacity: percent } : p))
      );

      if (isTauri && pin.hwnd) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("set_opacity", { hwnd: pin.hwnd, percent });
        } catch (err) {
          console.error("Failed to set opacity:", err);
        }
      }
    },
    [pins]
  );

  // Toggle Click-Through
  const toggleClickThrough = useCallback(
    async (index: number) => {
      const pin = pins[index];
      if (!pin) return;

      const nextVal = !pin.click_through;

      // Optimistic update
      setPins((prev) =>
        prev.map((p, i) => (i === index ? { ...p, click_through: nextVal } : p))
      );

      showToast(
        nextVal
          ? "Clicks now pass through this window"
          : "Clicks reach this window again"
      );

      if (isTauri && pin.hwnd) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("set_click_through", { hwnd: pin.hwnd, enabled: nextVal });
        } catch (err) {
          console.error("Failed to set click through:", err);
        }
      }
    },
    [pins, showToast]
  );

  // Unpin single
  const unpin = useCallback(
    async (index: number) => {
      const pin = pins[index];
      if (!pin) return;

      if (pin.hwnd) {
        togglePin(pin.hwnd);
      } else {
        setPins((prev) => prev.filter((_, i) => i !== index));
        showToast(`Removed ${pin.process_name}`);
      }
    },
    [pins, togglePin, showToast]
  );

  // Unpin all
  const unpinAll = useCallback(async () => {
    if (isTauri) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("unpin_all");
      } catch (err) {
        console.error("Failed to unpin all:", err);
      }
    } else {
      setPins([]);
      setWindows((prev) => prev.map((w) => ({ ...w, is_pinned: false })));
      showToast("All windows unpinned");
    }
  }, [showToast]);

  // Save Settings
  const updateSettings = useCallback(
    async (newSettings: Partial<Settings>) => {
      const updated = { ...settings, ...newSettings };
      setSettings(updated);

      if (isTauri) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("save_settings", { settings: updated });
          if (newSettings.start_with_windows !== undefined) {
            await invoke("set_autostart", {
              enabled: newSettings.start_with_windows,
            });
          }
        } catch (err) {
          console.error("Failed to save settings:", err);
        }
      }
    },
    [settings]
  );

  // Save Shortcuts
  const saveShortcuts = useCallback(
    async (newShortcuts: Settings["shortcuts"]) => {
      setSettings((prev) => ({ ...prev, shortcuts: newShortcuts }));
      showToast("Shortcuts updated successfully");

      if (isTauri) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("save_shortcuts", { shortcuts: newShortcuts });
        } catch (err) {
          showToast(`Error saving shortcuts: ${err}`, "error");
        }
      }
    },
    [showToast]
  );

  // Hover row for overlay highlight
  const hoverRow = useCallback((hwnd: number | undefined, hovered: boolean) => {
    if (isTauri && hwnd) {
      import("@tauri-apps/api/core").then(({ invoke }) => {
        invoke("row_hovered", { hwnd, hovered }).catch(() => {});
      });
    }
  }, []);

  return {
    pins,
    settings,
    windows,
    toast,
    conflicts,
    dismissToast: () => setToast(null),
    showToast,
    fetchWindows,
    togglePin,
    setOpacity,
    toggleClickThrough,
    unpin,
    unpinAll,
    updateSettings,
    saveShortcuts,
    hoverRow,
  };
}
