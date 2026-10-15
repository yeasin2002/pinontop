# 10 — Session Handling and Pin Restoration

## 1. Overview

Session handling covers two scenarios that are central to PinIt's core promise of persistence:

1. **Startup restoration** — re-pinning windows that were pinned in a previous session.
2. **Session end** — handling Windows shutdown/logoff/restart gracefully so that pins survive the reboot.

Getting these right is what distinguishes PinIt from a simple "set topmost" utility.

---

## 2. Startup Restoration

When PinIt starts, it loads `pinned.json` and gets a list of saved pins. These are descriptions of windows (process name + title + settings), not live window handles. The restoration procedure matches saved pins to currently-open windows.

### 2.1 The restoration procedure

1. Load saved pins from `pinned.json`.
2. Enumerate all currently-open pinnable windows (see `04-window-pinning.md` section 10).
3. For each saved pin, search the enumerated window list for a match:
   - Process name matches (case-insensitive).
   - Window title matches exactly.
   - The window is not already pinned (a saved pin appearing twice should only pin one window).
4. For each saved pin that found a match:
   - Apply the saved pin to the live window: pin it, apply opacity, apply click-through.
   - Record the prior state flags using the saved `was_*` values (with an AND to handle the case where the window was previously modified by PinIt and is still topmost from the last session).
   - Add to the live pin list.
5. For each saved pin that did NOT find a match:
   - Add it to the pending pin list.
   - Show as "Waiting for [process name]" in the UI.

### 2.2 Applying a saved pin

When applying a saved pin to a live window (both at startup and when a pending pin matches):

1. Call the normal pin procedure (see `04-window-pinning.md` section 6) to set `WS_EX_TOPMOST`.
2. Correct the `was_*` flags: after pinning, the live entry records the current window state as the prior state. But if the window is still topmost/layered/click-through from PinIt's previous run (after a crash), those would be recorded as the prior state incorrectly. To fix this:
   - `was_layered` = `was_layered (just recorded) AND saved.was_layered`
   - `was_topmost` = `was_topmost (just recorded) AND saved.was_topmost`
   - `was_click_through` = `was_click_through (just recorded) AND saved.was_click_through`
   The AND operation means: only consider the flag as "original state" if BOTH the current window has it AND the saved record says it was original. After a crash, the window may still be topmost (from our previous run), but `saved.was_topmost = false` tells us the user's app didn't originally have that flag.
3. If `saved.opacity < 255` → call `setOpacity(hwnd, alphaToPercent(saved.opacity))`.
4. If `saved.click_through` → call `setClickThrough(hwnd, true)`.

### 2.3 After restoration

After the restoration loop:
- Save the current state to `pinned.json` immediately. This updates the stored titles to their current values.
- Notify the UI with the current lists.
- Start the re-enforce timer and foreground hook (they run only while there are active pins).

---

## 3. Pending Pin Restoration

Pending pins are applied continuously while the app runs, not just at startup. This is part of the re-enforce loop (see `04-window-pinning.md` section 8).

### 3.1 The matching strategy

Only the current foreground window is checked against the pending list on each re-enforce tick. The reason: when an app opens for the first time, it typically becomes the foreground window. Checking only the foreground window is efficient (no enumeration cost) and catches the right moment.

The procedure on each tick:

1. Get the foreground HWND.
2. If it is null, or already pinned, or not pinnable → skip.
3. Get the title and process name of the foreground window.
4. Search the pending list for a match (process + title).
5. If a match is found:
   - Apply the saved pin (see section 2.2 above).
   - Remove the entry from the pending list.
   - Save to `pinned.json`.
   - Notify the UI (the "Waiting" row should become a live row).
   - Return. Only one pending pin is applied per tick.

### 3.2 Why only one per tick

Processing only one pending pin per foreground change prevents accidentally pinning multiple windows in a single moment if several apps happen to match. The user gets a flash confirmation for each pin, and each one is clearly visible.

---

## 4. Session End: Windows Shutting Down

Windows sends a session-end notification when the user shuts down, restarts, or logs off. PinIt must handle this correctly.

