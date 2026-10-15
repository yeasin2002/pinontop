# 06 — Persistence

## 1. Overview

Persistence is the feature that makes PinIt genuinely useful rather than a novelty. Without it, every Windows restart would require the user to manually re-pin every window. With it, PinIt remembers everything and silently restores it.

The persistence system has three responsibilities:
1. Write the current state to disk whenever it changes.
2. Read the state back on startup and restore windows.
3. Handle every error condition without losing data.

All state lives in a single file: `pinned.json`. Its schema is fully documented in `03-data-and-state.md`. This document covers the lifecycle, timing, and error handling around that file.

---

## 2. File Location

```
%LOCALAPPDATA%\PinIt\pinned.json
```

The directory `%LOCALAPPDATA%\PinIt` must be created on first save if it does not exist. Creating it silently at startup (before the first write) is the cleanest approach.

In dev mode (`PINIT_DEV=1` environment variable), use `%LOCALAPPDATA%\PinIt-dev\pinned.json` instead. This prevents a development build from touching the data of an installed release copy.

---

## 3. Startup Load

On application startup, before showing the UI:

1. Read `pinned.json` from disk.
2. Parse the JSON.
3. Handle errors (see section 6).
4. Return the parsed state: a list of saved pins + settings.

The loaded pins are NOT the live pin list. They are saved descriptions of windows. The startup restoration procedure (matching saved pins to open windows) is covered in `10-session-and-restore.md`.

### 3.1 What "loading" produces

After a successful load:
- A list of `SavedPin` objects ordered by their key index (see `03-data-and-state.md` section 4.3).
- A `Settings` object with all user preferences.
- Unknown fields preserved in memory for round-trip preservation.

### 3.2 Missing file

If the file does not exist, return an empty pin list and default settings. Log nothing — a missing file is the expected state on first run.

### 3.3 Empty file

Treat identically to a missing file.

---

## 4. Saving

### 4.1 When to save

PinIt saves `pinned.json` in response to specific events. It does not save on a timer or continuously. The events that trigger a save:

| Event | Save type |
|-------|-----------|
| A window is pinned | Immediate |
| A window is unpinned | Immediate |
| A window is removed from the live list (closed) | Immediate |
| A setting is changed | Immediate |
| Opacity changes (slider drag) | Debounced (600 ms) |
| Click-through is toggled | Immediate |
| A pending pin is manually removed | Immediate |
| Session end (`WM_ENDSESSION`) | Immediate (flush any debounced write first) |

### 4.2 Debounced saves

Opacity changes fire many times per second while the user drags the slider. Saving on every change would cause excessive disk writes. Instead:

- Maintain a single debounce timer.
- Each opacity change resets the timer to 600 ms from now.
- The save happens only when the timer fires.
- If an immediate save is triggered for any other reason before the timer fires, cancel the timer and include the current opacity in that save.

### 4.3 What is saved

The save payload consists of two parts built from the current in-memory state:

**Pins list** — concatenation of the live pin list (converted to `SavedPin` format) followed by the pending pin list (already in `SavedPin` format). Order matters: live pins first, then pending.

Converting a live pin to `SavedPin`:
- `process_name` ← from the live entry.
- `title` ← the current title from the live entry.
- `opacity` ← convert percent to alpha using `(percent × 255 + 50) / 100`.
- `click_through` ← from the live entry.
- `was_layered`, `was_topmost`, `was_click_through` ← from the live entry.

**Settings** — the current settings object verbatim.

---

## 5. Atomic Save Procedure

Never write `pinned.json` directly. A crash or power cut mid-write would leave a corrupt file. Use the atomic rename pattern:

1. Ensure the directory exists (`create_dir_all`).
2. Serialize the state to pretty-printed JSON.
3. Write the JSON to `pinned.json.tmp` in the same directory.
4. Rename `pinned.json.tmp` to `pinned.json`.

A rename within the same directory is atomic on Windows (NTFS). Either the old file exists or the new file exists — there is no half-written state.

If the write to `.tmp` fails: log a warning, do not rename, leave the previous `pinned.json` intact.
If the rename fails: log a warning.

---

## 6. Error Handling

### 6.1 Parse error (corrupt file)

If the file exists but JSON parsing fails:

1. Log a warning: `"pinned.json is corrupt (<error>); backing it up to pinned.json.corrupt"`.
2. Rename `pinned.json` to `pinned.json.corrupt`. Do not delete it — the user may be able to manually recover their data from it.
3. Return empty pin list and default settings.
4. On the next save, a fresh valid file is written.

If a `.corrupt` file already exists from a previous crash, overwrite it with the new backup (keeping the most recent corrupt file).

### 6.2 Partial data

If the JSON is valid but some pins have an empty `process_name`, skip those entries silently. An empty process name cannot be used for matching.

### 6.3 Unknown fields

Any JSON key at any level of the structure that the current code does not recognise must be preserved. The recommended implementation approach:

- Use a "flatten unknown fields into a map" pattern (serde's `#[serde(flatten)]` in Rust, or a manual approach in other languages).
- On save, write known fields plus all unknown fields from the preserved map.

This ensures a file written by a newer version of PinIt with additional fields is not silently stripped when opened by an older version.

---

## 7. Pending Pins

Pending pins are saved pins whose windows are not currently open. They are stored in `pinned.json` exactly like live pins — there is no special "pending" section.

When PinIt starts and restores saved pins, the windows that could not be matched to an open window become pending. They remain in the JSON file in their original position (after any live pins).

When a pending pin's window eventually opens (detected by the re-enforce loop), the pin is applied and the entry moves from the pending list to the live list. The file is rewritten to reflect this.

When the user manually removes a pending pin from the UI, the entry is removed from the pending list and the file is rewritten.

### 7.1 Pending pin lifecycle

```
Startup:
  saved pin → no matching open window → added to pending list

Running:
  re-enforce tick:
    foreground window matches pending pin? → pin it → move to live list → save

User action:
  user removes pending row from UI → remove from pending list → save

Session end:
  pending list is kept in the file (pins survive reboot)

Manual quit:
  pending list is cleared from the file
```

---

## 8. Settings-Only Saves

When saving settings (a setting changed but no pins changed), the full file must still be written — including the current pin list. This ensures the settings change does not accidentally truncate the pin data.

The implementation always saves the complete state: all live pins + all pending pins + all settings, as one atomic write.

---

## 9. In-Memory Cache

To avoid re-reading and re-parsing the file before every save, the last-loaded (or last-saved) state is kept in memory. When saving, this in-memory state is merged with the current live and pending lists to produce the new state to write.

The cache must be invalidated (and the file re-read) only in test scenarios where the file is written behind the app's back. In production, PinIt is the only writer of `pinned.json`.

---

## 10. Other Files in the Data Directory

The `%LOCALAPPDATA%\PinIt` directory may contain:

| File | Created by | Purpose |
|------|-----------|---------|
| `pinned.json` | PinIt on any save | Main state file |
| `pinned.json.tmp` | PinIt during atomic save | Temporary; cleaned up by rename |
| `pinned.json.corrupt` | PinIt on parse error | Backup of the corrupt file |
| `pinit.log` | PinIt logging | Diagnostic log (see `14-logging.md`) |
| `pinit.log.old` | PinIt logging | Previous log after rotation |

PinIt should not delete any of these files except as part of its own write operations (renaming `.tmp` over the main file).
