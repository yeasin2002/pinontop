import React, { useState, useMemo } from "react";
import { WindowInfo } from "../types";
import { AppAvatar } from "./AppAvatar";
import { SearchIcon, XIcon, CheckIcon } from "./icons";

interface WindowPickerProps {
  isOpen: boolean;
  onClose: () => void;
  windows: WindowInfo[];
  onTogglePin: (hwnd: number) => void;
}

export const WindowPicker: React.FC<WindowPickerProps> = ({
  isOpen,
  onClose,
  windows,
  onTogglePin,
}) => {
  const [search, setSearch] = useState("");

  const filteredWindows = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return windows;
    return windows.filter(
      (w) =>
        w.title.toLowerCase().includes(q) ||
        w.process_name.toLowerCase().includes(q)
    );
  }, [windows, search]);

  if (!isOpen) return null;

  return (
    <div className="sheet-backdrop">
      <div className="sheet-container max-h-[85vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between pb-3 border-b border-zinc-200 dark:border-zinc-800">
          <div>
            <h2 className="text-base font-bold text-zinc-950 dark:text-zinc-50 font-heading">
              Pin a window
            </h2>
            <p className="text-xs text-zinc-500 dark:text-zinc-400">
              Select an open window to keep always on top
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

        {/* Search */}
        <div className="relative my-3">
          <SearchIcon
            size={14}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-zinc-400"
          />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search windows or apps..."
            autoFocus
            className="w-full pl-9 pr-8 py-2 rounded-md border border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900 text-xs text-zinc-900 dark:text-zinc-100 placeholder:text-zinc-400 focus:outline-none focus:border-black dark:focus:border-white transition-colors"
          />
          {search && (
            <button
              type="button"
              onClick={() => setSearch("")}
              className="absolute right-2.5 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-black dark:hover:text-white"
            >
              <XIcon size={12} />
            </button>
          )}
        </div>

        {/* Windows list */}
        <div className="flex-1 overflow-y-auto space-y-1.5 pr-0.5 custom-scrollbar min-h-[180px] max-h-[300px]">
          {filteredWindows.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-10 text-center text-xs text-zinc-400">
              {search ? (
                <>No matches for "{search}"</>
              ) : (
                <>No open windows detected</>
              )}
            </div>
          ) : (
            filteredWindows.map((win) => (
              <button
                key={win.hwnd}
                type="button"
                onClick={() => onTogglePin(win.hwnd)}
                className={`w-full flex items-center gap-3 p-2.5 rounded-md border text-left transition-all ${
                  win.is_pinned
                    ? "bg-zinc-100 dark:bg-zinc-800/80 border-black dark:border-white"
                    : "bg-white dark:bg-zinc-900/40 border-zinc-200 dark:border-zinc-800 hover:border-zinc-400 dark:hover:border-zinc-600"
                }`}
              >
                <AppAvatar processName={win.process_name} size={28} />
                <div className="flex-1 min-w-0">
                  <div className="text-xs font-semibold text-zinc-900 dark:text-zinc-100 truncate">
                    {win.title}
                  </div>
                  <div className="text-[11px] font-mono text-zinc-500 dark:text-zinc-400 truncate">
                    {win.process_name}
                  </div>
                </div>
                {win.is_pinned && (
                  <span className="flex items-center gap-1 text-[11px] font-mono font-medium px-2 py-0.5 rounded-md bg-black text-white dark:bg-white dark:text-black">
                    <CheckIcon size={11} />
                    Pinned
                  </span>
                )}
              </button>
            ))
          )}
        </div>
      </div>
    </div>
  );
};
