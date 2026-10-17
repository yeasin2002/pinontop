# PinOnTop — Project Steering Document

## 1. Project Overview

**PinOnTop** (repository: `pinontop`, product name: **PinIt**) is a Windows 10/11 system utility that lets users keep any application window permanently on top of all others ("always on top") via a global hotkey. Its three design principles: zero friction (one hotkey, no mouse), stays out of the way (system tray only, no taskbar button), and survives everything (pins persist across app restarts, Windows restarts, and crashes).

The project is a **Tauri v2 + React + Rust** rebuild of an app with two prior implementations: a C++/Qt 6 build (v2.2.0) and a Rust/Slint build (v3.0.0). The repository currently contains only the Tauri scaffold — the actual feature implementation has not been written yet. The `documentation/` folder (15 files, `00-index.md` through `15-tauri-migration-notes.md`) is the **complete specification** for the entire app.

**The app is Windows-only.** All Win32, registry, and system tray features are Windows-specific.

---

## 2. Tech Stack

| Layer | Technology | Version |
|---|---|---|
| Desktop runtime | Tauri | v2 |
| Rust edition | 2024 | `edition = "2024"` |
| Frontend framework | React | ^19.1.0 |
| Frontend language | TypeScript | ~6.0.3 |
| Build tool | Vite | ^8.0.16 |
| Package manager | **pnpm** | (always use pnpm, never npm/yarn) |
| Serialization | serde + serde_json | 1.x |
| Tauri opener plugin | tauri-plugin-opener | 2.x |

**Never switch the package manager away from pnpm.** All scripts in `package.json` and `tauri.conf.json` reference `pnpm`.

---

## 3. Repository Structure

### Current (scaffold)
```
pinontop/
├── src/                        ← React frontend (currently: Tauri template)
│   ├── main.tsx                ← ReactDOM.createRoot entry
│   ├── App.tsx                 ← Single placeholder component
│   ├── App.css                 ← Template styles (to be replaced)
│   └── assets/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs             ← Binary entry; delegates to lib
│   │   └── lib.rs              ← Library root; registers Tauri commands
│   ├── capabilities/
│   │   └── default.json        ← Tauri v2 capability declarations
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── build.rs
├── index.html
├── vite.config.ts
├── package.json
├── tsconfig.json
└── documentation/              ← Full specification (read before implementing anything)
```

### Intended (spec-defined) structure
```
src-tauri/src/
├── main.rs         ← Tauri Builder + setup hook
├── commands.rs     ← All #[tauri::command] handlers (the bridge layer)
├── pins.rs         ← Pin/unpin bookkeeping — NO Win32, NO UI
├── win.rs          ← ALL Win32 calls — the ONLY place for unsafe code
├── host.rs         ← Hidden helper Win32 window (hotkeys, tray, session events)
├── flash.rs        ← Layered overlay window for visual feedback
├── store.rs        ← pinned.json read/write
├── shortcuts.rs    ← Shortcut string parsing and building
├── autostart.rs    ← HKCU Run key
└── logfile.rs      ← Diagnostic log

src/
├── components/
│   ├── PinList.tsx
│   ├── PinRow.tsx
│   ├── WindowPicker.tsx
│   ├── ShortcutEditor.tsx
│   ├── Settings.tsx
│   └── Toast.tsx
├── hooks/
│   ├── usePins.ts      ← subscribes to pins-changed events from backend
│   └── useTheme.ts     ← light/dark mode detection
├── App.tsx
└── main.tsx
```

---

## 4. Architecture

The app has three strict layers. **The platform layer never calls upward.** The application layer calls the platform layer and emits events upward. The UI calls downward in response to user actions.

```
┌─────────────────────────────────────────────┐
│    React UI  (webview)                      │
│    invoke() ↓          ↑ app_handle.emit()  │
├─────────────────────────────────────────────┤
│    commands.rs  (Tauri command bridge)      │
├─────────────────────────────────────────────┤
│    Application logic                        │
│    pins.rs | store.rs | shortcuts.rs        │
│    autostart.rs | logfile.rs                │
├─────────────────────────────────────────────┤
│    Platform layer  (Windows-only)           │
│    win.rs | host.rs | flash.rs              │
│    ← all unsafe Win32 calls confined here   │
└─────────────────────────────────────────────┘
```

