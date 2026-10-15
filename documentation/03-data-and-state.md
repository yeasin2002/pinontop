# 03 — Data and State

## 1. Overview

All persisted state lives in a single JSON file called `pinned.json`. Understanding this file's schema completely is the most important prerequisite for implementing persistence, restoration, and cross-version compatibility. This document covers the schema in full, how it is used at runtime, and every encoding convention that must be preserved exactly.

---

## 2. File Location

| Mode | Path |
|------|------|
| Normal | `%LOCALAPPDATA%\PinIt\pinned.json` |
| Dev mode (`PINIT_DEV=1`) | `%LOCALAPPDATA%\PinIt-dev\pinned.json` |

`%LOCALAPPDATA%` resolves to something like `C:\Users\<username>\AppData\Local`. On Windows this is always available. If for any reason it is missing, fall back to the system temp directory.

The directory must be created if it does not exist. The file must be created on first save.

---

## 3. Top-Level Schema

```
{
  "pins": { ... },
  "settings": { ... }
}
```

Both fields are always present in a saved file. Any additional top-level fields that appear (from future versions) must be preserved verbatim and written back on the next save. Never strip unknown fields.

---

## 4. The `pins` Object

`pins` is a JSON object — not an array. The keys are strings in the format `"<processName>:<index>"`.

Example:
```
"pins": {
  "notepad.exe:0": { ... },
  "chrome.exe:1": { ... },
  "notepad.exe:2": { ... }
}
```

### 4.1 Why an object, not an array?

This format originated in the original Tauri v1 build of PinIt. It has been kept unchanged through every subsequent rebuild for backwards compatibility. When loading, the key is parsed to extract the index (the number after the last colon), pins are sorted by that index to reconstruct the original order, and the keys themselves are discarded. When saving, keys are generated as `"<processName>:<i>"` where `i` is the pin's position in the array (0-based).

### 4.2 Pin entry fields

Each value in the `pins` object is an object with these fields:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `process_name` | string | `""` | The executable file name of the process that owns the window. Example: `"notepad.exe"`. Case-insensitive when matching. |
| `title` | string | `""` | The window title at the time the pin was saved. Example: `"notes.txt - Notepad"`. Used for matching — both process name AND title must match. |
| `opacity` | integer | `255` | The window opacity as an 8-bit alpha value (0–255). See section 6 for the encoding details. |
| `click_through` | boolean | `false` | Whether click-through mode is enabled for this window. |
| `was_layered` | boolean | `true` | Whether the window had the `WS_EX_LAYERED` style before PinIt first touched it. Used to restore the window correctly on unpin. |
| `was_topmost` | boolean | `true` | Whether the window was already always-on-top before PinIt pinned it. Used to restore correctly. |
| `was_click_through` | boolean | `true` | Whether the window already had click-through before PinIt set it. Used to restore correctly. |

The three `was_*` fields default to `true` in older files. The default of `true` means "unknown — trust the window's current state", which is the safe fallback for files written before these fields existed.

Any additional fields on a pin entry must be preserved verbatim (same rule as the top-level object).

### 4.3 Loading pins

When loading:
1. Parse the `pins` object.
2. For each entry, extract the index from the key (`"chrome.exe:1"` → index 1).
3. Sort all entries by index ascending.
4. Discard the keys; keep only the values as an ordered list.
5. Skip any entry where `process_name` is empty.

### 4.4 Saving pins

When saving:
1. Take the ordered list of pins.
2. For each pin at position `i`, write it under the key `"<processName>:<i>"`.
3. The same process name can appear multiple times with different indices (multiple windows of the same app can be pinned simultaneously).

---

## 5. The `settings` Object

```
"settings": {
  "enable_sound": true,
  "show_notifications": false,
  "has_seen_tray_notice": false,
  "start_with_windows": false,
  "shortcuts": { ... }
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable_sound` | boolean | `true` | Play the tick sound on pin/unpin. |
| `show_notifications` | boolean | `false` | Show balloon notifications when pinning while the window is hidden. |
| `has_seen_tray_notice` | boolean | `false` | Whether the user has been told that closing the window sends the app to the tray. Set to `true` after showing this message once; never reset. |
| `start_with_windows` | boolean | `false` | Reflects the state of the Run key. Written here for display purposes; the actual state is always read from the registry. |
| `shortcuts` | object | see below | The four customisable hotkeys. |

The `shortcuts` object:

```
"shortcuts": {
  "toggle_pin":    "super+ctrl+KeyT",
  "opacity_up":    "super+ctrl+Equal",
  "opacity_down":  "super+ctrl+Minus",
  "toggle_window": "super+ctrl+KeyP"
}
```

