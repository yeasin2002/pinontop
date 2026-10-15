# 04 — Window Pinning

## 1. What "Pinning" Means at the Win32 Level

Windows has a window style flag called `WS_EX_TOPMOST` (part of the extended window style, `GWL_EXSTYLE`). When this flag is set on a window, that window is always drawn above all windows that do not have the flag. Multiple topmost windows stack among themselves by their normal Z-order.

Pinning a window in PinIt means setting this flag. Unpinning means removing it.

The Win32 function used is `SetWindowPos`. To make a window topmost:

```
SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)
```

To remove topmost:

```
SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)
```

`SWP_NOMOVE | SWP_NOSIZE` tells Windows to only change the Z-order, not the position or size. `SWP_NOACTIVATE` prevents the window from stealing focus.

---

## 2. Recording Prior State

Before pinning, PinIt records three things about the window:

1. **`was_topmost`** — was the window already always-on-top before PinIt touched it? Some applications (Task Manager's "Always on top" mode, many media players, some game overlays) manage their own topmost state. If `was_topmost` is `true`, PinIt must not remove the topmost flag when unpinning — doing so would change the window's behaviour in a way the user did not request.

2. **`was_layered`** — did the window already have `WS_EX_LAYERED`? Some applications manage their own transparency. If `was_layered` is `true`, PinIt must not remove the layered style when unpinning (and must take care when applying opacity).

3. **`was_click_through`** — did the window already have `WS_EX_TRANSPARENT`? Same reasoning.

How to read these flags:
- `GetWindowLongW(hwnd, GWL_EXSTYLE)` returns the extended style bitmask.
- Check `result & WS_EX_TOPMOST` for topmost.
- Check `result & WS_EX_LAYERED` for layered.
- Check `result & WS_EX_TRANSPARENT` for click-through.

---

## 3. The UIPI Problem

User Interface Privilege Isolation (UIPI) is a Windows security feature. A process running at normal integrity level cannot send messages to — or modify the window styles of — a process running at a higher integrity level (such as administrator).

This means: if the user tries to pin a window belonging to an elevated process (e.g. Task Manager running as administrator, or an installer), and PinIt itself is running at normal privilege, the `SetWindowPos` call will appear to succeed (it returns a success code) but the change will silently not take effect.

### 3.1 How to detect UIPI failure

After calling `SetWindowPos(hwnd, HWND_TOPMOST, ...)`, call `GetWindowLongW(hwnd, GWL_EXSTYLE)` and check whether `WS_EX_TOPMOST` is now set. If the call appeared to succeed but the flag is not set, UIPI blocked it.

This check is the canonical detection method. Do not rely on the return value of `SetWindowPos` alone.

### 3.2 What to show the user

When UIPI is detected:
- Do not add the window to the pin list.
- Show an error: "Can't pin [process name]. It may be running as administrator."
- Log a warning with the process name (never the window title).

### 3.3 Workaround

Running PinIt as administrator resolves UIPI. This should be documented in the FAQ but not automated — automatically elevating the entire app to administrator is a security risk for a persistent background process.

---

## 4. Windows That Cannot Be Pinned

Several categories of windows must be rejected before attempting to pin:

### 4.1 Invalid or closed windows

Check `IsWindow(hwnd)`. If it returns false, the HWND is no longer valid.

### 4.2 PinIt's own windows

Compare the process ID of the target window against PinIt's own process ID (from `GetCurrentProcessId()`). If they match, reject.

How to get the process ID of a window:
```
GetWindowThreadProcessId(hwnd, &pid)
```

### 4.3 Shell windows (Windows UI chrome)

The Windows shell runs several always-on-screen windows that must never be pinned. Pinning the desktop or taskbar would break the Windows UI.

The check is by window class name, retrieved via `GetClassNameW(hwnd, buffer, size)`.

The full list of shell class names to reject:

| Class Name | What it is |
|-----------|-----------|
| `Progman` | The desktop |
| `WorkerW` | The desktop's wallpaper host |
| `Shell_TrayWnd` | The taskbar |
| `Shell_SecondaryTrayWnd` | The taskbar on secondary monitors |
| `NotifyIconOverflowWindow` | The system tray overflow (Windows 10) |
| `TopLevelWindowForOverflowXamlIsland` | The system tray overflow (Windows 11) |
| `Windows.UI.Core.CoreWindow` | Start menu, Search, Action Center |
| `XamlExplorerHostIslandWindow` | Task View / Alt+Tab (Windows 11) |
| `MultitaskingViewFrame` | Task View / Alt+Tab (Windows 10) |
| `TaskListThumbnailWnd` | Taskbar thumbnail previews |
| `ForegroundStaging` | Shell focus hand-off helper |

This list covers all known Windows 10 and 11 versions as of the app's development. New shell classes may appear in future Windows versions.

---

## 5. The `isPinnable` Function

Before pinning, every HWND must pass through an `is_pinnable` check. The function returns `true` only if all of the following are true:

1. `IsWindow(hwnd)` returns true.
2. The window's PID is not PinIt's own PID.
3. The window's class name is not in the shell class list.

This check is also used when enumerating windows for the window picker — any window that fails this check is excluded from the list.

---

## 6. Pinning a Window — Full Procedure

When the user requests to pin a window (via hotkey or UI):

1. Confirm the HWND is still valid (`IsWindow`).
2. Run the `is_pinnable` check.
3. Read the current extended style to record `was_topmost`, `was_layered`, `was_click_through`.
4. Call `SetWindowPos(hwnd, HWND_TOPMOST, ...)`.
5. Verify by reading the extended style again: is `WS_EX_TOPMOST` now set?
   - If not → UIPI failure → show error, abort.
6. Add the window to the live pin list with opacity=100, click_through = `was_click_through`.
7. Remove the window from the pending list if present.
8. Save to `pinned.json`.
9. Show the flash overlay (orange).
10. Play the tick sound (if enabled).
11. Send a "pins changed" event to the UI.

---

## 7. Unpinning a Window — Full Procedure

When the user requests to unpin a window:

1. Find the window in the live pin list.
2. Restore prior state:
   - If `click_through` is `true` AND `was_click_through` is `false` → remove `WS_EX_TRANSPARENT`.
   - If `opacity_changed` is `true` → restore the window's opacity (see `05-opacity-and-click-through.md`).
   - If `was_topmost` is `false` → remove `WS_EX_TOPMOST` via `SetWindowPos(hwnd, HWND_NOTOPMOST, ...)`.
3. Remove the window from the live pin list.
4. Save to `pinned.json`.
5. Show the flash overlay (grey).
6. Play the tick sound (if enabled).
7. Send a "pins changed" event to the UI.

If the window no longer exists when unpinning, skip step 2 (there is nothing to restore) but still do steps 3–7.

---

## 8. Topmost Re-enforcement

Windows 11's DWM compositor can silently strip `WS_EX_TOPMOST` from a window under several conditions:
- A full-screen Direct3D/DirectX application starts or stops.
- The user activates a window in a specific way.
- DWM recomposition occurs (e.g. when connecting a remote desktop session).
- Certain focus transitions in XAML-based Windows apps.

PinIt re-applies the flag by two mechanisms that run simultaneously:

### 8.1 Foreground-change event hook

Windows provides `SetWinEventHook` with `EVENT_SYSTEM_FOREGROUND`. This callback fires every time a different window becomes the foreground window. PinIt hooks this event and runs a re-enforce pass on every trigger.

Important: the hook must be registered as `WINEVENT_OUTOFCONTEXT`. This means Windows queues the event to the thread that registered the hook rather than injecting a DLL into the target process. Nothing is injected into foreign processes.

The hook is registered as `WINEVENT_SKIPOWNPROCESS` so PinIt's own focus changes do not trigger unnecessary passes.

### 8.2 Two-second periodic timer

A timer fires every 2000 milliseconds and runs the same re-enforce pass. This is a safety net for cases where the foreground hook does not fire (e.g. a full-screen game that does not change the foreground window before stripping topmost flags).

### 8.3 The re-enforce pass

On every tick (from either source):

1. Walk the live pin list.
2. For each entry:
   a. If `IsWindow(hwnd)` returns false → remove the entry from the list (window was closed).
   b. If `WS_EX_TOPMOST` is not set → call `SetWindowPos(hwnd, HWND_TOPMOST, ...)` to reapply it.
   c. Read the current window title. If it differs from the stored title and is not empty → update the stored title, mark as dirty.
3. Check the pending list against the current foreground window (see `06-persistence.md` for pending pin restoration).
4. If any windows were removed or titles changed → save to `pinned.json`, send "pins changed" to UI.

### 8.4 When the timer runs

The timer and foreground hook are only active while there is at least one entry in the live pin list or pending list. When both lists are empty, the timer is stopped and the hook is unregistered. This ensures PinIt uses zero CPU when idle.

---

## 9. Window Title Updates

Applications routinely update their window title. A browser shows the page title; a text editor shows the document name; a media player shows the currently playing track. The stored title for a pin must be kept current so that if the app is closed and reopened, the matching logic can find it again.

The re-enforce loop reads the current title on every tick and updates the stored title if it changed. This also updates the UI so the pin list always shows the current title.

One guard: if the new title is empty (some apps briefly set an empty title during transitions), do not update. Wait for a non-empty title.

---

## 10. Enumerating Windows for the Picker

The window picker needs a list of all pinnable windows. The procedure:

1. Call `EnumWindows` to walk all top-level windows.
2. For each window, apply these filters (skip if any fail):
   - `IsWindowVisible(hwnd)` must be true.
   - The extended style must NOT include `WS_EX_TOOLWINDOW` (tool windows are helper windows not meant to appear in task lists).
   - `DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, ...)` must return 0 or fail (cloaked windows are UWP apps running in the background, invisible to the user).
   - `is_pinnable(hwnd)` must return true.
3. For each surviving window, read the title. Skip if the title is empty.
4. Read the process name.
5. Try to extract the window's icon (see below).
6. Return the list.

### 10.1 Extracting window icons

To get a window's icon as an RGBA pixel buffer:
1. Send `WM_GETICON` with `ICON_BIG` using `SendMessageTimeoutW` with a 100-millisecond timeout. A timeout is mandatory — a hung window would otherwise block PinIt.
2. If that returns null, try `GetClassLongPtrW(hwnd, GCLP_HICON)` as a fallback.
3. If both fail, the window has no icon. The UI shows an avatar with the process initial instead.
4. If an icon handle is obtained, render it into a 32×32 RGBA pixel buffer.

Converting an HICON to RGBA pixels requires creating a compatible DC, selecting the icon into a DIB section, and reading the pixel data. Old-style icons (without an alpha channel) must have their mask applied separately to reconstruct the alpha channel.

---

## 11. Getting the Foreground Window

`GetForegroundWindow()` returns the HWND of the window the user is currently working in. This is the target for hotkey actions. Note:

- It can return NULL if no window is focused (e.g. the desktop is focused).
- It should always be checked before use.
- The value is only valid at the instant it is read. If the user switches windows between the hotkey press and the HWND being used, the value may refer to a different window.