### Frontend ↔ Backend communication
- **Frontend → Backend:** `invoke()` from `@tauri-apps/api/core`
- **Backend → Frontend:** `app_handle.emit()` from Rust; `listen()` from `@tauri-apps/api/event` in React
- React does **not** poll — it is purely event-driven. Always store the `unlisten` cleanup function and call it on component unmount.

---

## 5. Tauri Commands (Full Planned List)

All commands live in `commands.rs`. Each is an `async fn` annotated with `#[tauri::command]` and registered via `tauri::generate_handler![]` in `lib.rs`.

| Command | Parameters | Returns | Description |
|---|---|---|---|
| `get_state` | — | `AppState` | Full current state (pins + settings) on load |
| `toggle_pin` | `hwnd: i64` | `Result<PinResult>` | Pin or unpin a window |
| `set_opacity` | `hwnd: i64, percent: i32` | `Result<()>` | Set opacity for a pinned window |
| `set_click_through` | `hwnd: i64, enabled: bool` | `Result<()>` | Toggle click-through |
| `unpin_all` | — | `Result<()>` | Remove all pins |
| `forget_pending` | `index: usize` | `Result<()>` | Remove a pending pin |
| `get_windows` | — | `Vec<WindowInfo>` | List all pinnable windows for the picker |
| `save_shortcuts` | `shortcuts: Shortcuts` | `Result<ShortcutResult>` | Apply and save new shortcuts |
| `save_settings` | `settings: Settings` | `Result<()>` | Save settings |
| `set_autostart` | `enabled: bool` | `Result<()>` | Enable or disable autostart |
| `get_autostart` | — | `bool` | Read autostart state from registry |
| `row_hovered` | `hwnd: i64, hovered: bool` | `()` | Show/hide hover flash overlay |
| `open_url` | `url: String` | `()` | Open URL in default browser |

**Custom commands do not need capability entries** — they are registered via `generate_handler![]` and available automatically.

---

## 6. Tauri Events (Backend → Frontend Push)

| Event name | Payload | When emitted |
|---|---|---|
| `pins-changed` | Full pin + pending lists | Any time pin list changes |
| `settings-changed` | `Settings` object | Settings updated |
| `toast` | `{ message: string }` | Error or info message |
| `pin-toggled` | `{ hwnd: i64, pinned: bool, title: string }` | Window pinned/unpinned |
| `shortcut-conflicts` | `{ conflicts: string[] }` | Hotkey registration failures |

---

## 7. Data: `pinned.json`

### Location
| Mode | Path |
|---|---|
| Normal | `%LOCALAPPDATA%\PinIt\pinned.json` |
| Dev (`PINIT_DEV=1`) | `%LOCALAPPDATA%\PinIt-dev\pinned.json` |

**Always use `std::env::var("LOCALAPPDATA")` directly — NOT Tauri's `app_data_dir()`**, which would break compatibility with existing user files.

### Schema (must be preserved exactly — backward compat with C++/Qt6 and Rust/Slint builds)
```json
{
  "pins": {
    "notepad.exe:0": {
      "process_name": "notepad.exe",
      "title": "notes.txt - Notepad",
      "opacity": 255,
      "click_through": false,
      "was_layered": true,
      "was_topmost": true,
      "was_click_through": true
    }
  },
  "settings": {
    "enable_sound": true,
    "show_notifications": false,
    "has_seen_tray_notice": false,
    "start_with_windows": false,
    "shortcuts": {
      "toggle_pin":    "super+ctrl+KeyT",
      "opacity_up":    "super+ctrl+Equal",
      "opacity_down":  "super+ctrl+Minus",
      "toggle_window": "super+ctrl+KeyP"
    }
  }
}
```

- `pins` is a JSON **object** (not array). Keys are `"<processName>:<index>"`.
- On load: parse → extract index from key → sort by index → discard keys.
- On save: write each pin at position `i` under key `"<processName>:<i>"`.
- **Unknown fields must be preserved verbatim** — use serde `#[serde(flatten)]` to round-trip unknown fields at every level.
- HWNDs are **never** saved to disk.

### Opacity encoding (use exactly — lossless round-trip)
```
Percent → alpha:  alpha   = (percent × 255 + 50) / 100   [integer division]
Alpha → percent:  percent = (alpha × 100 + 127) / 255   [integer division]
```
Range: 20%–100%. Default `opacity = 255` (fully opaque, no transparency applied).

### Atomic saves
Always write to `pinned.json.tmp`, then rename over `pinned.json`. Never write directly.

