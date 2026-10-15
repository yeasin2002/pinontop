# PinIt — Project Steering Document

## What PinIt Is

PinIt is a Windows 10/11 system tray utility that keeps any window always on top with a single global hotkey (`Win+Ctrl+T`). Secondary features: per-window opacity (20–100 %), click-through mode, pins that survive restarts, and a shortcut customiser. It is a focused single-purpose alternative to utility suites like PowerToys.

**Author:** Saqlain Abbas  
**License:** Apache 2.0  
**Repo:** https://github.com/Razee4315/Pin-It  
**Platform:** Windows 10 and 11, x64 only

---

## Two Implementations — Know Which One Is Active

The repo contains two complete, separate implementations that share the same `pinned.json` schema and shortcut string format so pins carry over between them:

| | C++ / Qt 6 | Rust / Slint |
|---|---|---|
| Location | `/src/` + `CMakeLists.txt` | `/rust/` + `rust/Cargo.toml` |
| Version | 2.2.0 | 3.0.0 |
| Status | **Legacy / reference** | **Shipping / active primary** |
| Ships with | 22 files (~35.8 MB) | 1 exe (~9.5 MB) |
| Private memory | ~11 MB | ~6 MB |

**The CI pipeline, installer, and all releases build the Rust version.** The C++ code is the canonical logic reference; the Rust code ports it faithfully.

The `README.md` still says "Built with C++ & Qt 6" — that is stale. When working on the shipping product, work in `rust/`.

---

## Architecture

```
rust/
├── src/
│   ├── main.rs        — entry point; wires all pieces; App struct; event dispatch
│   ├── pins.rs        — pure bookkeeping: pin/unpin/opacity/click-through/reenforce
│   ├── win.rs         — ALL Win32 calls on foreign windows (only file with unsafe)
│   ├── host.rs        — hidden helper window: tray, hotkeys, session events, single-instance
│   ├── flash.rs       — SDF-rendered layered overlay drawn around pinned windows
│   ├── store.rs       — serde JSON ↔ pinned.json; atomic saves
│   ├── shortcuts.rs   — "super+ctrl+KeyT" string parser/builder
│   ├── autostart.rs   — HKCU Run key management
│   └── logfile.rs     — diagnostic log; info!/warning! macros
└── ui/
    ├── theme.slint    — colour tokens, typography, motion durations (light + dark)
    ├── controls.slint — reusable components: Button, CheckBox, Slider, AppAvatar, Sheet
    └── app.slint      — AppWindow; exports PinRow, PickItem, ShortcutEdit to Rust
```

C++ counterpart modules (reference only):

| Rust module | C++ equivalent |
|---|---|
| `pins.rs` | `pinmanager.h/cpp` |
| `win.rs` | `winpin.h/cpp` |
| `host.rs` | `main.cpp` + `globalhotkey.cpp` + `sessionwatcher.cpp` + `singleinstance.cpp` |
| `flash.rs` | `pinflash.h/cpp` |
| `store.rs` | `persistence.h/cpp` |
| `shortcuts.rs` | `shortcuts.h/cpp` |
| `autostart.rs` | `autostart.cpp` |
| `logfile.rs` | `logging.h/cpp` |

---

## Build Commands

### Rust (primary — use this)

```powershell
# From the rust/ directory:
cargo test --locked              # run all unit tests
cargo build --release --locked  # produces rust/target/release/pinit.exe
cargo run                        # debug run
```

