# 12 — UI Screens and Components

## 1. Overview

PinIt has one main window with a fixed width (~380 logical pixels). It contains a primary view (the pin list) and four slide-up panels (sheets) that appear on demand: the window picker, the shortcut editor, the settings panel, and the about panel. There are no separate dialogs or secondary windows — everything lives inside this one window.

The window is not resizable vertically by the user (its height expands to fit its content up to a maximum, then a scroll area handles overflow). Width is fixed.

---

## 2. Main Window — Pin List View

This is the default view. It is visible whenever none of the four sheets are open.

### 2.1 Header

A narrow header strip at the top of the window:
- App logo / wordmark on the left.
- A settings button (gear icon) on the right that opens the Settings sheet.
- Optionally a notification badge if there are hotkey conflicts.

### 2.2 Hotkey conflict warning

If any hotkey failed to register at startup (another app owns the keys), a yellow warning banner appears below the header:

> ⚠ Some shortcuts are unavailable. Open Shortcuts to reassign them.

Clicking the banner opens the shortcut editor. The banner disappears if there are no conflicts.

### 2.3 Pin list

A scrollable list of pinned windows. Each row represents one pin. The list is empty-stated with a hint message when nothing is pinned.

**Empty state:**
> No windows pinned yet.
> Press Win + Ctrl + T while a window is focused to pin it,
> or click "Pin a window" below.

**Each pin row contains** (left to right):
1. **App avatar** — a 32×32 pixel square with rounded corners showing the window's icon. If no icon is available, shows a coloured circle with the first letter of the process name (initial avatar). Live pins show the avatar at full opacity; pending pins show it at ~55% opacity.
2. **Title and process** — two lines of text. The top line is the window title (bold, truncated with ellipsis). The bottom line is the process name (muted colour, smaller). For pending pins, the bottom line reads "Waiting for [process name]".
3. **Opacity slider** — only for live pins. A horizontal slider ranging from 20 to 100. Shows the current opacity value as a percentage label to the right of the slider. Hidden for pending pins.
4. **Click-through button** — only for live pins. A small icon button (cursor/pointer icon). Active (highlighted) when click-through is on; inactive when off. Has a tooltip: "Clicks pass through (on)" or "Clicks pass through (off)". Hidden for pending pins.
5. **Unpin button** — a small "×" or unpin icon button on the far right. For pending pins it removes the pending entry. For live pins it unpins the window.

**Row hover behaviour:**
- When the mouse enters a row, the row gets a subtle lift/highlight effect.
- If the row is a live pin, the flash overlay (hover highlight) appears on the corresponding live window.
- When the mouse leaves, the flash overlay disappears.

**Row appearance animation:**
- When a new row is added to the list, it fades in from transparent to opaque over the standard animation duration.

### 2.4 Footer

A persistent button bar at the bottom of the pin list view:
- **"Pin a window"** button — opens the Window Picker sheet.
- **"Unpin all"** button — unpins all live pins and clears the pending list. Only active when there is at least one pin (live or pending).

---

## 3. Window Picker Sheet

A slide-up panel that covers the pin list. Opens when the user clicks "Pin a window" or when requested from the tray menu.

### 3.1 Header

- Title: "Pin a window"
- A close/dismiss button to go back to the pin list.

### 3.2 Search field

A text input at the top of the picker. Typing filters the window list in real time. Filtering is case-insensitive and matches against both the window title and the process name.

### 3.3 Window list

A scrollable list of all pinnable windows currently open on the system. The list is populated fresh each time the picker is opened (not cached).

**Each row contains:**
1. App avatar (same style as the pin list — icon if available, initial avatar as fallback).
2. Window title (bold, truncated).
3. Process name (muted, smaller).
4. A visual indicator if the window is already pinned (e.g. a check mark or highlighted row).

Clicking any row:
- If the window is NOT pinned → pin it, close the picker, show the flash overlay, play sound.
- If the window IS pinned → unpin it, close the picker, show the flash overlay.

### 3.4 Empty state

If no pinnable windows are found (unlikely but possible):
> No windows found. Make sure other apps are open.

