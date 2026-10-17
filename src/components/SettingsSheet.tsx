import React from "react";
import { Settings } from "../types";
import { XIcon, ChevronRightIcon, ExternalLinkIcon } from "./icons";

interface SettingsSheetProps {
  isOpen: boolean;
  onClose: () => void;
  settings: Settings;
  onUpdateSettings: (newSettings: Partial<Settings>) => void;
  onOpenShortcuts: () => void;
}

export const SettingsSheet: React.FC<SettingsSheetProps> = ({
  isOpen,
  onClose,
  settings,
  onUpdateSettings,
  onOpenShortcuts,
}) => {
  if (!isOpen) return null;

  return (
    <div className="sheet-backdrop">
      <div className="sheet-container max-h-[85vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between pb-3 border-b border-zinc-200 dark:border-zinc-800">
          <div>
            <h2 className="text-base font-bold text-zinc-950 dark:text-zinc-50 font-heading">
              Settings
            </h2>
            <p className="text-xs text-zinc-500 dark:text-zinc-400">
              Preferences and system integration
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="p-1.5 rounded-md text-zinc-400 hover:text-black dark:hover:text-white hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors"
          >
            <XIcon size={16} />
          </button>
        </div>

        {/* Content */}
        <div className="space-y-4 my-4 flex-1 overflow-y-auto pr-0.5 custom-scrollbar">
          {/* General Section */}
          <div className="space-y-3">
            <h3 className="text-[11px] font-mono font-semibold tracking-wider text-zinc-400 uppercase">
              Behavior
            </h3>

            {/* Sound Toggle */}
            <label className="flex items-start justify-between gap-3 p-3 rounded-lg border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/40 cursor-pointer hover:border-zinc-400 dark:hover:border-zinc-700 transition-colors">
              <div>
                <div className="text-xs font-semibold text-zinc-900 dark:text-zinc-100">
                  Sound effects
                </div>
                <div className="text-[11px] text-zinc-500 dark:text-zinc-400">
                  Play audio feedback when a window is pinned or unpinned
                </div>
              </div>
              <input
                type="checkbox"
                checked={settings.enable_sound}
                onChange={(e) =>
                  onUpdateSettings({ enable_sound: e.target.checked })
                }
                className="mt-0.5 h-4 w-4 rounded-md border-zinc-300 text-black focus:ring-black accent-black dark:accent-white cursor-pointer"
              />
            </label>

            {/* Notifications Toggle */}
            <label className="flex items-start justify-between gap-3 p-3 rounded-lg border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/40 cursor-pointer hover:border-zinc-400 dark:hover:border-zinc-700 transition-colors">
              <div>
                <div className="text-xs font-semibold text-zinc-900 dark:text-zinc-100">
                  Tray notifications
                </div>
                <div className="text-[11px] text-zinc-500 dark:text-zinc-400">
                  Show notification when pinning while pinontop is minimized
                </div>
              </div>
              <input
                type="checkbox"
                checked={settings.show_notifications}
                onChange={(e) =>
                  onUpdateSettings({ show_notifications: e.target.checked })
                }
                className="mt-0.5 h-4 w-4 rounded-md border-zinc-300 text-black focus:ring-black accent-black dark:accent-white cursor-pointer"
              />
            </label>

            {/* Start with Windows */}
            <label className="flex items-start justify-between gap-3 p-3 rounded-lg border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/40 cursor-pointer hover:border-zinc-400 dark:hover:border-zinc-700 transition-colors">
              <div>
                <div className="text-xs font-semibold text-zinc-900 dark:text-zinc-100">
                  Start with Windows
                </div>
                <div className="text-[11px] text-zinc-500 dark:text-zinc-400">
                  Automatically start pinontop minimized in system tray on login
                </div>
              </div>
              <input
                type="checkbox"
                checked={settings.start_with_windows}
                onChange={(e) =>
                  onUpdateSettings({ start_with_windows: e.target.checked })
                }
                className="mt-0.5 h-4 w-4 rounded-md border-zinc-300 text-black focus:ring-black accent-black dark:accent-white cursor-pointer"
              />
            </label>
          </div>

          {/* Shortcuts Link */}
          <div className="space-y-2">
            <h3 className="text-[11px] font-mono font-semibold tracking-wider text-zinc-400 uppercase">
              Hotkeys
            </h3>
            <button
              type="button"
              onClick={() => {
                onClose();
                onOpenShortcuts();
              }}
              className="w-full flex items-center justify-between p-3 rounded-lg border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/40 hover:border-black dark:hover:border-white transition-colors"
            >
              <div className="text-left">
                <div className="text-xs font-semibold text-zinc-900 dark:text-zinc-100">
                  Configure shortcuts
                </div>
                <div className="text-[11px] text-zinc-500 dark:text-zinc-400">
                  Customize hotkey bindings for pinning and opacity
                </div>
              </div>
              <ChevronRightIcon size={16} className="text-zinc-400" />
            </button>
          </div>

          {/* About Section */}
          <div className="p-3 rounded-lg border border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900/20 text-xs space-y-1.5">
            <div className="flex items-center justify-between">
              <span className="font-semibold text-zinc-900 dark:text-zinc-100 font-heading">
                pinontop
              </span>
              <span className="font-mono text-[11px] px-1.5 py-0.5 rounded-md bg-zinc-200 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300">
                v0.1.0 (Tauri v2)
              </span>
            </div>
            <p className="text-[11px] text-zinc-500 dark:text-zinc-400">
              Keep any window permanently on top of all others with zero friction.
            </p>
            <div className="pt-2 flex items-center justify-between text-[11px] text-zinc-500 border-t border-zinc-200 dark:border-zinc-800">
              <span>License: Apache 2.0</span>
              <a
                href="https://github.com/yeasin2002/pinontop"
                target="_blank"
                rel="noreferrer"
                className="flex items-center gap-1 hover:text-black dark:hover:text-white transition-colors"
              >
                GitHub <ExternalLinkIcon size={10} />
              </a>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
