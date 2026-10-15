# 01 — Project Overview

## 1. What PinIt Is

PinIt is a small Windows 10/11 utility that solves one problem: Windows has no built-in way to keep a specific application window permanently visible on top of all others. PinIt fills that gap. The user presses a hotkey while a window is focused and that window stays on top of every other window until the user unpins it.

The product is intentionally minimal. It does not manage workspaces, virtual desktops, window sizes, or anything else. Its value is in doing one thing completely and reliably.

---

## 2. The Core User Experience

The entire experience is designed around three principles:

1. **Zero friction** — the most common action (pin a window) requires no mouse interaction, no opening the app, and no searching for a menu item. One hotkey does it.
2. **Stays out of the way** — PinIt lives in the system tray and does not appear in the taskbar. The user only sees it when they want to.
3. **Survives everything** — pins are remembered across app restarts, Windows restarts, and crashes. When the user logs back in, their windows are re-pinned automatically.

---

## 3. Feature List

### 3.1 Global Hotkey Pinning

- The user presses `Win + Ctrl + T` while any window is focused.
- That window is immediately set as always-on-top.
- A coloured rectangular outline briefly appears around the window confirming the action (see section 3.6).
- Pressing the same hotkey again on an already-pinned window unpins it, restoring it to its original behaviour.
- The hotkey works regardless of what application is focused. The user does not need to click into PinIt first.

### 3.2 Per-Window Opacity

- Any pinned window can have its transparency adjusted independently.
- The opacity range is 20% (nearly invisible) to 100% (fully opaque).
- Two hotkeys control opacity: `Win + Ctrl + =` raises it by 10 percentage points, `Win + Ctrl + -` lowers it by 10.
- The same sliders are available in the main PinIt window for precise control.
- Opacity is stored per window and restored automatically on the next login.
- When a window is unpinned, its opacity is restored to whatever it was before PinIt changed it. If the application itself manages its own transparency, PinIt respects that and does not interfere.

### 3.3 Click-Through Mode

- Any pinned window can be put in click-through mode, which means mouse clicks pass through the window to whatever is behind it.
- This is useful for keeping a reference document or video visible while working in the app behind it.
- Click-through requires the window to be layered (have an alpha value). If a user enables click-through on a fully-opaque window, the app automatically applies a small transparency value to make it work.
- The setting is toggled per window from the UI.
- It is saved and restored across restarts.

### 3.4 Persistent Pins

- Every pinned window is written to `pinned.json` immediately.
- When PinIt starts (including after a Windows reboot), it reads that file and re-pins every window that is already open.
- For windows that are not yet open at startup time, PinIt enters a waiting state. It watches for the foreground window to change and checks whether the newly-focused window matches a saved pin. The moment that application opens and becomes the foreground window, PinIt pins it automatically.
- The main window shows "Waiting for [process name]" for these pending pins, so the user can see and manage them.

### 3.5 Windows 11 Topmost Re-enforcement

- Windows 11's compositor can silently strip the always-on-top flag from a window under certain conditions (DWM transitions, focus changes, full-screen games, etc.).
- PinIt re-applies the flag automatically by listening to foreground-change events and also running a background timer every two seconds.
- This is invisible to the user. Their windows simply stay pinned.

### 3.6 Flash Overlay

- When a window is pinned or unpinned, a coloured rectangular outline is briefly drawn around it.
- Orange outline = window was just pinned.
- Grey outline = window was just unpinned.
- The outline matches the visible frame of the window precisely (not the invisible resize border that Windows 10/11 adds around all windows).
- The outline holds for 300 milliseconds then fades out over 250 milliseconds.
- When hovering over a pinned window's row in the UI, a persistent orange outline is shown on the live window so the user can identify which window corresponds to which row.

### 3.7 System Tray

- PinIt lives in the system tray. There is no taskbar button.
- Clicking the tray icon opens or closes the main window.
- Right-clicking shows a context menu with:
  - "Show PinIt" — opens the main window
  - A list of every currently-pinned window (clicking one unpins it)
  - "Pin a window…" — opens the window picker
  - "Unpin all" — removes all pins at once
  - "Quit PinIt" — exits cleanly, restoring all windows to their original state
- When PinIt is closed via the window's close button, it goes to the tray rather than quitting. The first time this happens a balloon notification explains this.

### 3.8 Window Picker

- The user can pin a window from the main PinIt UI without using the hotkey.
- Clicking "Pin a window…" opens a picker panel listing every visible, pinnable window currently open on the system.
- Each entry shows the window title, process name, and the application icon.
- The list is filterable by typing.
- Clicking an entry pins or unpins that window.

### 3.9 Shortcut Customisation

- All four default hotkeys can be changed from the Settings area.
- The four actions are: Toggle pin, Increase opacity, Decrease opacity, Show/hide PinIt.
- Each shortcut requires at least one of Win, Ctrl, or Alt as a modifier (Shift-only or bare key shortcuts are rejected as they would interfere with normal typing).
- If a requested shortcut is already held by another application, PinIt shows a warning in the editor.
- Custom shortcuts are saved to `pinned.json` and survive restarts.

### 3.10 Start with Windows

- A toggle in settings writes a value to `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` so PinIt starts automatically on login.
- When started this way, PinIt launches directly into the tray (hidden window) using the `--minimized` flag.
- The Inno Setup installer also offers this as an optional task during installation.

### 3.11 Sound Feedback

- A soft "tick" sound plays when a window is pinned or unpinned.
- This can be disabled in settings.
- The sound is a bundled WAV file, not a system sound. It is played asynchronously so it never delays the pin action.

### 3.12 Notification Feedback

- A Windows balloon notification can be shown when a window is pinned/unpinned and the PinIt window is not visible.
- This is off by default and can be enabled in settings.

### 3.13 Light and Dark Theme

- PinIt follows the Windows system colour mode automatically.
- If the user switches between light and dark mode in Windows Settings, PinIt updates immediately without a restart.
- The palette is a warm "paper" tone — not the generic blue/grey found in most Windows apps.

---

## 4. What PinIt Does NOT Do

Understanding the intentional limits is as important as the feature list.

- It does not pin windows to specific monitors or positions.
- It does not resize, move, or snap windows.
- It does not manage multiple desktops or virtual workspaces.
- It does not remember the window's position or size.
- It does not pin windows from elevated (administrator) processes when PinIt itself is running at normal privilege. This is a Windows security limitation (UIPI). The workaround is to run PinIt as administrator, which is documented in the FAQ.
- It cannot pin its own windows.
- It cannot pin the Windows desktop, taskbar, Start menu, or other shell windows.

---

## 5. Default Keyboard Shortcuts

| Action | Default Shortcut |
|--------|-----------------|
| Pin / unpin the focused window | `Win + Ctrl + T` |
| Increase opacity by 10% | `Win + Ctrl + =` |
| Decrease opacity by 10% | `Win + Ctrl + -` |
| Show / hide the PinIt window | `Win + Ctrl + P` |

All four are fully customisable.

---

## 6. Installer and Distribution

- The app ships as a single self-contained executable. No runtime is required.
- Two distribution formats:
  - An Inno Setup installer that creates a Start Menu shortcut and an uninstaller.
  - A portable ZIP that can be extracted and run from anywhere.
- Per-user install — no administrator prompt.
- The installer handles upgrading from previous versions cleanly.

---

## 7. Target User

The target user is a developer, writer, or knowledge worker who regularly needs one or two windows visible at all times while working in other applications — for example, a documentation page alongside a code editor, or a video call window alongside a spreadsheet. They want to set it and forget it. They do not want a utility suite, a configuration wizard, or an app that requires ongoing attention.
