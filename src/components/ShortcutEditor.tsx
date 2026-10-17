import React, { useState } from "react";
import { Shortcuts } from "../types";
import { XIcon, RefreshCwIcon, AlertTriangleIcon } from "./icons";

interface ShortcutEditorProps {
  isOpen: boolean;
  onClose: () => void;
  shortcuts: Shortcuts;
  onSave: (newShortcuts: Shortcuts) => void;
}

const DEFAULT_SHORTCUTS: Shortcuts = {
  toggle_pin: "super+ctrl+KeyT",
  opacity_up: "super+ctrl+Equal",
  opacity_down: "super+ctrl+Minus",
  toggle_window: "super+ctrl+KeyP",
};

interface ParsedShortcut {
  super: boolean;
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  key: string;
}

function parseShortcutString(s: string): ParsedShortcut {
  const parts = s.split("+");
  const parsed: ParsedShortcut = {
    super: false,
    ctrl: false,
    alt: false,
    shift: false,
    key: "KeyT",
  };

  for (const p of parts) {
    const l = p.toLowerCase();
    if (l === "super" || l === "win" || l === "meta") parsed.super = true;
    else if (l === "ctrl" || l === "control") parsed.ctrl = true;
    else if (l === "alt") parsed.alt = true;
    else if (l === "shift") parsed.shift = true;
    else parsed.key = p;
  }

  return parsed;
}

function buildShortcutString(parsed: ParsedShortcut): string {
  const parts: string[] = [];
  if (parsed.super) parts.push("super");
  if (parsed.ctrl) parts.push("ctrl");
  if (parsed.alt) parts.push("alt");
  if (parsed.shift) parts.push("shift");
  parts.push(parsed.key);
  return parts.join("+");
}

const AVAILABLE_KEYS = [
  { label: "T", value: "KeyT" },
  { label: "P", value: "KeyP" },
  { label: "A", value: "KeyA" },
  { label: "B", value: "KeyB" },
  { label: "O", value: "KeyO" },
  { label: "W", value: "KeyW" },
  { label: "= / +", value: "Equal" },
  { label: "- / _", value: "Minus" },
  { label: "[", value: "BracketLeft" },
  { label: "]", value: "BracketRight" },
  { label: "F1", value: "F1" },
  { label: "F2", value: "F2" },
  { label: "F3", value: "F3" },
  { label: "F4", value: "F4" },
  { label: "F9", value: "F9" },
  { label: "F10", value: "F10" },
  { label: "F11", value: "F11" },
  { label: "F12", value: "F12" },
  { label: "Space", value: "Space" },
];

