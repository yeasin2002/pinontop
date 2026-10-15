# PinIt — Tauri Rebuild Documentation Index

This folder contains the complete tutorial documentation for rebuilding PinIt
from scratch using **Tauri v2 + React + Rust**. Each file is self-contained and
numbered so you can read them in order or jump to any subsystem independently.

---

## Reading Order

| # | File | What it covers |
|---|------|----------------|
| 01 | [01-project-overview.md](./01-project-overview.md) | What PinIt is, every user-facing feature, and how the product behaves end-to-end |
| 02 | [02-architecture.md](./02-architecture.md) | System architecture, module responsibilities, and how data flows through the app |
| 03 | [03-data-and-state.md](./03-data-and-state.md) | `pinned.json` schema, the in-memory state model, opacity encoding, and shortcut string format |
| 04 | [04-window-pinning.md](./04-window-pinning.md) | How window pinning works at the Win32 level, UIPI, shell window exclusions, and topmost re-enforcement |
| 05 | [05-opacity-and-click-through.md](./05-opacity-and-click-through.md) | Per-window transparency, click-through mode, and the layered window model |
| 06 | [06-persistence.md](./06-persistence.md) | The full save/load lifecycle, atomic writes, crash recovery, and pending pins |
| 07 | [07-hotkeys.md](./07-hotkeys.md) | Global hotkey registration, the shortcut string format, conflict detection, and safe-modifier rules |
| 08 | [08-tray-and-single-instance.md](./08-tray-and-single-instance.md) | System tray icon, tray menu contents, balloon notifications, and single-instance enforcement |
| 09 | [09-autostart.md](./09-autostart.md) | Start-with-Windows via the HKCU Run key, repair logic, and dev-mode isolation |
| 10 | [10-session-and-restore.md](./10-session-and-restore.md) | Windows session-end handling, keeping pins across reboots, and restoring pins on next login |
| 11 | [11-flash-overlay.md](./11-flash-overlay.md) | The visual feedback overlay drawn around a window when it is pinned or unpinned |
| 12 | [12-ui-screens.md](./12-ui-screens.md) | Every screen, panel, and component in the UI with layout, content, and interaction details |
| 13 | [13-theme-and-styling.md](./13-theme-and-styling.md) | Light and dark palettes, colour tokens, typography, motion, and Windows colour-mode detection |
| 14 | [14-logging.md](./14-logging.md) | Log file location, format, rotation, and the privacy rule |
| 15 | [15-tauri-migration-notes.md](./15-tauri-migration-notes.md) | Tauri v2-specific guidance for rebuilding each subsystem — commands, events, permissions, and known gaps |

---

## How to Use This Documentation

- **If you are building feature by feature** — start at `01` and follow the numbers.
- **If you are building a specific subsystem** — jump to its file; each one opens with a summary of what it covers and any dependencies on other sections.
- **If you are an AI agent building this project** — read `01`, `02`, and `03` first to establish the full model, then read the subsystem file for whatever you are implementing next.

---

## Key Facts to Keep in Mind Across All Files

1. The original app has **two implementations**: a C++/Qt 6 build (v2.2.0, legacy) and a Rust/Slint build (v3.0.0, the shipping version). This documentation describes **what the product does**, not how those two codebases implement it. You are free to implement it differently in Tauri.
2. The **`pinned.json` file** is the single source of truth for all persisted state. Its schema must be preserved exactly so users can migrate from the original app.
3. The **shortcut string format** (`"super+ctrl+KeyT"`) originated in the first Tauri v1 build of this app. It must be preserved exactly for the same reason.
4. The product is **Windows-only**. Every Win32 call, registry access, and system tray feature described here is Windows-specific.
5. **Window titles must never be logged** — only process names. This is a hard privacy rule.
