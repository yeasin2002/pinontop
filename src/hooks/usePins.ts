import { useState, useEffect, useCallback, useRef } from "react";
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

// Lightweight synthesized sound effects using Web Audio API
let audioCtx: AudioContext | null = null;
function getAudioContext(): AudioContext | null {
  if (typeof window === "undefined") return null;
  const AudioContextClass = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
  if (!AudioContextClass) return null;
  if (!audioCtx || audioCtx.state === "closed") {
    audioCtx = new AudioContextClass();
  }
  if (audioCtx.state === "suspended") {
    audioCtx.resume().catch(() => {});
  }
  return audioCtx;
}

function playPinSound() {
  try {
    const ctx = getAudioContext();
    if (!ctx) return;
    const now = ctx.currentTime;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = "sine";
    osc.frequency.setValueAtTime(520, now);
    osc.frequency.exponentialRampToValueAtTime(780, now + 0.04);

    gain.gain.setValueAtTime(0.12, now);
    gain.gain.exponentialRampToValueAtTime(0.001, now + 0.05);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(now);
    osc.stop(now + 0.05);
  } catch {
    // Audio context may be restricted before user interaction
  }
}

function playUnpinSound() {
  try {
    const ctx = getAudioContext();
    if (!ctx) return;
    const now = ctx.currentTime;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = "sine";
    osc.frequency.setValueAtTime(620, now);
    osc.frequency.exponentialRampToValueAtTime(420, now + 0.04);

    gain.gain.setValueAtTime(0.10, now);
    gain.gain.exponentialRampToValueAtTime(0.001, now + 0.045);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(now);
    osc.stop(now + 0.045);
  } catch {
    // Audio context may be restricted
  }
}

