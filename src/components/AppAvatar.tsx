import React from "react";

interface AppAvatarProps {
  processName: string;
  isPending?: boolean;
  size?: number;
}

export const AppAvatar: React.FC<AppAvatarProps> = ({
  processName,
  isPending = false,
  size = 32,
}) => {
  // Extract clean initial letter
  const cleanName = processName.replace(/\.exe$/i, "");
  const firstChar = cleanName.match(/[a-zA-Z0-9]/)?.[0]?.toUpperCase() || "?";

  return (
    <div
      style={{ width: `${size}px`, height: `${size}px` }}
      className={`rounded-md flex items-center justify-center font-mono font-bold text-xs select-none transition-opacity ${
        isPending ? "opacity-50 border border-dashed border-zinc-400" : ""
      } bg-black text-white dark:bg-white dark:text-black border border-black dark:border-white shadow-sm flex-shrink-0`}
      title={processName}
    >
      {firstChar}
    </div>
  );
};
