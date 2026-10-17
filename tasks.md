# pinontop — Implementation Roadmap & Task Tracker

This document tracks all remaining tasks required to bring **pinontop** (formerly PinIt) to 100% feature completion. It is derived directly from the full 16-file specification in [`documentation/`](file:///d:/programming/personal/pinontop/documentation/) and [`AGENTS.MD`](file:///d:/programming/personal/pinontop/AGENTS.MD).

---

## Current Status & Progress Summary

- [x] **Frontend Architecture & Design System**:
  - [x] Solid Monochrome Engineering System implemented (pure `#000000` / `#ffffff`, Zinc grayscale ladder, crisp `rounded-lg`/`rounded-md` radii).
  - [x] Typography wired via Google Fonts (Plus Jakarta Sans, Geist, Geist Mono).
  - [x] Main window UI (fixed 380px native layout) with Header, Primary Action button, Shortcuts card, Pinned list (empty & populated states), and Quick settings.
  - [x] 3 slide-up sheets implemented: Window Picker with search, Shortcut Editor with modifier chips & validation, and Settings sheet.
  - [x] Floating in-app toast notification system.
  - [x] Light / Dark mode toggle with `localStorage` persistence.
  - [x] Multi-resolution icon set generated for Windows (`icon.ico`, `.png`, Store logos).
- [ ] **Backend (Rust & Win32)**:
  - Scaffold only. Core Win32 platform layer, state persistence, hotkeys, helper window, tray, and IPC command handlers remain to be built.

---

## Phase 1: Rust Dependencies & Logging Infrastructure

### 1.1 Cargo Dependencies
- [x] Add the pinned `windows` crate (`=0.62.2`) with exact required feature flags to [`src-tauri/Cargo.toml`](file:///d:/programming/personal/pinontop/src-tauri/Cargo.toml):
  - `Win32_Foundation`, `Win32_Graphics_Dwm`, `Win32_Graphics_Gdi`, `Win32_Media_Audio`, `Win32_System_LibraryLoader`, `Win32_System_Registry`, `Win32_System_SystemInformation`, `Win32_System_Threading`, `Win32_UI_Accessibility`, `Win32_UI_Input_KeyboardAndMouse`, `Win32_UI_Shell`, `Win32_UI_WindowsAndMessaging`
- [x] Add `thiserror` (v2.x) to handle structured Rust error definitions.
- [x] Verify `cargo check` compiles cleanly on Windows.

### 1.2 Diagnostic Logging Subsystem (`src-tauri/src/logfile.rs`)
- [x] Implement thread-safe file logger writing to `%LOCALAPPDATA%\PinIt\pinit.log` (or `%LOCALAPPDATA%\PinIt-dev\pinit.log` when `PINIT_DEV=1`).
- [x] Ensure **Privacy Hard Rule**: Window titles must **NEVER** be written to logs (only process names, HWNDs, and error codes).
- [x] Log entry format: `YYYY-MM-DD HH:MM:SS.mmm [I/W] <message>\r\n`.
- [x] Size limit rotation: when log exceeds 512KB, atomically rotate to `pinit.log.old`.
- [x] Install a global panic hook via `std::panic::set_hook` to log crash details before process termination.
- [x] Unit tests: verify log format, rotation logic, and directory resolution.

---

## Phase 2: State Persistence & Shortcut Models

### 2.1 Storage & Schema Compatibility (`src-tauri/src/store.rs`)
- [ ] Define data directory resolution using `std::env::var("LOCALAPPDATA")` (do **not** use Tauri's `app_data_dir()`).
- [ ] Implement backward-compatible `pinned.json` serialization:
  - Key format: `"<process_name>:<index>"`.
  - Serde `#[serde(flatten)]` on all structs to preserve unknown legacy fields verbatim.
- [ ] Atomic save logic: write to `pinned.json.tmp`, then atomically rename over `pinned.json`.
- [ ] Corrupt file recovery: if JSON is unparseable, rename to `pinned.json.corrupt` and return defaults without crashing.
- [ ] Opacity lossless integer math:
  - Percent to Alpha: `alpha = (percent * 255 + 50) / 100`
  - Alpha to Percent: `percent = (alpha * 100 + 127) / 255`
- [ ] Unit tests: round-trip serialization, legacy compatibility fixtures, and corrupt file handling.

### 2.2 Shortcut Parser & Validator (`src-tauri/src/shortcuts.rs`)
- [ ] Implement parser for string tokens (e.g., `"super+ctrl+KeyT"`).
- [ ] Map tokens to Win32 Virtual Key codes (`VK_*`) and modifier flags (`MOD_WIN`, `MOD_CONTROL`, `MOD_ALT`, `MOD_SHIFT`).
- [ ] Enforce the Safe Modifier Rule: reject shortcuts that lack at least one of Win, Ctrl, or Alt.
- [ ] Provide factory defaults:
  - `toggle_pin`: `"super+ctrl+KeyT"`
  - `opacity_up`: `"super+ctrl+Equal"`
  - `opacity_down`: `"super+ctrl+Minus"`
  - `toggle_window`: `"super+ctrl+KeyP"`
- [ ] Unit tests: test modifier validation, key code mapping, and string rebuilds.

---

## Phase 3: Win32 Platform Layer (`src-tauri/src/win.rs`)

*Strict Architecture Rule: ALL `unsafe` Win32 calls must reside strictly within `win.rs`.*

### 3.1 Window Manipulation
- [ ] Pin window: call `SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)`.
- [ ] Unpin window: call `SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)`.
- [ ] UIPI Detection: verify `GetWindowLongW(hwnd, GWL_EXSTYLE) & WS_EX_TOPMOST`. If not set, report privilege isolation block (target app running as Administrator).
- [ ] Opacity control:
  - Add `WS_EX_LAYERED` style via `SetWindowLongW`.
  - Apply alpha via `SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA)`.
  - Remove layered flag on unpin if window was not layered initially.
- [ ] Click-through toggle: toggle `WS_EX_TRANSPARENT` style on layered windows.

### 3.2 Shell Blacklist & Window Enumeration
- [ ] Blacklist detector: identify and reject shell windows:
  - Desktop (`Progman`, `WorkerW`)
  - Taskbar (`Shell_TrayWnd`, `Shell_SecondaryTrayWnd`)
  - Start menu and notification center
  - pinontop's own window handles
- [ ] Implement `get_pinnable_windows()` using `EnumWindows`:
  - Filter out invisible, zero-size, minimized, or cloaked windows (`DwmGetWindowAttribute` with `DWMWA_CLOAKED`).
  - Extract process name via `GetWindowThreadProcessId` + `QueryFullProcessImageNameW`.
  - Extract window title via `GetWindowTextW`.

---

## Phase 4: Helper Window, Hotkeys, Single Instance & Session End

### 4.1 Single-Instance Enforcement (`src-tauri/src/main.rs`)
- [ ] Create named Win32 mutex `PinIt_SingleInstance_v2` (or `PinIt_SingleInstance_v2-dev` in dev mode).
- [ ] If `GetLastError() == ERROR_ALREADY_EXISTS`:
  - Find existing window via `FindWindowW("PinItHost", ...)`.
  - Post custom message to bring existing window to foreground.
  - Terminate current instance immediately.

### 4.2 Raw Win32 Helper Window (`src-tauri/src/host.rs`)
- [ ] Register window class `PinItHost` (regular top-level window, **not** `HWND_MESSAGE`, to receive session broadcast events).
- [ ] Embed helper window in the message loop to receive Windows messages without extra threads.
- [ ] Register global hotkeys using Win32 `RegisterHotKey`:
  - Ensure opacity hotkeys support auto-repeat.
  - Catch registration collisions and emit `shortcut-conflicts` event to frontend.

### 4.3 Session End vs Manual Quit (`src-tauri/src/host.rs`)
- [ ] Handle `WM_QUERYENDSESSION`: return `TRUE` and set `is_session_ending = true`.
- [ ] Handle `WM_ENDSESSION`:
  - If `wParam != 0`: save pins to disk for reboot survival, release visual topmost styles, return immediately.
- [ ] Handle manual quit: clear pins from `pinned.json` on user-initiated exit so app starts fresh next launch.
- [ ] Handle `WM_SETTINGCHANGE` (`"ImmersiveColorSet"`): notify frontend of Windows theme change.
- [ ] Handle `TaskbarCreated`: re-create system tray icon if Windows Explorer restarts.

---

## Phase 5: Visual Flash Overlay & Autostart

### 5.1 Flash Window Overlay (`src-tauri/src/flash.rs`)
- [ ] Create a borderless layered top-most Win32 window (`WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW`).
- [ ] Position accurately using `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS)` to avoid invisible Windows 10/11 drop-shadow margins.
- [ ] Draw high-contrast monochrome outline (solid white/black accent border).
- [ ] Implement smooth 300ms quadratic ease-out fade animation.
- [ ] Respect Windows accessibility setting: check `SPI_GETCLIENTAREAANIMATION` and skip animation if disabled.

### 5.2 Windows Registry Autostart (`src-tauri/src/autostart.rs`)
- [ ] Manage `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
- [ ] Key name: `PinIt` (or `PinIt-dev`).
- [ ] Command value: `"<path_to_pinontop.exe>" --minimized`.
- [ ] Path self-repair: update registry path if executable has been relocated.
- [ ] Do **not** use Tauri's autostart plugin (maintains backward compatibility).

---

## Phase 6: System Tray & Window Lifecycle

### 6.1 Tray Icon & Context Menu
- [ ] Create tray icon with `app-icon.png` / `icon.ico`.
- [ ] Context menu actions:
  - Dynamic list of currently pinned windows (click to unpin)
  - "Pin a window..." (opens Window Picker)
  - "Show pinontop" / "Hide pinontop"
  - "Settings"
  - "Quit"
- [ ] Minimize to tray on close button click:
  - Intercept window close event and hide window instead of exiting.
  - Show balloon notice on first-time hide ("pinontop is running in the system tray").

---

## Phase 7: Pin Bookkeeping Engine (`src-tauri/src/pins.rs`)

### 7.1 Pin Manager Architecture
- [ ] Manage synchronized thread-safe state:
  - `live_pins`: active windows with verified HWNDs.
  - `pending_pins`: saved pins whose process is not currently running.
- [ ] Topmost re-enforcement loop:
  - Run background interval (every 1.5 seconds) checking `WS_EX_TOPMOST` on all live pins.
  - Re-apply `HWND_TOPMOST` if another fullscreen/greedy app displaced it.
- [ ] Process watcher / pending pin resolution:
  - When new windows open, match against pending pins by `process_name` and restore pin status automatically.
- [ ] Emit Tauri event `pins-changed` whenever the list updates.

---

## Phase 8: Tauri IPC Commands & Capabilities

### 8.1 Command Bridge (`src-tauri/src/commands.rs`)
- [ ] Implement all 13 spec commands:
  1. `get_state()` -> Returns full `AppState` (pins + settings)
  2. `toggle_pin(hwnd)` -> Pins or unpins foreign window
  3. `set_opacity(hwnd, percent)` -> Debounced opacity adjustment
  4. `set_click_through(hwnd, enabled)` -> Toggles click passthrough
  5. `unpin_all()` -> Clears all live and pending pins
  6. `forget_pending(index)` -> Removes an orphaned pending pin
  7. `get_windows()` -> Lists pinnable windows for WindowPicker
  8. `save_shortcuts(shortcuts)` -> Re-registers global hotkeys
  9. `save_settings(settings)` -> Persists preferences
  10. `set_autostart(enabled)` -> Writes HKCU registry Run key
  11. `get_autostart()` -> Reads HKCU registry Run key
  12. `row_hovered(hwnd, hovered)` -> Triggers flash overlay on target window
  13. `open_url(url)` -> Opens link in default browser
- [ ] Register all commands in `tauri::generate_handler![]` in [`src-tauri/src/lib.rs`](file:///d:/programming/personal/pinontop/src-tauri/src/lib.rs).

### 8.2 Tauri Capabilities Configuration (`src-tauri/capabilities/default.json`)
- [ ] Add required Tauri window and event permissions:
  - `core:window:allow-show`, `core:window:allow-hide`, `core:window:allow-close`, `core:window:allow-set-focus`
  - `core:event:allow-listen`, `core:event:allow-emit`
  - `opener:default` / `shell:allow-open`

---

## Phase 9: Frontend & Backend Integration

### 9.1 Live Hook Binding ([`src/hooks/usePins.ts`](file:///d:/programming/personal/pinontop/src/hooks/usePins.ts))
- [ ] Switch from mock fallbacks to live Tauri `invoke()` calls.
- [ ] Subscribe to backend events: `pins-changed`, `settings-changed`, `toast`, `shortcut-conflicts`.
- [ ] Implement optimistic UI updates with rollback on command errors.
- [ ] Add sound effects playback for pin/unpin events using Web Audio API or bundled WAV clicks.

---

## Phase 10: Quality Assurance & Build Verification

### 10.1 Unit & Integration Testing
- [ ] Run backend unit tests: `cargo test --manifest-path src-tauri/Cargo.toml`.
- [ ] Verify `pinned.json` round-trip with Qt and Slint legacy test fixtures.
- [ ] Test UIPI privilege handling (verify clear toast error when attempting to pin an Admin window from standard user mode).
- [ ] Test Windows reboot survival vs clean manual exit.

### 10.2 Production Packaging
- [ ] Run release build: `pnpm tauri build`.
- [ ] Verify standalone executable `src-tauri/target/release/pinontop.exe` size (< 5MB target).
- [ ] Verify clean install and uninstall behavior via NSIS installer.
