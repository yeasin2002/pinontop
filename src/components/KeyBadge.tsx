import React from "react";

interface KeyBadgeProps {
  keys: string[];
  separator?: string;
  className?: string;
  size?: "sm" | "md";
}

// Convert shortcut tokens like "super" -> "Win", "ctrl" -> "Ctrl", "KeyT" -> "T", "Equal" -> "=", "Minus" -> "-"
export function formatKeyToken(token: string): string {
  const lower = token.toLowerCase();
  switch (lower) {
    case "super":
    case "win":
    case "meta":
      return "Win";
    case "ctrl":
    case "control":
      return "Ctrl";
    case "alt":
      return "Alt";
    case "shift":
      return "Shift";
    case "equal":
      return "=";
    case "minus":
      return "-";
    default:
      if (token.startsWith("Key") && token.length === 4) {
        return token.slice(3).toUpperCase();
      }
      if (token.startsWith("Digit") && token.length === 6) {
        return token.slice(5);
      }
      return token;
  }
}

export const KeyBadge: React.FC<KeyBadgeProps> = ({
  keys,
  separator = "+",
  className = "",
  size = "md",
}) => {
  return (
    <span className={`inline-flex items-center gap-1.5 select-none ${className}`}>
      {keys.map((k, index) => {
        const isOr = k === "/";
        if (isOr) {
          return (
            <span key={index} className="text-xs text-zinc-400 font-mono">
              /
            </span>
          );
        }

        return (
          <React.Fragment key={index}>
            <kbd
              className={`inline-flex items-center justify-center font-mono font-medium rounded-sm border transition-colors ${
                size === "sm"
                  ? "px-1.5 py-0.5 text-[10px] min-w-[20px] h-[20px]"
                  : "px-2 py-0.5 text-[11px] min-w-[24px] h-[24px]"
              } bg-zinc-100 dark:bg-zinc-800/80 border-zinc-200 dark:border-zinc-700 text-zinc-900 dark:text-zinc-100 shadow-[0_1px_1px_rgba(0,0,0,0.05)]`}
            >
              {formatKeyToken(k)}
            </kbd>
            {index < keys.length - 1 && keys[index + 1] !== "/" && (
              <span className="text-[10px] text-zinc-400 dark:text-zinc-500 font-mono">
                {separator}
              </span>
            )}
          </React.Fragment>
        );
      })}
    </span>
  );
};