### Corrupt file handling
If unparseable: rename to `pinned.json.corrupt`, return defaults. Do NOT delete the corrupt file.

---

## 8. Shortcut String Format

Format: `"<modifier>+<modifier>+...+<key>"`

Modifier tokens: `super`, `ctrl`, `alt`, `shift`  
Key tokens: follow Web API `KeyboardEvent.code` convention (e.g., `KeyT`, `Equal`, `Minus`, `KeyP`)

Default shortcuts:
- `toggle_pin`: `"super+ctrl+KeyT"`
- `opacity_up`: `"super+ctrl+Equal"`
- `opacity_down`: `"super+ctrl+Minus"`
- `toggle_window`: `"super+ctrl+KeyP"`

**This format must be preserved exactly** — it originated in the Tauri v1 build and is stored in user files.

Safe modifier rule: every shortcut must include at least one of Win (`super`), Ctrl, or Alt. Shift-only or bare-key shortcuts are rejected.

---

## 9. Win32 Implementation Rules

All `unsafe` Win32 calls are **strictly confined to `win.rs`**. No Win32 in any other module.

Key Win32 operations:
- Pin: `SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)`
- Unpin: `SetWindowPos(hwnd, HWND_NOTOPMOST, ...)`
- After pinning, always verify: `GetWindowLongW(hwnd, GWL_EXSTYLE) & WS_EX_TOPMOST` — if not set, UIPI blocked the call.
- Opacity: `SetLayeredWindowAttributes` (requires `WS_EX_LAYERED` first)
- Flash overlay: use `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS)` for positioning (not `GetWindowRect` — avoids invisible resize border)

HWNDs are stored as `i64` in Rust (isize on 64-bit Windows).

Required `windows` crate features (add to `Cargo.toml` when implementing Win32):
```toml
windows = { version = "=0.62.2", features = [
  "Win32_Foundation", "Win32_Graphics_Dwm", "Win32_Graphics_Gdi",
  "Win32_Media_Audio", "Win32_System_LibraryLoader", "Win32_System_Registry",
  "Win32_System_Threading", "Win32_UI_Accessibility",
  "Win32_UI_Input_KeyboardAndMouse", "Win32_UI_Shell", "Win32_UI_WindowsAndMessaging"
]}
```
**Pin to the exact version** (`=0.62.2`) — not a range. The `windows` crate has breaking changes between minor versions.

---

## 10. Key Constraints and Hard Rules

### Hard rules (never violate)
1. **Window titles are NEVER logged** — only process names. This is a privacy rule.
2. **`pinned.json` schema must match exactly** — backward compat with prior builds.
3. **Shortcut string format must match exactly** — stored in user files.
4. **Unknown JSON fields must be preserved** on every read/write cycle.
5. **Do NOT use Tauri's single-instance plugin** — use Win32 `CreateMutexW("PinIt_SingleInstance_v2")` directly.
6. **Do NOT use Tauri's autostart plugin** — use the Windows Registry via the `windows` crate directly.
7. **Do NOT use `app_data_dir()`** for the data directory — use `LOCALAPPDATA` env var directly.
8. **Helper window must be a regular top-level Win32 window** (NOT `HWND_MESSAGE`) — `WM_QUERYENDSESSION` is not delivered to message-only windows.
9. **`main.rs` must keep** `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` — suppresses the console window in release builds.
10. **Hotkeys via Win32 `RegisterHotKey` directly** — not the Tauri hotkeys plugin (opacity hotkeys need auto-repeat that the plugin may not expose).

### Session end vs manual quit
- **Session end** (`WM_ENDSESSION`): keep pins in file (for reboot survival), release windows visually.
- **Manual quit**: clear pins from file (start clean next time), release windows visually.

### Dev mode isolation
`PINIT_DEV=1` env var switches to isolated names: `PinIt-dev` data dir, `PinIt_SingleInstance_v2-dev` mutex, `PinIt-dev` log, `"PinIt-dev"` registry value name, `"PinItHost-dev"` helper window class.

---

## 11. Window Configuration

The Tauri window config in `tauri.conf.json` needs to be updated from the scaffold defaults:
```json
{
  "title": "pinontop",
  "width": 380,
  "height": 600,
  "resizable": false,
  "skipTaskbar": true,
  "visible": false
}
```
The main window starts hidden and is shown by the tray icon click or the `--minimized` flag check.

---

## 12. Rust Crate Structure Notes

