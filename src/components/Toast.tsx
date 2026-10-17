import React, { useEffect } from "react";
import { ToastMessage } from "../types";
import { CheckIcon, AlertTriangleIcon } from "./icons";

interface ToastProps {
  toast: ToastMessage | null;
  onDismiss: () => void;
}

export const Toast: React.FC<ToastProps> = ({ toast, onDismiss }) => {
  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => {
      onDismiss();
    }, 3200);
    return () => clearTimeout(timer);
  }, [toast, onDismiss]);

  if (!toast) return null;

  return (
    <div className="fixed bottom-4 left-1/2 -translate-x-1/2 z-50 pointer-events-none transition-all duration-200 animate-slide-up">
      <div className="pointer-events-auto flex items-center gap-2 px-3.5 py-2 rounded-md bg-zinc-950 text-white dark:bg-zinc-100 dark:text-zinc-950 text-xs font-medium shadow-lg border border-zinc-800 dark:border-zinc-300">
        {toast.type === "error" || toast.type === "warning" ? (
          <AlertTriangleIcon size={14} className="text-yellow-400 flex-shrink-0" />
        ) : (
          <CheckIcon size={14} className="text-emerald-400 dark:text-emerald-600 flex-shrink-0" />
        )}
        <span>{toast.message}</span>
      </div>
    </div>
  );
};