**Toolchain requirements:**
- Rust stable ≥ 1.88 (edition 2024)
- Windows SDK with `rc.exe` (for the exe's icon/version resource via `embed-resource`)
- The `windows-latest` GitHub Actions runner has all of this by default

**To run alongside an installed PinIt without touching its data:**
```powershell
$env:PINIT_DEV = "1"; cargo run
# Uses %LOCALAPPDATA%\PinIt-dev, its own mutex name, its own Run-key value.
```

### C++ / Qt (legacy — only if working on the reference implementation)

```powershell
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=<Qt6-path>
cmake --build build
ctest --test-dir build          # run tests
```

**Requirements:** Qt 6.5+, CMake 3.21+, C++17 (MinGW or MSVC).

### Installer

```powershell
# Requires Inno Setup 6 and a release build already at rust/target/release/pinit.exe
& "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" installer\PinIt.iss
# Output: release\PinIt_<version>_x64-setup.exe
```

The installer reads the version directly from `rust/Cargo.toml`. That file is the single source of truth for the version number.

---

## Key Data & Conventions

### pinned.json schema

Location: `%LOCALAPPDATA%\PinIt\pinned.json` (or `PinIt-dev\` in dev mode).

```json
{
  "pins": {
    "<process.exe>:<index>": {
      "process_name": "notepad.exe",
      "title": "notes.txt - Notepad",
      "opacity": 204,
      "click_through": false,
      "was_layered": false,
      "was_topmost": false,
      "was_click_through": false
    }
  },
  "settings": {
    "enable_sound": true,
    "show_notifications": false,
    "has_seen_tray_notice": false,
    "start_with_windows": false,
    "shortcuts": {
      "toggle_pin": "super+ctrl+KeyT",
      "opacity_up": "super+ctrl+Equal",
      "opacity_down": "super+ctrl+Minus",
      "toggle_window": "super+ctrl+KeyP"
    }
  }
}
```

The `"<process>:<index>"` key format comes from the original Tauri era and is maintained for backwards compatibility. Both codebases preserve unknown JSON keys via `#[serde(flatten)]` / JSON round-trip so future versions never silently drop fields written by newer builds.

### Opacity encoding

Stored in JSON as 8-bit alpha (0–255). Displayed in the UI as percent (20–100).  
**Round-trip formulas (use these exactly — a bug here causes drift on every restart):**
- `alpha = (percent × 255 + 50) / 100`  (rounded)
- `percent = (alpha × 100 + 127) / 255`  (rounded)

A test guards this (`opacityRoundTripIsLossless` / `opacity_round_trips`).

### Shortcut string format

`"super+ctrl+KeyT"` — Tauri / Web API syntax. `"super"` means the Win key. Keys follow the Web API `KeyboardEvent.code` naming (`KeyT`, `Equal`, `Minus`, `ArrowUp`, `PageDown`, `F9`, etc.). Use `shortcuts::parse` / `shortcuts::build` to work with these strings; never construct them by hand.

### Shell classes that must never be pinned

Progman, WorkerW, Shell_TrayWnd, Shell_SecondaryTrayWnd, NotifyIconOverflowWindow, TopLevelWindowForOverflowXamlIsland, Windows.UI.Core.CoreWindow, XamlExplorerHostIslandWindow, MultitaskingViewFrame, TaskListThumbnailWnd, ForegroundStaging.

Both codebases maintain this list in `win::is_shell_class` / `winpin::isShellClass`. PinIt's own windows are also excluded (by PID comparison).

### UIPI / elevated windows

`SetWindowPos` is silently ignored on elevated windows. Both codebases detect this by calling `apply_topmost(hwnd)` then verifying `is_topmost(hwnd)`. Surface the error as "It may be running as administrator."

### Session handling

`WM_QUERYENDSESSION` / `WM_ENDSESSION` must be handled on a non-message-only window (message-only windows don't receive the broadcast). Session-end events are processed immediately (not via the event loop) because Windows expects an answer. When `session_ending = true`, the `reenforce` loop skips removing closed windows so pins survive a reboot.

### Atomic writes

Both codebases write to a temp file then rename — never write `pinned.json` directly. In Rust: `pinned.json.tmp` → rename. In C++: `QSaveFile`. On a corrupt file: rename to `pinned.json.corrupt`, never delete.

### Logging

Use `logfile::info!` / `logfile::warning!` (Rust) or `qInfo()` / `qWarning()` (C++).  
**Log process names only. Never log window titles** (titles can contain document names, URLs, message subjects, and users are asked to attach logs to bug reports).

### Single-instance

Mutex name: `PinIt_SingleInstance_v2` (same string used as the installer's `AppMutex`).  
On a second launch: find the existing instance's window by class, call `AllowSetForegroundWindow(pid)`, `PostMessageW(WM_SHOW_REQUEST)`, exit.

### Dev mode (`instance_suffix()`)

All names that must not collide with an installed copy get `instance_suffix()` appended:
- Single-instance mutex name
- Host window class name
- Autostart Run-key value name
- Data directory (`PinIt` → `PinIt-dev`)

---

## Unsafe Boundaries (Rust)

`unsafe` is used in `win.rs`, `host.rs`, and `flash.rs` only. Every other module (`pins.rs`, `store.rs`, `shortcuts.rs`, `autostart.rs`, `logfile.rs`, `main.rs`) is safe Rust. When adding Win32 calls, add them to `win.rs` or `host.rs`; never call `unsafe` from `pins.rs` or `main.rs`.

---

## State Management (Rust)

The `App` struct owns all mutable state. It is wrapped in `Rc<RefCell<App>>` and lives on the single Slint UI thread. Module-level state in `host.rs` and `flash.rs` uses `thread_local!` statics. There are no worker threads; all timers and events arrive on the UI thread.

---

## CI/CD

The workflow (`.github/workflows/build.yml`) runs on every push/PR to `main`/`master` and on version tags (`v*`). It:
1. Runs `cargo test --locked`
2. Runs `cargo build --release --locked`
3. Stages the exe, runs Inno Setup, zips a portable build, computes `SHA256SUMS.txt`
4. Uploads all as the `PinIt-windows` artifact

On a version tag, a second job downloads the artifact and publishes a GitHub Release with auto-generated release notes and a stable-named `PinIt-setup-x64.exe`.

**Third-party actions are pinned to full commit SHAs** (not just tag names). Keep this practice when adding or updating actions.

---

## Testing

### Rust tests (run these first)

```powershell
cd rust
cargo test --locked
```

Tests live inline in each module (`#[cfg(test)]`):
- `store.rs` — schema round-trip, backwards compat, corrupt file, matching logic
- `win.rs` — opacity round-trip, shell class detection
- `shortcuts.rs` — parse/build/display round-trips
- `logfile.rs` — line format, rotation, timestamp shape

### C++ tests

```powershell
cmake --build build
ctest --test-dir build
# or: build/pinit_tests.exe
```

`tests/test_pinit.cpp` covers opacity conversion, shortcut parsing, pin matching, autostart (scratch registry key), and persistence (scratch `LOCALAPPDATA` via `QTemporaryDir`). The autostart test cleans up `HKCU\Software\PinIt-Tests` after itself.

**Do not add tests that write to the real `LOCALAPPDATA\PinIt` directory** or the real `HKCU\...\Run` key. Use `ScratchDataDir` / `setRegistryKeyForTesting()` fixtures.

---

## Coding Conventions

### Rust
- Standard `snake_case` for functions/variables/modules, `PascalCase` for types/enums.
- No `unwrap()` on anything that can fail at runtime. Return `Result` or `Option`; surface errors via `PinError::message()`.
- `panic = "abort"` in release; the panic hook in `logfile::init()` logs the info before abort. Do not add `panic = "unwind"`.
- Module-level `pub` items should have a doc comment explaining their contract.

### C++
- `.clang-format`: LLVM base, 4-space indent, 100-column limit, `PointerAlignment: Right`.
- Naming: `PascalCase` classes, `camelCase` methods, `m_` prefix for members, `k` prefix for constants.
- Namespaces for every module (`winpin::`, `persistence::`, `pinmatch::`, etc.); anonymous namespace for file-local helpers.
- No raw `#include <windows.h>` in headers. Keep Win32 types behind `winpin::WindowId` (= `intptr_t`).

### Both
- Add comments explaining non-obvious Win32 behaviour (UIPI, WS_EX_LAYERED, DWM cloaking, WINEVENT_OUTOFCONTEXT, etc.). This codebase is read as documentation.
- Keep opacity formulas consistent with the constants in `win.rs` (`MIN_OPACITY = 20`, `MAX_OPACITY = 100`).
- All four shortcut IDs (`HOTKEY_PIN = 1`, `HOTKEY_OPACITY_UP = 2`, `HOTKEY_OPACITY_DOWN = 3`, `HOTKEY_WINDOW = 4`) are stable — do not renumber them.

---

## Known Gaps in the Rust Port

The `rust/README.md` tracks two open items:
1. **Mica backdrop** — Windows 11 Mica/Acrylic title-bar effect not yet implemented
2. **Maximize button** — The window is fixed-width but Windows still shows the maximize button

Do not implement either without first checking whether the `rust/README.md` still lists them, and do not mark them done in the README without a working implementation.

---

## Flash Overlay Details

The flash (coloured outline drawn around a window when pinned/unpinned) positions itself using `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS)` — not `GetWindowRect` — to get the visible frame without Windows 10/11's invisible resize borders. The overlay window itself must be `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST` and must never be listed as pinnable (`isPinnable` / `is_pinnable` already excludes PinIt's own process).

Colours: orange `#C49464` (pin), grey `#8A857C` (unpin). Hold 300 ms, quadratic-ease fade 250 ms. `Kind::Hover` is persistent (no fade timer) — only dismissed when the row is unhovered.

---

## Installer Notes

- Version is read from `rust/Cargo.toml` `[package].version` using Inno Setup's INI reader.
- Pre-release suffixes (`3.0.0-alpha.1`) are stripped to a numeric form for `VersionInfoVersion`.
- Per-user install (`PrivilegesRequired=lowest`) — no UAC prompt.
- `AppMutex = PinIt_SingleInstance_v2` — must match the mutex name in `host.rs`.
- `[InstallDelete]` removes Qt DLLs and subdirectories from the old 2.2.0 Qt build on upgrade. Do not remove these lines.

---

## Dependency Pins

The `windows` crate is pinned to `=0.62.2` in `Cargo.toml`. A comment explains why: version 0.100 renames every feature and breaks the API. **Do not float this version** without a full migration of all `windows::Win32::*` imports in `win.rs`, `host.rs`, and `flash.rs`.

All other dependencies may be updated with care; run `cargo test --locked` after any change.

---

## Privacy Rule

Never log, display, or store window titles in log files. Process names are safe to log. This applies to all future code: use `process_name` in diagnostics, not `title`.
