# 15 — Tauri Migration Notes

## 1. Overview

This document provides Tauri v2 + React-specific guidance for implementing every subsystem described in this documentation suite. It assumes you have already read the relevant subsystem files. Its purpose is to bridge the gap between the platform-agnostic descriptions in those files and the practical realities of building with Tauri.

---

## 2. Project Structure

A clean starting structure for the Tauri rebuild:

```
src-tauri/
├── src/
│   ├── main.rs          — entry point; Tauri app builder; setup hook
│   ├── pins.rs          — pin/unpin bookkeeping (no Win32, no UI)
│   ├── win.rs           — all Win32 calls on foreign windows
│   ├── host.rs          — hidden helper window, hotkeys, session events
│   ├── flash.rs         — layered overlay window
│   ├── store.rs         — pinned.json load/save
│   ├── shortcuts.rs     — shortcut string parsing and building
│   ├── autostart.rs     — HKCU Run key
│   ├── logfile.rs       — diagnostic log
│   └── commands.rs      — all Tauri command handlers (the bridge to React)
├── Cargo.toml
└── tauri.conf.json

src/                     — React frontend
├── components/
│   ├── PinList.tsx
│   ├── PinRow.tsx
│   ├── WindowPicker.tsx
│   ├── ShortcutEditor.tsx
│   ├── Settings.tsx
│   └── Toast.tsx
├── hooks/
│   ├── usePins.ts       — subscribes to pin-list events from backend
│   └── useTheme.ts      — light/dark mode detection
├── App.tsx
└── main.tsx
```

---

## 3. Startup Sequence

The Tauri app setup hook in `main.rs` runs before the window is shown. This is the right place for everything that must happen before the UI is ready.

Startup order:
1. Initialise the log file.
2. Log `"PinIt <version> starting"`.
3. Check single-instance mutex. If another instance is running, signal it and exit.
4. Load `pinned.json`.
5. Register global hotkeys (via the hidden helper window).
6. Create the hidden helper window (tray icon, session events, Explorer restart recovery).
7. Add the system tray icon.
8. Restore saved pins: pin open windows immediately, move the rest to pending.
9. Save the now-current state to `pinned.json`.
10. If `--minimized` flag is present, do not show the Tauri window.

In Tauri v2, use the `setup` hook on the `Builder`:

```
// Pseudocode — not real Rust
tauri::Builder::default()
    .setup(|app| {
        // steps 1-10 above
        Ok(())
    })
    .run(...)
```

---

## 4. Tauri Commands (Backend → Frontend Bridge)

All backend operations that the React UI needs to trigger are exposed as Tauri commands. Each command is an `async fn` in `commands.rs` annotated with `#[tauri::command]`.

### 4.1 Recommended command list

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `get_state` | none | `AppState` (pins + settings) | Get full current state on load |
| `toggle_pin` | `hwnd: i64` | `Result<PinResult>` | Pin or unpin a window |
| `set_opacity` | `hwnd: i64, percent: i32` | `Result<()>` | Set opacity for a pinned window |
| `set_click_through` | `hwnd: i64, enabled: bool` | `Result<()>` | Toggle click-through |
| `unpin_all` | none | `Result<()>` | Remove all pins |
| `forget_pending` | `index: usize` | `Result<()>` | Remove a pending pin |
| `get_windows` | none | `Vec<WindowInfo>` | List all pinnable windows for the picker |
| `save_shortcuts` | `shortcuts: Shortcuts` | `Result<ShortcutResult>` | Apply and save new shortcuts |
| `save_settings` | `settings: Settings` | `Result<()>` | Save settings |
| `set_autostart` | `enabled: bool` | `Result<()>` | Enable or disable autostart |
| `get_autostart` | none | `bool` | Read autostart state from registry |
| `row_hovered` | `hwnd: i64, hovered: bool` | `()` | Show/hide hover flash overlay |
| `open_url` | `url: String` | `()` | Open a URL in the default browser |

In the React frontend, call these via `invoke` from `@tauri-apps/api/core`:

```typescript
// Pseudocode
import { invoke } from '@tauri-apps/api/core';

const pins = await invoke<AppState>('get_state');
await invoke('set_opacity', { hwnd: pin.hwnd, percent: 75 });
```

### 4.2 State types

Define matching types in both Rust (for the commands) and TypeScript (for the frontend). Use serde `#[derive(Serialize, Deserialize)]` on the Rust side. The TypeScript types should mirror them exactly.

---

## 5. Tauri Events (Backend → Frontend Push)

The React UI must update in real time when pins change due to hotkeys, re-enforcement, or session events. This requires the backend to push events to the frontend — Tauri's event system handles this.

### 5.1 Recommended events

