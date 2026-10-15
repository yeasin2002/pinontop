# 02 — Architecture

## 1. Overview

PinIt is a single-process desktop application. It has no server, no cloud component, and no inter-process communication beyond what Windows provides natively for single-instance enforcement. All work happens in one process on the UI thread. There are timers, but no background worker threads.

The application divides cleanly into three layers:

```
┌─────────────────────────────────────────────────────┐
│                   UI Layer                          │
│  (React / Slint / Qt — what the user sees)          │
├─────────────────────────────────────────────────────┤
│                 Application Layer                   │
│  (Business logic: pins, settings, hotkeys, tray)    │
├─────────────────────────────────────────────────────┤
│               Platform Layer                        │
│  (All Win32 calls, registry, filesystem)            │
└─────────────────────────────────────────────────────┘
```

The platform layer never calls upward. The application layer calls the platform layer and emits events upward to the UI. The UI calls down into the application layer in response to user actions.

---

## 2. Module Responsibilities

Each module has one job. When rebuilding in Tauri, these map cleanly to Rust backend modules (invoked via Tauri commands) and React frontend components.

### 2.1 Window Pinning (`pins` module)

**What it owns:** The list of currently-pinned windows and the list of pending pins (saved windows not yet open).

**What it does:**
- Pin a window: mark it always-on-top via Win32, record its prior state, add it to the live list.
- Unpin a window: restore its prior state, remove it from the live list.
- Re-enforce: walk the live list and reapply always-on-top for any window that lost the flag. Remove any window from the list that no longer exists.
- Restore pending: check whether the current foreground window matches any saved pin that is still waiting; if so, apply it.
- Produce the save payload: convert the live list and pending list into the format written to `pinned.json`.

**What it does NOT own:** timers, UI events, file I/O, or Win32 calls. Those belong to the layers around it.

### 2.2 Win32 Abstraction (`win` module)

**What it owns:** Every direct call to the Windows API that touches a foreign window.

**What it does:**
- Get/set the topmost (always-on-top) flag on any HWND.
- Get/set the layered (transparency) attribute on any HWND.
- Get/set the click-through (transparent to mouse) attribute on any HWND.
- Get the window title, process name, class name, and visible frame rectangle for any HWND.
- Enumerate all pinnable windows currently on screen.
- Extract the icon of any window as an RGBA pixel buffer.
- Watch for foreground window changes via a Windows event hook.
- Play the pin sound.
- Detect whether Windows animations are enabled (for deciding whether to animate the flash overlay).

**Contract:** This module never touches application state. It receives an HWND (stored as a plain integer) and returns a result. All `unsafe` Win32 calls are confined to this module.

### 2.3 Helper Window / Host (`host` module)

**What it owns:** A hidden, top-level Win32 window that acts as PinIt's message sink.

**Why this exists:** Windows delivers certain system-wide messages to a specific window rather than broadcasting them. Global hotkeys (`WM_HOTKEY`), session-end notifications (`WM_QUERYENDSESSION`), tray icon events, and the "Explorer restarted" message all require a real HWND to receive them. PinIt creates one hidden window for this purpose.

**What it does:**
- Register and unregister global hotkeys.
- Own and manage the system tray icon.
- Show balloon notifications.
- Show the tray right-click context menu.
- Receive `WM_QUERYENDSESSION` / `WM_ENDSESSION` and forward them to the app as events.
- Receive the `TaskbarCreated` message (sent by Windows when Explorer restarts) and re-add the tray icon.
- Enforce single-instance: check for an existing mutex on startup; if one exists, find the other instance's window and ask it to show itself.

### 2.4 Persistence (`store` module)

**What it owns:** Reading and writing `pinned.json`.

**What it does:**
- Load the file on startup, returning a default state if it is missing or unreadable.
- Save the state atomically: write to a temporary file, then rename it over the real file.
- On a corrupt file: rename it to `pinned.json.corrupt` (for manual recovery) and return defaults.
- Preserve unknown JSON keys across load/save cycles so future versions of the file do not lose fields written by a newer build.

### 2.5 Shortcuts (`shortcuts` module)

**What it owns:** The shortcut string format used throughout the app.

**What it does:**
- Parse a shortcut string like `"super+ctrl+KeyT"` into a modifier bitmask and a virtual key code.
- Build a shortcut string from a set of modifier checkboxes and a key name.
- Produce the list of human-readable display tokens for showing in the UI (e.g. `["Win", "Ctrl", "T"]`).
- Validate that a shortcut has at least one safe modifier (Win, Ctrl, or Alt).
- Provide the list of all key labels offered in the shortcut editor.

### 2.6 Autostart (`autostart` module)

**What it owns:** The Windows Run key entry that starts PinIt with Windows.

**What it does:**
- Check whether the Run key value exists for the current executable path.
- Enable autostart: write the value.
- Disable autostart: delete the value.
- Repair: if the value exists but points to a path that no longer exists (e.g. the app was moved), update it to the current path.

### 2.7 Flash Overlay (`flash` module)

**What it owns:** The coloured outline drawn around a window when it is pinned or unpinned.

**What it does:**
- Show a coloured outline around a given HWND immediately.
- Distinguish between a "pin flash" (orange, timed), an "unpin flash" (grey, timed), and a "hover highlight" (orange, persistent until cancelled).
- Position the overlay using the window's DWM frame bounds.
- Fade out over 250 ms after holding for 300 ms (for timed flashes).
- Hide immediately when cancelled (for hover highlights).

### 2.8 Logging (`logfile` module)

**What it owns:** Writing diagnostic messages to `pinit.log`.

