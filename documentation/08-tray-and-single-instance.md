# 08 — System Tray and Single Instance

## 1. System Tray Overview

PinIt is a tray application. It has no taskbar button. The only persistent visible element when the main window is closed is the tray icon. The tray icon is the entry point for all user interaction when the main window is hidden.

---

## 2. Tray Icon

### 2.1 Adding the icon

The tray icon is added using `Shell_NotifyIconW` with `NIM_ADD`. The icon data structure (`NOTIFYICONDATAW`) must be configured with:

- The HWND of the hidden helper window (to receive tray messages).
- A unique icon ID (use `1`).
- `NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP` flags.
- A custom message ID for tray events (e.g. `WM_APP + 1`).
- The PinIt application icon (loaded from the embedded resource).
- A tooltip string: `"PinIt"`.

After adding, call `Shell_NotifyIconW` with `NIM_SETVERSION` and `NOTIFYICON_VERSION_4`. This version enables the improved message format where `lParam` contains the event type and `wParam` contains the anchor point (used for positioning the context menu).

### 2.2 Removing the icon

Call `Shell_NotifyIconW` with `NIM_DELETE` before the app exits. If this is not done, Windows will leave a ghost icon in the tray until the user hovers over it.

### 2.3 Explorer restart recovery

When Windows Explorer crashes and restarts (or is restarted by the user), all tray icons are lost. Windows broadcasts a registered message called `TaskbarCreated` when Explorer's taskbar is re-created.

To handle this:
1. On startup, call `RegisterWindowMessageW("TaskbarCreated")` and store the returned message ID.
2. In the helper window's message loop, handle this message by calling `Shell_NotifyIconW(NIM_ADD, ...)` again to re-add the icon.

Without this handling, PinIt's tray icon disappears permanently if Explorer is restarted.

---

## 3. Tray Events

With `NOTIFYICON_VERSION_4`, tray events arrive as the custom message (`WM_APP + 1`). When this message is received:
- `LOWORD(lParam)` — the event type.
- `wParam` — the anchor point as a packed `x` (low word) and `y` (high word), used for menu positioning.

Relevant event types:

| Event | Value | Action |
|-------|-------|--------|
| `WM_CONTEXTMENU` | 0x007B | Show the context menu at the anchor point |
| `NIN_SELECT` | 0x0400 | User left-clicked (single) → toggle main window |
| `NIN_KEYSELECT` | 0x0401 | User activated with keyboard → toggle main window |

Double-click on the tray icon is not treated specially. Single click or keyboard activation is enough.

---

## 4. Tray Context Menu

The context menu appears when the user right-clicks the tray icon. It is built dynamically from the current state.

### 4.1 Menu structure

```
Show PinIt                          [always present]
─────────────────────────────────
[Window title of pin 1]     ✓       [one entry per live pin, checked]
[Window title of pin 2]     ✓
...
─────────────────────────────────   [separator, only if there are pins]
Pin a window…
Unpin all                           [greyed out if no active pins]
─────────────────────────────────
Quit PinIt
```

### 4.2 Menu item IDs

Assign stable ID ranges:

| Range | Purpose |
|-------|---------|
| 1 | Show PinIt |
| 2 | Pin a window… |
| 3 | Unpin all |
| 4 | Quit PinIt |
| 100 + i | The i-th live pin entry |

### 4.3 Pin entry labels

Each live pin's menu entry shows the window title, truncated to 48 characters with an ellipsis if longer. This prevents very long titles (browser tabs, document names) from making the menu unreadably wide.

### 4.4 "Unpin all" greying

"Unpin all" is disabled (greyed out) if there are no active pins (live or pending). It is enabled if there is at least one.

### 4.5 Showing the menu

Use `TrackPopupMenuEx` (Win32) to show the menu at the anchor point from the tray event. Before calling it, call `SetForegroundWindow(helperHwnd)` — this is required by Windows for tray menus to dismiss correctly when the user clicks elsewhere.

After `TrackPopupMenuEx` returns, post `WM_NULL` to the helper window to flush the message queue.

### 4.6 Handling menu selections

- **Show PinIt** → show and focus the main window.
- **Pin entry (ID ≥ 100)** → unpin that window (clicking a checked pin toggles it off).
- **Pin a window…** → show main window, open the window picker.
- **Unpin all** → unpin all live pins, clear pending list.
- **Quit PinIt** → perform the manual quit procedure (see `10-session-and-restore.md`).