If the search filter produces no matches:
> No matches for "[filter text]".

---

## 4. Shortcut Editor Sheet

A slide-up panel for customising the four global hotkeys.

### 4.1 Header

- Title: "Shortcuts"
- A close/dismiss button (cancels any unsaved changes).

### 4.2 Shortcut rows

Four rows, one per action. Each row:
- Label: the action name (e.g. "Pin / unpin window").
- Modifier chips: four toggleable chips labelled "Win", "Ctrl", "Alt", "Shift". Each chip is a toggle button. Clicking it enables/disables that modifier.
- Key selector: a dropdown or selector showing the current key. Contains all available keys from the key labels list.
- Unavailability badge: if this shortcut is currently unavailable (Windows refused the registration), show a warning indicator on the row.

The modifier chips and key selector together represent the current edited state of the shortcut. Changes are not applied until the user saves.

### 4.3 Validation

Before saving, validate each shortcut:
- Must have at least one safe modifier (Win, Ctrl, or Alt).
- If not → show an inline warning on that row: "Add Win, Ctrl, or Alt to avoid interfering with typing."

Invalid shortcuts block saving.

### 4.4 Footer buttons

- **"Save"** (primary button) — applies and saves all four shortcuts. If any registration fails, the sheet stays open and shows which ones failed.
- **"Restore defaults"** — resets all four shortcuts to their factory defaults and applies them immediately.
- **"Cancel"** / dismiss — closes without saving, reverts the displayed values to the current saved state.

### 4.5 Showing unavailable shortcuts

After saving, if any hotkey registration failed (another app owns those keys), show a warning banner inside the sheet:
> Some shortcuts couldn't be registered. Another app may be using those keys.

The affected rows are individually marked so the user knows which ones to change.

---

## 5. Settings Sheet

A slide-up panel for all user preferences.

### 5.1 Header

- Title: "Settings"
- Close button.

### 5.2 Settings items

Each item is a row with a label, optional description, and a toggle switch or button.

**Sound**
- Label: "Play sound on pin/unpin"
- Control: toggle switch (on by default).

**Notifications**
- Label: "Show a notification when pinning while the window is hidden"
- Description: "Helpful if you pin windows via hotkey without opening PinIt."
- Control: toggle switch (off by default).

**Start with Windows**
- Label: "Start PinIt with Windows"
- Description: "Runs in the tray at login."
- Control: toggle switch. Reads its initial state from the registry (not from `pinned.json`). When toggled, writes to the registry immediately.

**Shortcuts**
- Label: "Keyboard shortcuts"
- Control: a button "Edit shortcuts →" that opens the Shortcut Editor sheet.
- If any shortcuts are unavailable, show a warning badge here too.

### 5.3 About section

At the bottom of the settings sheet (or as a separate "About" area):
- App name and version number.
- A link to the GitHub repository / website.
- Credit for Slint (in the original app) — in the Tauri rebuild, equivalent credits for any frameworks used.
- License: "Apache 2.0".

---

## 6. Toast / Status Message

A transient message that appears inside the main window (not as a system notification) when an action occurs while the window is open.

**Appearance:**
- A pill-shaped banner that slides in at the bottom of the window or at the top of the pin list.
- Dark background with light text (inverted from the window background).
- Auto-dismisses after approximately 3.2 seconds.
- If a new message comes in while one is showing, replace it immediately.

**Used for:**
- "Can't pin [app] — it may be running as administrator."
- "No window to pin — click a window first."
- "Pin this window first to change its opacity."
- "Clicks now pass through this window."
- "Clicks reach this window again."
- "Couldn't change the startup setting."

---

## 7. Component Inventory

These are the reusable UI components used across all screens.

### 7.1 Button

Three variants:
- **Primary** — filled with the accent colour, white text, bold. Used for the main action in a sheet (e.g. "Save").
- **Ghost** — outline border, text in default colour. Used for secondary actions.
- **Danger ghost** — outline border, text in a muted colour; border and text turn a danger-red on hover. Used for destructive actions like "Unpin".

States: default, hover, focus (focus ring visible for keyboard navigation), disabled (reduced opacity).