**What it does:**
- Write timestamped lines to `%LOCALAPPDATA%\PinIt\pinit.log`.
- Rotate to `pinit.log.old` when the log exceeds 512 KB.
- Provide `info!` and `warning!` macros for the rest of the app.
- Install a panic hook so crashes are recorded before the process exits.

---

## 3. Data Flow

### 3.1 Startup Flow

```
1. Check single-instance mutex
   └─ If already running: send "show" message to existing instance, exit.

2. Load pinned.json
   └─ Produces: list of saved pins + settings

3. Register global hotkeys

4. Create hidden helper window
   └─ Add tray icon

5. Restore saved pins:
   a. Enumerate all visible windows
   b. For each saved pin, find a matching live window → pin it immediately
   c. Remaining saved pins → move to "pending" list

6. Start re-enforce timer (2-second interval)
7. Start foreground-change event hook

8. Show main window (unless --minimized flag was passed)
```

### 3.2 Pin / Unpin Flow (hotkey triggered)

```
1. WM_HOTKEY received by helper window
2. Get the current foreground HWND
3. Check: is this HWND already pinned?
   ├─ Yes → unpin:
   │         restore topmost / opacity / click-through to original state
   │         remove from live list
   │         save to pinned.json
   │         show unpin flash
   │         notify UI
   └─ No  → pin:
             record current window state (was_topmost, was_layered, was_click_through)
             apply WS_EX_TOPMOST via SetWindowPos
             verify it actually took (UIPI check)
             add to live list with default opacity=100
             remove from pending list if present
             save to pinned.json
             show pin flash
             play sound (if enabled)
             notify UI
```

### 3.3 Re-enforce Flow (timer + foreground event)

```
1. Walk the live pin list:
   a. If a window no longer exists → remove it from the list
   b. If a window exists but lost WS_EX_TOPMOST → reapply it
   c. If a window's title changed → update the stored title, mark dirty

2. Check the current foreground window against the pending list:
   a. Does the foreground window match a saved pending pin?
      └─ Yes → apply the saved pin (opacity, click-through) → move to live list

3. If the list changed → save to pinned.json, notify UI
```

### 3.4 Session-End Flow

```
1. WM_QUERYENDSESSION received (Windows is shutting down / logging off)
2. Set session_ending = true
3. Reply TRUE immediately (PinIt agrees to end the session)

4. WM_ENDSESSION received (session is actually ending):
   a. Flush any pending writes to pinned.json
   b. Release all windows (restore their original state visually)
   c. Do NOT clear the pin list from the file
      └─ Pins must survive the reboot so they are restored on next login

5. Remove tray icon
```

### 3.5 Manual Quit Flow

```
1. User selects "Quit PinIt" from tray menu
2. Release all windows (restore original state)
3. Clear the pin list from pinned.json (keep settings)
4. Remove tray icon
5. Exit
```

The distinction between session-end quit and manual quit is critical: session-end preserves pins; manual quit clears them.

---

## 4. Event Model

The application is event-driven. There are five sources of events:

| Source | Examples |
|--------|---------|
| User hotkey | `WM_HOTKEY` → pin/unpin, opacity up/down, show/hide window |
| Tray icon | Click (show/hide), right-click (menu), balloon click |
| UI actions | Button clicks, slider changes, toggle switches |
| System timer | Re-enforce tick every 2 seconds |
| Windows system | Foreground changed, session ending, Explorer restarted |

All events ultimately reach the application layer which mutates state, then pushes updated state back to the UI.

---

## 5. Threading Model

PinIt is single-threaded on the UI thread. There are no worker threads. The re-enforce timer fires on the UI thread. Win32 event hooks (foreground change) deliver their callbacks out-of-context, which means Windows queues the event to the thread that registered the hook — the UI thread — so no locking is needed.

The only place that crosses a thread boundary is the session-end handler: `WM_QUERYENDSESSION` must be responded to immediately, so it is handled synchronously rather than being posted to the event loop.

---

## 6. State Boundaries

There are two categories of state:

**Persisted state** (lives in `pinned.json`, survives process exit):
- The list of saved pins (process name, title, opacity, click-through, prior state flags)
- User settings (sound, notifications, autostart, shortcuts)

**Ephemeral state** (lives in memory only, lost on exit):
- The live window list with HWNDs (HWNDs are process-lifetime values; they become invalid when the window closes)
- The current UI view state (which panel is open, current filter text, etc.)
- The flash overlay position and animation state

HWNDs are never written to `pinned.json`. The pairing between a saved pin and a live window is done at runtime by matching process name and window title.

---

## 7. Tauri Architecture Mapping

When rebuilding in Tauri, the layers map as follows:

| Original layer | Tauri equivalent |
|---------------|-----------------|
| Win32 abstraction (`win` module) | Rust backend module; called only by other backend modules |
| Application logic (`pins`, `store`, `shortcuts`, etc.) | Rust backend modules; exposed to the frontend via Tauri commands |
| Helper window (`host` module) | Rust backend; runs on startup, cannot be exposed as Tauri commands directly — must be initialised in `main.rs` and communicate via Tauri events |
| Flash overlay (`flash` module) | Rust backend; a pure Win32 layered window, independent of Tauri's webview |
| UI (React) | Tauri frontend; calls backend via `invoke()`, receives updates via Tauri events |
| Tray icon | Tauri's built-in `SystemTray` API (Tauri v2) |

The most important architectural decision in the Tauri rebuild is that **the helper window, hotkeys, flash overlay, and re-enforce timer all live on the Rust side** and communicate with the React UI via Tauri events. The React UI does not poll — it reacts to events pushed from the backend.

See `15-tauri-migration-notes.md` for detailed Tauri-specific implementation guidance.