| Event name | Payload | When emitted |
|-----------|---------|-------------|
| `pins-changed` | `PinsPayload` (full pin + pending lists) | Any time the pin list changes |
| `settings-changed` | `Settings` | Settings updated |
| `toast` | `{ message: string }` | Error or info message to show in the toast |
| `pin-toggled` | `{ hwnd: i64, pinned: bool, title: string }` | A specific window was pinned or unpinned |
| `shortcut-conflicts` | `{ conflicts: string[] }` | Hotkey registration failures |

Emit from Rust using the app handle:

```
// Pseudocode
app_handle.emit("pins-changed", &pins_payload)?;
```

Subscribe in React:

```typescript
// Pseudocode
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen<PinsPayload>('pins-changed', (event) => {
    setPins(event.payload.pins);
    setPending(event.payload.pending);
});
```

Always store the `unlisten` function and call it on component unmount.

---

## 6. The Hidden Helper Window

The hidden helper window (for hotkeys, tray, session events) must be created in the Rust backend and cannot be a Tauri window — it is a raw Win32 window with its own `wndproc`.

Create it in the `setup` hook after the Tauri app is initialised. The helper window lives for the entire duration of the process.

The helper window runs its message loop inside Tauri's event loop. Do not create a separate thread. Use Tauri's `run_on_main_thread` or the Win32 message loop that Tauri's winit backend already runs — messages posted to the helper window's queue will be processed in the same loop.

To emit Tauri events from inside the `wndproc` (which is a C-style callback), use `slint::invoke_from_event_loop` (original) or Tauri's equivalent: store the `AppHandle` in a thread-local and call `app_handle.emit(...)` from a posted callback.

---

## 7. Tray Icon

In Tauri v2, use the built-in `SystemTray` API for the basic tray icon. However, PinIt's tray menu is dynamic (it lists pinned windows) and requires balloon notifications, which may need Win32 calls directly.

Options:
- Use Tauri's `SystemTray` for the icon and left-click handling; intercept right-click and show a custom Win32 `TrackPopupMenuEx` menu from the helper window.
- Or use Tauri's menu builder API if it supports dynamic items and checked states (verify for v2).

The helper window must re-add the tray icon on `TaskbarCreated` (Explorer restart) regardless of whether Tauri's API is used — Tauri does not handle this automatically.

---

## 8. Global Hotkeys

Tauri v2 has a built-in `GlobalShortcut` plugin. Evaluate whether to use it or use Win32 `RegisterHotKey` directly.

Recommendation: use Win32 `RegisterHotKey` directly via the `windows` crate. Reasons:
- The opacity hotkeys need `auto-repeat` behaviour (no `MOD_NOREPEAT`), which the Tauri plugin may not expose.
- `RegisterHotKey` is simple and well-understood.
- The helper window already has a `wndproc` that handles `WM_HOTKEY`; adding hotkeys there keeps all system message handling in one place.

If using Win32 directly, register hotkeys on the helper window's HWND (not NULL), as described in `07-hotkeys.md`.

---

## 9. Win32 Calls from Rust

Use the `windows` crate for all Win32 calls. Pin the version to a specific release:

```toml
[dependencies]
windows = { version = "=0.62.2", features = [...] }
```

Pin to an exact version (not a range). The `windows` crate has breaking changes between minor versions. See `03-data-and-state.md` (Cargo.toml notes) for the rationale.

Required feature flags:
```toml
"Win32_Foundation",
"Win32_Graphics_Dwm",
"Win32_Graphics_Gdi",
"Win32_Media_Audio",
"Win32_System_LibraryLoader",
"Win32_System_Registry",
"Win32_System_Threading",
"Win32_UI_Accessibility",
"Win32_UI_Input_KeyboardAndMouse",
"Win32_UI_Shell",
"Win32_UI_WindowsAndMessaging",
```

---

## 10. Flash Overlay

The flash overlay is a pure Win32 layered window. It is completely independent of Tauri's webview. Create and manage it entirely from the Rust backend.

The overlay's HWND is obtained at creation and kept for the process lifetime. Show, hide, and update it by calling Win32 functions directly from the backend command handlers (`row_hovered`, and from the internal pin/unpin logic).

There is no need to expose the flash overlay to the React frontend. When a command like `toggle_pin` succeeds, the backend shows the flash directly in Rust before returning. The frontend does not coordinate this.

---

## 11. Session End Handling

Tauri does not expose `WM_QUERYENDSESSION`. Handle it in the helper window's `wndproc`.

When `WM_QUERYENDSESSION` is received:
1. Set the `session_ending` flag in your shared state (`Arc<Mutex<AppState>>` or equivalent).
2. Return `TRUE` immediately.

When `WM_ENDSESSION` is received with `wParam != 0`:
1. Flush the state to `pinned.json` synchronously.
2. Release all windows.
3. Remove the tray icon.
4. Return `LRESULT(0)` quickly.