### 7.2 Toggle switch (CheckBox)

A pill-shaped toggle. On = accent fill. Off = grey fill. Has an animated thumb that slides between positions. Keyboard-accessible (Space to toggle).

### 7.3 Slider

A horizontal range slider. Has:
- A filled track (from the minimum to the thumb position) in the accent colour.
- An unfilled track from the thumb to the maximum.
- A draggable thumb.
- Keyboard support: left/right arrows to change value by 1, Page Up/Down to change by larger steps.
- A visible label indicating the current value (shown outside the slider, not on the thumb).

### 7.4 App Avatar

A 32×32 square with rounded corners (6 px radius). Shows either:
- The application icon as a bitmap image, if available.
- A coloured background with the first letter of the process name, if no icon is available.

The background colour for the initial variant is chosen from a palette of 6 pastel tones based on a hash of the process name. This ensures the same app always gets the same colour.

Pastel tones (hex RGB):
- `#D8C6F0` (lavender)
- `#FFE08A` (yellow)
- `#C3E4CF` (mint green)
- `#F6CAB8` (peach)
- `#BEDCF2` (sky blue)
- `#F2CFE2` (pink)

Select index = `hash(processName.toLower()) mod 6`.

### 7.5 Key combo display (KeyCombo)

Used to display a keyboard shortcut as a series of "key chips". Each chip is a small pill with a border, showing one modifier or key (e.g. "Win", "Ctrl", "T"). Chips are separated by a "+" label in a muted colour.

### 7.6 Card

A container with a white (light) or dark-grey (dark) background, a subtle border, and rounded corners (12 px radius). Used to visually group pin rows.

### 7.7 Sheet

A panel that slides up from the bottom of the window to cover the pin list. Has:
- A white/dark background.
- A subtle top border.
- Slides in from below over the standard animation duration (~200 ms ease-out).
- Slides out downward when dismissed.
- Only one sheet is open at a time.

### 7.8 Section Title

A small-caps label in a muted colour, used to separate groups of settings items. Example: "APPEARANCE", "BEHAVIOUR".

### 7.9 Elided text

Any text that may overflow its container is truncated with a trailing "…". This applies to window titles and process names throughout.

---

## 8. Window Behaviour

### 8.1 Fixed width

The window has a fixed width of approximately 380 logical pixels. It cannot be resized horizontally.

### 8.2 Height

The window height adapts to content up to a maximum (approximately 600 logical pixels). Above that, the pin list becomes scrollable. The window itself does not grow beyond the maximum.

### 8.3 Close button

Clicking the window's close button hides the window to the tray (does not quit). See `08-tray-and-single-instance.md` section 6 for the first-time notice.

### 8.4 Title bar styling

On Windows 11, the title bar colour matches the window's background colour so that the title bar does not contrast with the content. This uses `DwmSetWindowAttribute(DWMWA_CAPTION_COLOR, ...)`. On Windows 10, this API is not available; the default title bar colour is used.

Dark mode: the title bar also uses the immersive dark mode flag `DWMWA_USE_IMMERSIVE_DARK_MODE` so the system draws it in dark style.

### 8.5 Maximize button

The window has a fixed width; it should not be maximisable. In practice, Windows still shows the maximize button and it is a known gap in the original implementation. The correct fix is to remove the maximize button from the window style, which can be done by removing `WS_MAXIMIZEBOX` from the window style via `SetWindowLongW`.

### 8.6 No taskbar button

The main window should not appear in the taskbar. In Win32 this is achieved by creating the window with the `WS_EX_TOOLWINDOW` extended style. In Tauri, this is configurable via the window configuration or by calling `SetWindowLongW` after window creation.

---

## 9. Accessibility

All interactive elements must be keyboard-accessible:
- Tab moves between focusable elements.
- Enter/Space activates buttons and toggles.
- Sliders respond to arrow keys.
- All interactive elements have visible focus indicators.
- Buttons and toggles have accessible labels (for screen readers).
- The pin list rows are navigable by keyboard.
- The shortcut editor modifier chips are toggle buttons with proper role and checked state.