| Field | Default | Description |
|-------|---------|-------------|
| `toggle_pin` | `"super+ctrl+KeyT"` | Pin or unpin the focused window. |
| `opacity_up` | `"super+ctrl+Equal"` | Raise the focused window's opacity by 10%. |
| `opacity_down` | `"super+ctrl+Minus"` | Lower the focused window's opacity by 10%. |
| `toggle_window` | `"super+ctrl+KeyP"` | Show or hide the PinIt window. |

See `07-hotkeys.md` for the full shortcut string format specification.

---

## 6. Opacity Encoding

Opacity is stored in `pinned.json` as an 8-bit alpha value (integer, 0–255). The UI displays and accepts opacity as a percentage (integer, 20–100).

### 6.1 Conversion formulas

These formulas must be used exactly. They differ from a simple linear scale by using rounding, which ensures that converting back and forth produces the same number every time. An earlier version of PinIt used a formula without rounding, which caused the stored opacity to drift by 1% on every restart.

**Percent to alpha (for saving):**
```
alpha = (percent × 255 + 50) / 100   [integer division, result truncated]
```

**Alpha to percent (for loading and displaying):**
```
percent = (alpha × 100 + 127) / 255   [integer division, result truncated]
```

### 6.2 Bounds

- The minimum opacity the user can set is 20%. At 20%, `alpha = (20 × 255 + 50) / 100 = 51`.
- The maximum is 100%. At 100%, `alpha = 255`.
- The default (no opacity change applied) is stored as `255` (fully opaque).
- When loading, if `alpha = 255`, the opacity is 100% and no transparency should be applied to the window.
- When loading, if `alpha < 255`, the stored opacity must be applied to the window after pinning.

### 6.3 Example round-trip

| Percent | Alpha stored | Percent on reload |
|---------|-------------|-------------------|
| 100 | 255 | 100 |
| 90 | 230 | 90 |
| 50 | 128 | 50 |
| 20 | 51 | 20 |

---

## 7. In-Memory State Model

The application maintains two lists at runtime. Neither list is the same as what is in the file at any moment — the file is written when state changes, not continuously.

### 7.1 Live pins list

A list of currently-pinned windows. Each entry contains:

- `hwnd` — the window handle as an integer (isize on 64-bit Windows). **Never saved to disk.** Becomes invalid when the window closes.
- `title` — the current window title. Updated continuously by the re-enforce loop when the title changes.
- `process` — the process executable name.
- `opacity` — current opacity as a percentage (20–100).
- `click_through` — whether click-through is currently active.
- `was_layered` — the window's layered state before PinIt touched it.
- `was_topmost` — the window's topmost state before PinIt touched it.
- `was_click_through` — the window's click-through state before PinIt touched it.
- `opacity_changed` — a flag indicating whether PinIt actually changed the opacity (used to decide whether to restore it on unpin).

### 7.2 Pending pins list

A list of saved pins whose windows are not yet open. Each entry is a direct copy of a `SavedPin` from the file. When the re-enforce loop finds a foreground window that matches a pending pin, it moves the entry to the live list and applies its saved settings.

### 7.3 Settings

A single settings object mirroring the `settings` field in `pinned.json`. Loaded once on startup. Written back whenever any setting changes.

---

## 8. Matching Logic

When restoring a saved pin to a live window, both the process name and the window title must match.

Rules:
- Process name comparison is **case-insensitive** (`notepad.exe` matches `Notepad.EXE`).
- Title comparison is **case-sensitive and exact**.
- If the saved pin's `title` field is empty, it matches nothing. This prevents legacy entries (from very old versions that did not save titles) from incorrectly latching onto any random window of the right process.
- The first pending pin that matches the foreground window is applied. Only one match is processed per re-enforce tick.

---

## 9. Unknown Fields

Both the top-level object, the settings object, each pin entry, and the shortcuts object may contain fields that the current version does not know about. These must be:
- Read and kept in memory alongside the known fields.
- Written back to the file verbatim on the next save.

This ensures that a file written by a newer version of PinIt is not corrupted if the user temporarily runs an older version.

---

## 10. File Integrity

### 10.1 Missing file

If `pinned.json` does not exist, return an empty pin list and default settings. Do not treat a missing file as an error.

### 10.2 Empty file

Treat an empty file the same as a missing file.

### 10.3 Corrupt file

If the file exists but cannot be parsed as valid JSON:
1. Log a warning (process name safe; do not log the file path if it could contain user-identifying info).
2. Rename the file to `pinned.json.corrupt`. This preserves it for manual recovery.
3. Return empty pin list and default settings.
4. On the next save, write a fresh valid file.

Do not delete the corrupt file.

### 10.4 Atomic saves

Never write `pinned.json` directly. Always:
1. Write the new content to `pinned.json.tmp` in the same directory.
2. Rename `pinned.json.tmp` over `pinned.json`.

This ensures a crash or power cut mid-write cannot leave a half-written file. A rename on the same filesystem is atomic on Windows.