export const ShortcutEditor: React.FC<ShortcutEditorProps> = ({
  isOpen,
  onClose,
  shortcuts,
  onSave,
}) => {
  const [editing, setEditing] = useState<Shortcuts>(shortcuts);

  if (!isOpen) return null;

  const handleModifierToggle = (
    action: keyof Shortcuts,
    mod: "super" | "ctrl" | "alt" | "shift"
  ) => {
    const parsed = parseShortcutString(editing[action]);
    parsed[mod] = !parsed[mod];
    setEditing({ ...editing, [action]: buildShortcutString(parsed) });
  };

  const handleKeyChange = (action: keyof Shortcuts, key: string) => {
    const parsed = parseShortcutString(editing[action]);
    parsed.key = key;
    setEditing({ ...editing, [action]: buildShortcutString(parsed) });
  };

  // Safe modifier rule: Must include at least one of Win, Ctrl, or Alt
  const isValid = (shortcutStr: string) => {
    const p = parseShortcutString(shortcutStr);
    return p.super || p.ctrl || p.alt;
  };

  const allValid = Object.values(editing).every(isValid);

  const actions: { id: keyof Shortcuts; title: string }[] = [
    { id: "toggle_pin", title: "Pin / unpin window" },
    { id: "opacity_up", title: "Increase opacity (+5%)" },
    { id: "opacity_down", title: "Decrease opacity (-5%)" },
    { id: "toggle_window", title: "Show / hide pinontop" },
  ];

  return (
    <div className="sheet-backdrop">
      <div className="sheet-container max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between pb-3 border-b border-zinc-200 dark:border-zinc-800">
          <div>
            <h2 className="text-base font-bold text-zinc-950 dark:text-zinc-50 font-heading">
              Keyboard Shortcuts
            </h2>
            <p className="text-xs text-zinc-500 dark:text-zinc-400">
              Customize global hotkeys for instant actions
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

        {/* Action Rows */}
        <div className="space-y-3.5 my-4 flex-1 overflow-y-auto pr-0.5 custom-scrollbar">
          {actions.map(({ id, title }) => {
            const parsed = parseShortcutString(editing[id]);
            const valid = isValid(editing[id]);

            return (
              <div
                key={id}
                className="p-3 rounded-lg border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/40"
              >
                <div className="flex items-center justify-between mb-2">
                  <span className="text-xs font-semibold text-zinc-900 dark:text-zinc-100">
                    {title}
                  </span>
                  {!valid && (
                    <span className="flex items-center gap-1 text-[10px] font-mono text-red-600 dark:text-red-400">
                      <AlertTriangleIcon size={12} />
                      Must use Win, Ctrl, or Alt
                    </span>
                  )}
                </div>

                <div className="flex items-center gap-1.5 flex-wrap">
                  {(["super", "ctrl", "alt", "shift"] as const).map((mod) => {
                    const active = parsed[mod];
                    const label =
                      mod === "super"
                        ? "Win"
                        : mod.charAt(0).toUpperCase() + mod.slice(1);
                    return (
                      <button
                        key={mod}
                        type="button"
                        onClick={() => handleModifierToggle(id, mod)}
                        className={`px-2.5 py-1 text-xs font-mono font-medium rounded-md border transition-all ${
                          active
                            ? "bg-black text-white dark:bg-white dark:text-black border-black dark:border-white shadow-sm"
                            : "bg-zinc-50 dark:bg-zinc-800/60 text-zinc-500 dark:text-zinc-400 border-zinc-200 dark:border-zinc-700 hover:border-zinc-400 dark:hover:border-zinc-500"
                        }`}
                      >
                        {label}
                      </button>
                    );
                  })}

                  <span className="text-xs text-zinc-400 font-mono px-1">+</span>

                  {/* Key selector */}
                  <select
                    value={parsed.key}
                    onChange={(e) => handleKeyChange(id, e.target.value)}
                    className="px-2 py-1 text-xs font-mono font-semibold rounded-md border border-zinc-200 dark:border-zinc-700 bg-zinc-100 dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 focus:outline-none focus:border-black dark:focus:border-white"
                  >
                    {AVAILABLE_KEYS.map((k) => (
                      <option key={k.value} value={k.value}>
                        {k.label}
                      </option>
                    ))}
                  </select>
                </div>
              </div>
            );
          })}
        </div>

        {/* Footer */}
        <div className="pt-3 border-t border-zinc-200 dark:border-zinc-800 flex items-center justify-between gap-2">
          <button
            type="button"
            onClick={() => setEditing(DEFAULT_SHORTCUTS)}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-zinc-600 dark:text-zinc-400 hover:text-black dark:hover:text-white rounded-md border border-zinc-200 dark:border-zinc-800 hover:border-zinc-400 transition-colors"
          >
            <RefreshCwIcon size={12} />
            Restore defaults
          </button>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-1.5 text-xs font-medium text-zinc-600 dark:text-zinc-400 hover:text-black dark:hover:text-white rounded-md transition-colors"
            >
              Cancel
            </button>
            <button
              type="button"
              disabled={!allValid}
              onClick={() => {
                if (allValid) {
                  onSave(editing);
                  onClose();
                }
              }}
              className={`px-4 py-1.5 text-xs font-semibold rounded-md transition-all ${
                allValid
                  ? "bg-black text-white dark:bg-white dark:text-black hover:bg-zinc-800 dark:hover:bg-zinc-200"
                  : "bg-zinc-200 dark:bg-zinc-800 text-zinc-400 cursor-not-allowed"
              }`}
            >
              Save shortcuts
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