export function usePins() {
  const [pins, setPins] = useState<PinItem[]>([]);
  const [settings, setSettings] = useState<Settings>(INITIAL_SETTINGS);
  const [windows, setWindows] = useState<WindowInfo[]>(INITIAL_MOCK_WINDOWS);
  const [toast, setToast] = useState<ToastMessage | null>(null);
  const [conflicts, setConflicts] = useState<string[]>([]);

  // Ref to always access latest settings inside callbacks without stale closures
  const settingsRef = useRef(settings);
  useEffect(() => {
    settingsRef.current = settings;
  }, [settings]);

  const showToast = useCallback(
    (message: string, type: ToastMessage["type"] = "info") => {
      setToast({ id: Math.random().toString(), message, type });
    },
    []
  );

  // Initialize and subscribe to Tauri events
  useEffect(() => {
    let unlistenPins: (() => void) | undefined;
    let unlistenSettings: (() => void) | undefined;
    let unlistenToast: (() => void) | undefined;
    let unlistenConflicts: (() => void) | undefined;

    if (isTauri) {
      import("@tauri-apps/api/core").then(({ invoke }) => {
        invoke<{ pins: PinItem[]; settings: Settings }>("get_state")
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
          // Sync open windows is_pinned status
          setWindows((prev) =>
            prev.map((w) => ({
              ...w,
              is_pinned: event.payload.some((p) => p.hwnd === w.hwnd),
            }))
          );
        }).then((fn) => {
          unlistenPins = fn;
        });

        listen<Settings>("settings-changed", (event) => {
          setSettings(event.payload);
        }).then((fn) => {
          unlistenSettings = fn;
        });

        listen<string | { message: string }>("toast", (event) => {
          const msg =
            typeof event.payload === "string"
              ? event.payload
              : event.payload?.message;
          if (msg) showToast(msg);
        }).then((fn) => {
          unlistenToast = fn;
        });

        listen<{ conflicts?: string[] } | string[]>("shortcut-conflicts", (event) => {
          if (Array.isArray(event.payload)) {
            setConflicts(event.payload);
          } else if (event.payload?.conflicts) {
            setConflicts(event.payload.conflicts);
          } else {
            setConflicts([]);
          }
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

  // Toggle Pin with optimistic update, rollback on error, and sound feedback
  const togglePin = useCallback(
    async (hwnd: number) => {
      const targetWin = windows.find((w) => w.hwnd === hwnd);
      const isCurrentlyPinned = pins.some((p) => p.hwnd === hwnd);
      const prevPins = [...pins];
      const prevWindows = [...windows];

      // Optimistic update
      if (isCurrentlyPinned) {
        setPins((prev) => prev.filter((p) => p.hwnd !== hwnd));
        setWindows((prev) =>
          prev.map((w) => (w.hwnd === hwnd ? { ...w, is_pinned: false } : w))
        );
      } else if (targetWin) {
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
      }

      if (isTauri) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          const pinned = await invoke<boolean>("toggle_pin", { hwnd });

          // Play sound on success
          if (settingsRef.current.enable_sound) {
            if (pinned) {
              playPinSound();
            } else {
              playUnpinSound();
            }
          }

          if (targetWin) {
            showToast(
              pinned
                ? `Pinned ${targetWin.process_name}`
                : `Unpinned ${targetWin.process_name}`
            );
          }
        } catch (err) {
          // Rollback on failure
          setPins(prevPins);
          setWindows(prevWindows);
          showToast(`Pin failed: ${err}`, "error");
        }
      } else {
        // Mock feedback
        if (settingsRef.current.enable_sound) {
          if (isCurrentlyPinned) playUnpinSound();
          else playPinSound();
        }
        if (targetWin) {
          showToast(
            isCurrentlyPinned
              ? `Unpinned ${targetWin.process_name}`
              : `Pinned ${targetWin.process_name}`
          );
        }
      }
    },
    [windows, pins, showToast]
  );

  // Set Opacity with optimistic update and rollback
  const setOpacity = useCallback(
    async (index: number, percent: number) => {
      const pin = pins[index];
      if (!pin) return;

      const prevPins = [...pins];
      // Optimistic update
      setPins((prev) =>
        prev.map((p, i) => (i === index ? { ...p, opacity: percent } : p))
      );

      if (isTauri && pin.hwnd) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("set_opacity", { hwnd: pin.hwnd, percent });
        } catch (err) {
          setPins(prevPins);
          showToast(`Failed to set opacity: ${err}`, "error");
        }
      }
    },
    [pins, showToast]
  );

  // Toggle Click-Through with optimistic update and rollback
  const toggleClickThrough = useCallback(
    async (index: number) => {
      const pin = pins[index];
      if (!pin) return;

      const nextVal = !pin.click_through;
      const prevPins = [...pins];

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
          setPins(prevPins);
          showToast(`Failed to toggle click-through: ${err}`, "error");
        }
      }
    },
    [pins, showToast]
  );

  // Unpin single (live window or forget pending pin)
  const unpin = useCallback(
    async (index: number) => {
      const pin = pins[index];
      if (!pin) return;

      if (pin.hwnd) {
        await togglePin(pin.hwnd);
      } else {
        // Pending pin: remove from state and notify backend
        const prevPins = [...pins];
        setPins((prev) => prev.filter((_, i) => i !== index));

        if (isTauri) {
          try {
            const { invoke } = await import("@tauri-apps/api/core");
            await invoke("forget_pending", { index });
            if (settingsRef.current.enable_sound) {
              playUnpinSound();
            }
            showToast(`Removed pending pin for ${pin.process_name}`);
          } catch (err) {
            setPins(prevPins);
            showToast(`Error removing pending pin: ${err}`, "error");
          }
        } else {
          if (settingsRef.current.enable_sound) {
            playUnpinSound();
          }
          showToast(`Removed pending pin for ${pin.process_name}`);
        }
      }
    },
    [pins, togglePin, showToast]
  );

  // Unpin all
  const unpinAll = useCallback(async () => {
    const prevPins = [...pins];
    const prevWindows = [...windows];

    setPins([]);
    setWindows((prev) => prev.map((w) => ({ ...w, is_pinned: false })));

    if (isTauri) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("unpin_all");
        if (settingsRef.current.enable_sound) {
          playUnpinSound();
        }
        showToast("All windows unpinned");
      } catch (err) {
        setPins(prevPins);
        setWindows(prevWindows);
        showToast(`Failed to unpin all: ${err}`, "error");
      }
    } else {
      if (settingsRef.current.enable_sound) {
        playUnpinSound();
      }
      showToast("All windows unpinned");
    }
  }, [pins, windows, showToast]);

  // Save Settings with optimistic update and rollback
  const updateSettings = useCallback(
    async (newSettings: Partial<Settings>) => {
      const prevSettings = { ...settings };
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
          setSettings(prevSettings);
          showToast(`Failed to save settings: ${err}`, "error");
        }
      }
    },
    [settings, showToast]
  );

  // Save Shortcuts with validation and rollback
  const saveShortcuts = useCallback(
    async (newShortcuts: Settings["shortcuts"]) => {
      const prevShortcuts = { ...settings.shortcuts };
      setSettings((prev) => ({ ...prev, shortcuts: newShortcuts }));

      if (isTauri) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("save_shortcuts", { shortcuts: newShortcuts });
          showToast("Shortcuts updated successfully");
        } catch (err) {
          setSettings((prev) => ({ ...prev, shortcuts: prevShortcuts }));
          showToast(`Error saving shortcuts: ${err}`, "error");
        }
      } else {
        showToast("Shortcuts updated successfully");
      }
    },
    [settings.shortcuts, showToast]
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