The library crate is named `pinontop_lib` (not `pinontop`) to avoid a Windows-specific name collision between the lib and bin artifacts (Cargo issue #8519). It is compiled as `staticlib`, `cdylib`, and `rlib`. Do not rename it.

The release profile is already optimised for minimal size and maximum performance:
```toml
[profile.release]
codegen-units = 1
lto = true
opt-level = 3
panic = "abort"
strip = true
```

---

## 13. Frontend Conventions

- **State management:** Use Zustand or Jotai to share state across components without prop drilling.
- **Optimistic updates:** Update local state immediately on command call; revert on error.
- **No polling:** React subscribes to Tauri events (`listen()`). Always store and call the returned `unlisten` function on unmount.
- **TypeScript types** must mirror the Rust serde types exactly.
- **Strict TypeScript:** `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch` are all enabled.
- **Accessibility:** All interactive components must have proper ARIA labels and keyboard interaction.

### Theme / Styling
Warm "paper" palette (not generic blue/grey):
- Light mode background: `#f8f6f2`; Dark mode background: `#1f1d1a`
- Font: `"Segoe UI", system-ui, sans-serif`
- Spacing base unit: 4px
- Detect colour mode via `HKCU\...\Themes\Personalize\AppsUseLightTheme` registry (Rust side), emit `settings-changed` event when it changes.

---

## 14. Logging Rules

- Log file: `%LOCALAPPDATA%\PinIt\pinit.log`
- Format: `YYYY-MM-DD HH:MM:SS.mmm [I/W] message\r\n`
- Rotate at 512KB → rename to `pinit.log.old`
- **NEVER log window titles.** Only process names.
- Install a panic hook to log crashes before the process exits.
- Log file opened in append mode per write (no persistent handle).
- Log writer must be mutex-protected (Tauri backend is multi-threaded).

---

## 15. Build and Dev Workflow

```bash
# Development (runs pnpm dev + Rust backend simultaneously)
pnpm tauri dev

# Production build
pnpm tauri build

# Frontend-only type check
pnpm build   # runs tsc && vite build

# Vite dev server only
pnpm dev
```

The Vite dev server runs on fixed port `1420` (`strictPort: true`). The `src-tauri/` directory is excluded from Vite's file watcher.

---

## 16. Capabilities

Current `capabilities/default.json` grants:
- `core:default` — standard Tauri core permissions
- `opener:default` — `tauri-plugin-opener` for opening URLs/files

Additional capabilities to add as features are built:
- `core:window:allow-show`, `core:window:allow-hide`, `core:window:allow-close`, `core:window:allow-set-focus`
- `core:event:allow-listen`, `core:event:allow-emit`

Custom Tauri commands registered via `generate_handler![]` do **not** need capability entries.

---

## 17. Documentation Reference

Before implementing any subsystem, read the relevant file in `documentation/`:

| File | Covers |
|---|---|
| `00-index.md` | Master index, reading order, 5 key facts |
| `01-project-overview.md` | All user-facing features, defaults, what the app does NOT do |
| `02-architecture.md` | Three-layer architecture, all 8 module responsibilities, data flow diagrams |
| `03-data-and-state.md` | Complete `pinned.json` schema, opacity encoding, matching logic, unknown-field rules |
| `04-window-pinning.md` | Win32 pin/unpin procedure, UIPI detection, shell window blacklist, re-enforcement |
| `05-opacity-and-click-through.md` | Layered window model, opacity formulas, click-through, debounce |
| `06-persistence.md` | Save/load lifecycle, atomic writes, corrupt file handling, pending pins |
| `07-hotkeys.md` | Win32 hotkey registration, shortcut string format spec, safe-modifier rule |
| `08-tray-and-single-instance.md` | `Shell_NotifyIconW`, tray menu structure, balloon notifications, mutex |
| `09-autostart.md` | HKCU Run key, repair logic, dev-mode isolation |
| `10-session-and-restore.md` | `WM_QUERYENDSESSION`/`WM_ENDSESSION`, startup restore, pending pin lifecycle |
| `11-flash-overlay.md` | Win32 layered window, SDF outline, animation timing, DWM frame bounds |
| `12-ui-screens.md` | All screens, components, layout, interaction details |
| `13-theme-and-styling.md` | Full colour token tables, typography, motion, WCAG contrast |
| `14-logging.md` | Log format, rotation, privacy rule, what to log |
| `15-tauri-migration-notes.md` | **Most actionable** — maps every subsystem to Tauri v2 implementation |
