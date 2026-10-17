import { useState } from "react";
import { usePins } from "./hooks/usePins";
import { useTheme } from "./hooks/useTheme";
import { SheetView } from "./types";
import { PinList } from "./components/PinList";
import { WindowPicker } from "./components/WindowPicker";
import { ShortcutEditor } from "./components/ShortcutEditor";
import { SettingsSheet } from "./components/SettingsSheet";
import { Toast } from "./components/Toast";
import { KeyBadge } from "./components/KeyBadge";
import {
  SettingsIcon,
  SunIcon,
  MoonIcon,
  PlusIcon,
  AlertTriangleIcon,
} from "./components/icons";
import "./index.css";
import "./App.css";

function App() {
  const { theme, toggleTheme } = useTheme();
  const {
    pins,
    settings,
    windows,
    toast,
    conflicts,
    dismissToast,
    fetchWindows,
    togglePin,
    setOpacity,
    toggleClickThrough,
    unpin,
    unpinAll,
    updateSettings,
    saveShortcuts,
    hoverRow,
  } = usePins();

  const [activeSheet, setActiveSheet] = useState<SheetView>("none");

  const handleOpenPicker = () => {
    fetchWindows();
    setActiveSheet("picker");
  };

  return (
    <div className="app-container">
          {/* Header */}
          <header className="flex items-center justify-between pb-1">
            <div className="flex items-center gap-2.5">
              <img
                src="/app-icon.png"
                alt="pinontop icon"
                className="w-8 h-8 rounded-md shadow-sm border border-zinc-200 dark:border-zinc-800 object-cover"
              />
              <div>
                <h1 className="text-base font-bold tracking-tight text-zinc-950 dark:text-zinc-50 font-heading leading-tight">
                  pinontop
                </h1>
                <p className="text-xs text-zinc-500 dark:text-zinc-400">
                  Keep any window always on top
                </p>
              </div>
            </div>

            {/* Quick Actions (Theme & Settings) */}
            <div className="flex items-center gap-1">
              <button
                type="button"
                onClick={toggleTheme}
                title={`Switch to ${theme === "dark" ? "Light" : "Dark"} mode`}
                className="p-1.5 rounded-md text-zinc-500 hover:text-black dark:hover:text-white hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors"
                aria-label="Toggle theme"
              >
                {theme === "dark" ? <SunIcon size={16} /> : <MoonIcon size={16} />}
              </button>
              <button
                type="button"
                onClick={() => setActiveSheet("settings")}
                title="Open Settings"
                className="p-1.5 rounded-md text-zinc-500 hover:text-black dark:hover:text-white hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors"
                aria-label="Settings"
              >
                <SettingsIcon size={16} />
              </button>
            </div>
          </header>

          {/* Conflict Alert Banner */}
          {conflicts.length > 0 && (
            <div
              onClick={() => setActiveSheet("shortcuts")}
              className="flex items-center gap-2 p-2.5 rounded-md border border-amber-300 dark:border-amber-700/60 bg-amber-50 dark:bg-amber-950/30 text-amber-900 dark:text-amber-200 text-xs cursor-pointer hover:opacity-90 transition-opacity"
            >
              <AlertTriangleIcon size={14} className="flex-shrink-0 text-amber-600 dark:text-amber-400" />
              <span className="flex-1 font-medium">
                Some shortcuts are unavailable. Click to reassign.
              </span>
            </div>
          )}

          {/* Primary Action Button */}
          <button
            type="button"
            onClick={handleOpenPicker}
            className="w-full flex items-center justify-center gap-2 py-2 px-4 rounded-md bg-black text-white dark:bg-white dark:text-black hover:bg-zinc-800 dark:hover:bg-zinc-200 font-sans text-xs font-semibold shadow-sm transition-all duration-150 active:scale-[0.99]"
          >
            <PlusIcon size={15} />
            Pin a window...
          </button>

          {/* SHORTCUTS Reference Card */}
          <section className="flex flex-col gap-2">
            <h2 className="text-[11px] font-mono font-semibold tracking-wider text-zinc-500 dark:text-zinc-400 uppercase">
              Shortcuts
            </h2>
            <div className="p-3 rounded-lg border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/40 flex flex-col gap-2.5">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-1">
                  <KeyBadge
                    keys={settings.shortcuts.toggle_pin.split("+")}
                    size="sm"
                  />
                </div>
                <span className="text-xs text-zinc-600 dark:text-zinc-300 font-medium text-right">
                  Pin / unpin window
                </span>
              </div>

              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-1">
                  <KeyBadge
                    keys={[
                      ...settings.shortcuts.opacity_up.split("+"),
                      "/",
                      settings.shortcuts.opacity_down.split("+").slice(-1)[0],
                    ]}
                    size="sm"
                  />
                </div>
                <span className="text-xs text-zinc-600 dark:text-zinc-300 font-medium text-right">
                  Adjust opacity
                </span>
              </div>

              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-1">
                  <KeyBadge
                    keys={settings.shortcuts.toggle_window.split("+")}
                    size="sm"
                  />
                </div>
                <span className="text-xs text-zinc-600 dark:text-zinc-300 font-medium text-right">
                  Show / hide pinontop
                </span>
              </div>
            </div>

            <button
              type="button"
              onClick={() => setActiveSheet("shortcuts")}
              className="self-start py-1 px-2.5 rounded-md border border-zinc-200 dark:border-zinc-800 text-[11px] font-medium text-zinc-700 dark:text-zinc-300 hover:border-black dark:hover:border-white transition-colors"
            >
              Edit shortcuts...
            </button>
          </section>

          {/* PINNED Windows Section */}
          <PinList
            pins={pins}
            onOpacityChange={setOpacity}
            onClickThroughToggle={toggleClickThrough}
            onUnpin={unpin}
            onUnpinAll={unpinAll}
            onHoverRow={hoverRow}
          />

          {/* Quick Settings Footer */}
          <footer className="pt-2 border-t border-zinc-100 dark:border-zinc-800 flex flex-col gap-2">
            <label className="flex items-center gap-2.5 cursor-pointer text-xs text-zinc-700 dark:text-zinc-300 select-none">
              <input
                type="checkbox"
                checked={settings.enable_sound}
                onChange={(e) =>
                  updateSettings({ enable_sound: e.target.checked })
                }
                className="h-3.5 w-3.5 rounded border-zinc-300 text-black focus:ring-black accent-black dark:accent-white cursor-pointer"
              />
              <span>Play a sound when pinning</span>
            </label>

            <label className="flex items-center gap-2.5 cursor-pointer text-xs text-zinc-700 dark:text-zinc-300 select-none">
              <input
                type="checkbox"
                checked={settings.start_with_windows}
                onChange={(e) =>
                  updateSettings({ start_with_windows: e.target.checked })
                }
                className="h-3.5 w-3.5 rounded border-zinc-300 text-black focus:ring-black accent-black dark:accent-white cursor-pointer"
              />
              <span>Start pinontop with Windows</span>
            </label>
          </footer>

        {/* Slide-Up Sheets */}
        <WindowPicker
          isOpen={activeSheet === "picker"}
          onClose={() => setActiveSheet("none")}
          windows={windows}
          onTogglePin={togglePin}
        />

        <ShortcutEditor
          isOpen={activeSheet === "shortcuts"}
          onClose={() => setActiveSheet("none")}
          shortcuts={settings.shortcuts}
          onSave={saveShortcuts}
        />

        <SettingsSheet
          isOpen={activeSheet === "settings"}
          onClose={() => setActiveSheet("none")}
          settings={settings}
          onUpdateSettings={updateSettings}
          onOpenShortcuts={() => setActiveSheet("shortcuts")}
        />

        {/* Transient In-App Toast */}
        <Toast toast={toast} onDismiss={dismissToast} />
    </div>
  );
}

export default App;
