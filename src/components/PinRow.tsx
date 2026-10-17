import React from "react";
import { PinItem } from "../types";
import { AppAvatar } from "./AppAvatar";
import { PointerIcon, XIcon } from "./icons";

interface PinRowProps {
  pin: PinItem;
  onOpacityChange: (percent: number) => void;
  onClickThroughToggle: () => void;
  onUnpin: () => void;
  onHover?: (hovered: boolean) => void;
}

export const PinRow: React.FC<PinRowProps> = ({
  pin,
  onOpacityChange,
  onClickThroughToggle,
  onUnpin,
  onHover,
}) => {
  const isPending = Boolean(pin.is_pending);

  return (
    <div
      onMouseEnter={() => onHover?.(true)}
      onMouseLeave={() => onHover?.(false)}
      className="group relative flex flex-col gap-2 p-3 rounded-lg border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 hover:border-black dark:hover:border-zinc-500 transition-all duration-150"
    >
      <div className="flex items-center gap-3">
        {/* Avatar */}
        <AppAvatar processName={pin.process_name} isPending={isPending} />

        {/* Title and Process */}
        <div className="flex-1 min-w-0 pr-1">
          <div
            className="text-sm font-semibold text-zinc-950 dark:text-zinc-50 truncate tracking-tight"
            title={pin.title}
          >
            {pin.title || pin.process_name}
          </div>
          <div className="text-xs text-zinc-500 dark:text-zinc-400 font-mono truncate">
            {isPending ? `Waiting for ${pin.process_name}...` : pin.process_name}
          </div>
        </div>

        {/* Action Buttons: Click-through & Unpin */}
        <div className="flex items-center gap-1.5 flex-shrink-0">
          {!isPending && (
            <button
              type="button"
              onClick={onClickThroughToggle}
              title={
                pin.click_through
                  ? "Clicks pass through (click to disable)"
                  : "Enable click-through"
              }
              className={`p-1.5 rounded-md border text-xs transition-colors ${
                pin.click_through
                  ? "bg-black text-white dark:bg-white dark:text-black border-black dark:border-white"
                  : "bg-transparent text-zinc-500 hover:text-black dark:hover:text-white border-transparent hover:border-zinc-200 dark:hover:border-zinc-700"
              }`}
            >
              <PointerIcon size={14} />
            </button>
          )}

          <button
            type="button"
            onClick={onUnpin}
            title={isPending ? "Remove pending pin" : "Unpin window"}
            className="p-1.5 rounded-md text-zinc-400 hover:text-red-600 dark:hover:text-red-400 hover:bg-zinc-100 dark:hover:bg-zinc-800/80 transition-colors"
          >
            <XIcon size={14} />
          </button>
        </div>
      </div>

      {/* Opacity Control for live pins */}
      {!isPending && (
        <div className="flex items-center gap-2 pt-1.5 border-t border-zinc-100 dark:border-zinc-800/60 text-xs">
          <span className="text-[11px] font-mono text-zinc-400 dark:text-zinc-500">
            Opacity
          </span>
          <input
            type="range"
            min="20"
            max="100"
            step="5"
            value={pin.opacity}
            onChange={(e) => onOpacityChange(Number(e.target.value))}
            className="flex-1 h-1 bg-zinc-200 dark:bg-zinc-700 rounded-lg appearance-none cursor-pointer accent-black dark:accent-white"
          />
          <span className="font-mono text-[11px] font-medium text-zinc-700 dark:text-zinc-300 w-8 text-right">
            {pin.opacity}%
          </span>
        </div>
      )}
    </div>
  );
};