Do not call `app_handle.exit()` here — Windows is already shutting down the process.

---

## 12. Data Directory

Use `%LOCALAPPDATA%\PinIt` as the data directory. Do not use Tauri's `app_data_dir()` for `pinned.json` — it resolves to `%APPDATA%\com.pinit.app\` or similar, which would break compatibility with the existing file location that users already have.

Access `%LOCALAPPDATA%` via `std::env::var("LOCALAPPDATA")` and construct the path manually.

---

## 13. Single Instance

Implement single-instance enforcement in pure Win32 (mutex + `FindWindowW` + `PostMessageW`) as described in `08-tray-and-single-instance.md`. Do not use Tauri's `prevent_default_handler` or its `single-instance` plugin — they do not reproduce the exact behaviour (find the existing window, bring it to front, then exit).

---

## 14. Autostart

Implement autostart entirely in the Rust backend using the `windows` crate Registry API. Do not use Tauri's `autostart` plugin — it writes to a different Run key path and may use a different value name format, breaking compatibility with the existing registry entry.

---

## 15. pinned.json Compatibility

The `pinned.json` file written by the Tauri rebuild must be readable by the original app (and vice versa). This means:

- Same file path (`%LOCALAPPDATA%\PinIt\pinned.json`).
- Same schema: `pins` object with `"<process>:<index>"` keys, same field names.
- Same opacity encoding (8-bit alpha integer, not a float or percentage).
- Same shortcut string format (`"super+ctrl+KeyT"`).
- Unknown fields preserved on read/write.

Run the schema compatibility tests from `06-persistence.md` section 10 against the new implementation.

---

## 16. Tauri Window Configuration

In `tauri.conf.json`, configure the main window:

```json
{
  "width": 380,
  "height": 600,
  "resizable": false,
  "decorations": true,
  "skipTaskbar": true,
  "visible": false,
  "title": "PinIt"
}
```

- `skipTaskbar: true` keeps the window out of the taskbar.
- `visible: false` starts the window hidden; the setup hook shows it (unless `--minimized`).
- `resizable: false` fixes the width.

After the window is created, apply title bar styling via `DwmSetWindowAttribute` (see `13-theme-and-styling.md` section 8).

---

## 17. React State Management

The React frontend should maintain a local state that mirrors the backend state. The recommended approach:

- On mount, call `get_state` to get the initial full state.
- Subscribe to `pins-changed` and `settings-changed` events to receive real-time updates.
- When calling a command (e.g. `set_opacity`), update the local state optimistically (immediately), then confirm with the response.
- On error, revert to the last known good state.

Use React context or a lightweight state store (Zustand, Jotai) to share state between components without prop-drilling.

---

## 18. Permissions (Tauri v2)

In Tauri v2, all capabilities must be explicitly granted in the app's capability configuration. At minimum:

```json
{
  "permissions": [
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-close",
    "core:window:allow-set-focus",
    "core:event:allow-listen",
    "core:event:allow-emit"
  ]
}
```

For shell open (used by `open_url`):
```json
"shell:allow-open"
```

Custom commands (from `commands.rs`) do not need permission entries — they are registered directly and available to the frontend automatically.

---

## 19. Build and Release

The build pipeline from the original app (`.github/workflows/build.yml`) should be adapted:

1. `cargo test` — run backend unit tests.
2. `npm run build` (or equivalent) — build the React frontend.
3. `cargo tauri build` — produce `src-tauri/target/release/pinit.exe` and the installer.

Tauri's built-in bundler can produce an NSIS or WiX installer. Alternatively, the existing Inno Setup script can be adapted to package `src-tauri/target/release/pinit.exe`.

For the version: keep a single source of truth. Use `src-tauri/Cargo.toml`'s `[package].version` as the version. The React frontend can read it at build time via Tauri's `TAURI_APP_VERSION` environment variable.

---

## 20. Testing Strategy

The backend modules (`pins.rs`, `store.rs`, `shortcuts.rs`, `win.rs`, `autostart.rs`, `logfile.rs`) should have inline `#[cfg(test)]` unit tests. Write at minimum:

| Module | Tests to write |
|--------|---------------|
| `store.rs` | Schema round-trip, backwards compatibility, corrupt file, matching logic |
| `shortcuts.rs` | Parse defaults, build round-trip, display tokens, safe modifier validation |
| `win.rs` | Opacity round-trip losslessness (critical regression test), shell class detection |
| `autostart.rs` | Uses a scratch registry key; enable, disable, repair path |
| `pins.rs` | Pin/unpin logic, pending matching, saved output format |
| `logfile.rs` | Line format, rotation trigger |

Run all tests with:
```
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

For the React frontend, use Vitest + React Testing Library for component tests. Test the UI components in isolation with mocked `invoke` responses.
