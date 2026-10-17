export interface PinItem {
  hwnd?: number;
  process_name: string;
  title: string;
  opacity: number; // 20 to 100
  click_through: boolean;
  was_layered?: boolean;
  was_topmost?: boolean;
  was_click_through?: boolean;
  is_pending?: boolean;
}

export interface Shortcuts {
  toggle_pin: string;
  opacity_up: string;
  opacity_down: string;
  toggle_window: string;
}

export interface Settings {
  enable_sound: boolean;
  show_notifications: boolean;
  has_seen_tray_notice: boolean;
  start_with_windows: boolean;
  shortcuts: Shortcuts;
}

export interface WindowInfo {
  hwnd: number;
  title: string;
  process_name: string;
  is_pinned: boolean;
}

export interface ToastMessage {
  id: string;
  message: string;
  type?: 'info' | 'warning' | 'error' | 'success';
}

export type SheetView = 'none' | 'picker' | 'shortcuts' | 'settings';