### 4.1 Why this requires a non-message-only window

The session-end messages (`WM_QUERYENDSESSION` and `WM_ENDSESSION`) are broadcast messages — Windows sends them to all top-level windows. However, they are NOT sent to message-only windows (windows created with `HWND_MESSAGE` as the parent). PinIt's hidden helper window must therefore be a regular top-level window (with `WS_OVERLAPPED` style and no parent), even though it has no visible content.

### 4.2 WM_QUERYENDSESSION

When Windows is about to end the session, it sends `WM_QUERYENDSESSION` to all top-level windows and waits for responses.

PinIt's handler:
1. Set the `session_ending` flag to `true` in the app state.
2. Notify the application logic immediately (not via the event loop — Windows will not wait for the message loop to process it).
3. Return `TRUE` (LRESULT 1). This tells Windows that PinIt agrees to end the session. Never return `FALSE` here — blocking the shutdown indefinitely is the wrong behaviour.

Setting `session_ending = true` tells the re-enforce loop to skip removing closed windows. During shutdown, Windows closes applications one by one. If PinIt were to treat those closures as "the user closed this window", it would remove pins from the file — which is exactly what must NOT happen before a restart.

### 4.3 WM_ENDSESSION

After all applications have responded to `WM_QUERYENDSESSION`, Windows sends `WM_ENDSESSION`. The `wParam` value indicates whether the session is actually ending:
- `wParam != 0` → the session IS ending (shutdown/restart/logoff confirmed).
- `wParam == 0` → the session end was cancelled (e.g. another app blocked it). In this case, set `session_ending = false` and resume normal operation.

When `wParam != 0` (session really ending):

1. Flush any pending debounced write: save `pinned.json` immediately with the current state. **Do not clear the pin list from the file.** The pins must survive the reboot.
2. Release all pinned windows visually: restore their original state (topmost, opacity, click-through). This is a courtesy — Windows is about to close them anyway, but restoring them before they close is the clean behaviour.
3. Remove the tray icon.
4. Return LRESULT 0 promptly. Do not do time-consuming work here.

Note: after `WM_ENDSESSION` with `wParam != 0`, the process may be terminated at any time. Keep the handler fast.

### 4.4 The critical distinction: session end vs manual quit

| Scenario | Clear pins from file? | Release windows visually? |
|----------|----------------------|--------------------------|
| Session ending (reboot/shutdown) | **NO** — pins must survive reboot | Yes (visual cleanup) |
| Manual quit (user chose "Quit PinIt") | **YES** — start clean next time | Yes |

---

## 5. Manual Quit

When the user selects "Quit PinIt" from the tray menu:

1. Release all pinned windows (restore their original state).
2. Log: "Restored and cleared [n] pinned window(s) on manual quit".
3. Clear the pin list from `pinned.json` (write the file with an empty pin array, but keep settings).
4. Remove the tray icon.
5. Exit the application.

The rationale for clearing pins on manual quit: if the user quits PinIt, they are done using it for now. When they relaunch it later, they want a clean slate — their apps will not be in the same state they were in now, so re-pinning the same windows would be unexpected.

---

## 6. What Survives a Reboot

After a reboot (session end handled correctly):

- `pinned.json` contains the full pin list with all saved pins (live + pending at the time of shutdown).
- On next login:
  - PinIt starts (manually or via autostart).
  - The restoration procedure runs.
  - Windows that are already open get pinned immediately.
  - Windows that are not yet open appear as "Waiting for [app]" until they open.

This means if the user had 5 windows pinned when they restarted, all 5 will be re-pinned once those applications open. The user never has to manually re-pin anything.

---

## 7. Crash Recovery

If PinIt crashes without running its shutdown procedure:

- `pinned.json` still has the pin list from the last successful save (which includes any pins applied before the crash).
- The pinned windows may still be topmost/layered/click-through from PinIt's previous changes.
- On next start, the restoration procedure runs.
- The `was_*` flag correction logic (section 2.2) handles the case where a window is already in PinIt's desired state from the previous run.

The practical result: after a crash and restart, windows that were pinned are re-pinned correctly without double-applying effects.
