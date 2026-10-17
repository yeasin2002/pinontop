import React from "react";
import { PinItem } from "../types";
import { PinRow } from "./PinRow";
import { KeyBadge } from "./KeyBadge";
import { PinIcon } from "./icons";

interface PinListProps {
  pins: PinItem[];
  onOpacityChange: (index: number, percent: number) => void;
  onClickThroughToggle: (index: number) => void;
  onUnpin: (index: number) => void;
  onUnpinAll: () => void;
  onHoverRow?: (hwnd: number | undefined, hovered: boolean) => void;
}

export const PinList: React.FC<PinListProps> = ({
  pins,
  onOpacityChange,
  onClickThroughToggle,
  onUnpin,
  onUnpinAll,
  onHoverRow,
}) => {
  const pinCount = pins.length;

  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <h2 className="text-[11px] font-mono font-semibold tracking-wider text-zinc-500 dark:text-zinc-400 uppercase">
          Pinned ({pinCount})
        </h2>
        {pinCount > 0 && (
          <button
            type="button"
            onClick={onUnpinAll}
            className="text-[11px] font-mono text-zinc-500 hover:text-red-600 dark:hover:text-red-400 transition-colors"
          >
            Unpin all
          </button>
        )}
      </div>

      {pinCount === 0 ? (
        <div className="flex flex-col items-center justify-center p-6 text-center rounded-lg border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/40">
          <div className="w-10 h-10 rounded-md bg-zinc-100 dark:bg-zinc-800 flex items-center justify-center text-zinc-400 dark:text-zinc-500 mb-3">
            <PinIcon size={20} />
          </div>
          <p className="text-sm font-semibold text-zinc-900 dark:text-zinc-100 mb-1">
            No windows pinned
          </p>
          <p className="text-xs text-zinc-500 dark:text-zinc-400 mb-3">
            Focus any window and press shortcut to pin
          </p>
          <div className="flex items-center justify-center gap-1.5">
            <span className="text-xs text-zinc-400 font-mono">Use</span>
            <KeyBadge keys={["super", "ctrl", "KeyT"]} size="sm" />
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-2 max-h-[280px] overflow-y-auto pr-0.5 custom-scrollbar">
          {pins.map((pin, index) => (
            <PinRow
              key={pin.hwnd || `${pin.process_name}-${index}`}
              pin={pin}
              onOpacityChange={(percent) => onOpacityChange(index, percent)}
              onClickThroughToggle={() => onClickThroughToggle(index)}
              onUnpin={() => onUnpin(index)}
              onHover={(hovered) => onHoverRow?.(pin.hwnd, hovered)}
            />
          ))}
        </div>
      )}
    </section>
  );
};