---

## 5. Balloon Notifications

Balloon notifications (Windows toast-style popups from the tray icon) are used in two situations:

1. **First close to tray**: when the user closes the PinIt window for the first time, show: "PinIt is still running. Your pins stay active. Find PinIt in the tray to open it again." This is shown once and never again (`has_seen_tray_notice` is set to `true`).

2. **Background pin/unpin feedback**: if `show_notifications` is `true` in settings, show a notification when a window is pinned or unpinned while the PinIt window is not visible.

### 5.1 Sending a notification

Use `Shell_NotifyIconW` with `NIM_MODIFY` and `NIF_INFO` flag set, with the `NOTIFYICONDATAW` fields:
- `szInfoTitle` — the notification title (e.g. `"PinIt"`).
- `szInfo` — the notification body text.
- `dwInfoFlags` — `NIIF_NOSOUND | NIIF_USER` (use the app icon, no system sound).

### 5.2 No sound on notifications

Notifications never play a system sound (`NIIF_NOSOUND`). PinIt plays its own sound independently for pin/unpin actions.

---

## 6. Main Window Toggle

The main window is shown/hidden rather than opened/closed. When the user closes it via the close button or the `Win+Ctrl+P` hotkey, the window is hidden (`ShowWindow(SW_HIDE)`), not destroyed.

### 6.1 First time hiding

If `has_seen_tray_notice` is `false`:
1. Hide the window.
2. Set `has_seen_tray_notice = true`.
3. Save settings immediately.
4. Show a balloon notification explaining the tray.

### 6.2 Showing the window

When bringing the window back:
1. Show the window (`ShowWindow(SW_SHOW)` or `SW_RESTORE`).
2. Bring it to the foreground. On Windows, `SetForegroundWindow` requires the calling process to be the foreground process or to have permission. The reliable way is:
   - Call `AllowSetForegroundWindow(ASFW_ANY)` before posting the show request (done when another instance requests us to show).
   - Or use `AttachThreadInput` + `BringWindowToTop` + `SetForegroundWindow`.
   - In Tauri, the `window.set_focus()` method handles this.

---

## 7. Single-Instance Enforcement

Only one copy of PinIt should run at a time per user session. If the user launches PinIt and it is already running, the new instance should bring the existing instance's window to the front and then exit.

### 7.1 Mutex-based detection

On startup (before doing anything else):

1. Call `CreateMutexW(NULL, FALSE, "PinIt_SingleInstance_v2")`.
2. Immediately check `GetLastError()`.
   - If `GetLastError() == ERROR_ALREADY_EXISTS` → another instance is running.
   - Otherwise → this is the first instance. Keep the mutex handle alive for the process lifetime (never close it).

The mutex name `"PinIt_SingleInstance_v2"` is the same name used as `AppMutex` in the Inno Setup installer script, so the installer can detect a running PinIt and prompt the user to close it before installing.

In dev mode, append `"-dev"` to the mutex name to avoid conflicting with an installed release copy.

### 7.2 Signalling the existing instance

When `ERROR_ALREADY_EXISTS` is detected:

1. Find the existing instance's hidden helper window by class name using `FindWindowW`.
   - The helper window class name is `"PinItHost"` (or `"PinItHost-dev"` in dev mode).
2. Call `AllowSetForegroundWindow(pid)` where `pid` is the PID of the existing instance's window. This grants the existing instance permission to bring itself to the foreground.
3. Post `WM_APP + 2` (the "show request" custom message) to the existing instance's helper window.
4. Exit this new instance.

### 7.3 Handling the show request

The existing instance's helper window receives `WM_APP + 2` and shows its main window.

### 7.4 If the existing window cannot be found

If `FindWindowW` returns NULL (the mutex exists but the window is not found), the previous instance may have crashed without releasing the mutex. In this case, proceed as if this is the first instance. The mutex from the crashed instance will be released by Windows automatically.

---

## 8. Cursor Position for Menu

When the context menu needs to appear at a specific position (from a right-click on the tray icon), the anchor point is provided in `wParam` of the tray message. However, as a fallback when the coordinates are both zero, use `GetCursorPos()` to get the current mouse position.

This covers cases where the menu is triggered from a keyboard shortcut (no mouse position available from the tray event).
