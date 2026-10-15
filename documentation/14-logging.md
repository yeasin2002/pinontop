# 14 — Logging

## 1. Purpose

PinIt writes a diagnostic log file to help debug problems. When a user reports a bug, the log is the first thing to check. It is kept deliberately small and privacy-safe so users can share it without concern.

---

## 2. Log File Location

| Mode | Path |
|------|------|
| Normal | `%LOCALAPPDATA%\PinIt\pinit.log` |
| Dev mode | `%LOCALAPPDATA%\PinIt-dev\pinit.log` |

The directory is the same as `pinned.json`. It is created on first write if it does not exist.

---

## 3. Log Entry Format

Each log entry is one line in the following format:

```
YYYY-MM-DD HH:MM:SS.mmm [L] message\r\n
```

Fields:
- `YYYY-MM-DD` — local date (4-digit year, 2-digit month, 2-digit day).
- `HH:MM:SS.mmm` — local time with millisecond precision (24-hour clock).
- `[L]` — severity level: `[I]` for info, `[W]` for warning.
- `message` — the log message text.
- `\r\n` — Windows CRLF line endings.

Example:
```
2026-10-16 14:23:01.042 [I] PinIt 3.0.0 starting
2026-10-16 14:23:01.117 [I] Pinned a window of chrome.exe
2026-10-16 14:23:45.880 [W] Hotkeys unavailable: Pin / unpin window
```

---

## 4. Severity Levels

Only two levels are used:

| Level | Tag | When to use |
|-------|-----|-------------|
| Info | `[I]` | Normal lifecycle events: startup, shutdown, pin/unpin, restore |
| Warning | `[W]` | Recoverable problems: corrupt file, UIPI block, hotkey conflict, log rotation, save failure |

There is no debug/trace level in the release build. The log is meant for users to share, not for developer-level tracing.

---

## 5. What to Log

### 5.1 Always log

| Event | Level | Message format |
|-------|-------|---------------|
| App starting | Info | `"PinIt <version> starting"` |
| Window pinned | Info | `"Pinned a window of <process>"` |
| Window unpinned | Info | `"Unpinned a window of <process>"` (optional — not in the original) |
| Pins restored at startup | Info | `"Restored <n> window(s) on startup"` |
| Session ending | Info | `"Session ending: restored <n> window(s), keeping pins for next login"` |
| Manual quit | Info | `"Restored and cleared <n> pinned window(s) on manual quit"` |
| Another instance detected | Info | `"Another instance is running; asked it to show"` |
| Autostart repaired | Warning | `"Repaired autostart registry path"` |
| Hotkeys unavailable | Warning | `"Hotkeys unavailable: <action name>, <action name>"` |
| UIPI pin failure | Warning | `"Pin failed for <process> (likely elevated/UIPI)"` |
| Corrupt file backed up | Warning | `"pinned.json is corrupt (<error>); backing it up to pinned.json.corrupt"` |
| Save failure | Warning | `"Could not save <path>: <error>"` |
| Log rotation | Warning | `"Log rotated"` |

### 5.2 Never log

- **Window titles** — titles can contain document names, email subjects, URLs, or other sensitive content. They are never written to the log.
- **File paths that include the username** — if a path must be logged, log only the filename portion, not the full path.
- **Registry values** — values can contain paths; log the fact of the operation, not the value.
- **Shortcut strings** — key combinations are not sensitive, but they are not needed in the log either.
- **User settings content** — log the fact that settings were saved, not the contents.

The guiding principle: the log should be safe to attach to a GitHub issue without revealing anything personal about the user's machine or work.

---

## 6. Log Rotation

When the log file reaches **512 KB**:

1. Rename `pinit.log` to `pinit.log.old`. If `pinit.log.old` already exists, overwrite it.
2. Start a new `pinit.log` and continue writing to it.
3. Write a warning entry to the new log: `"Log rotated"`.

Only one old file is kept. Rotation is checked on each write. The simplest implementation: after writing, check the file size; if it exceeds 512 KB, rotate.

---

## 7. Panic / Crash Logging

Install a panic hook at startup that writes the panic information to the log before the process aborts.

The panic hook should:
1. Format the panic message (including the source file and line number if available).
2. Write it to the log as a warning entry: `"PANIC: <message>"`.
3. Flush the log file.
4. Allow the default panic behaviour (abort) to proceed.

This ensures that a crash leaves a record in the log, even if it happens before any other log entries.

---

## 8. Implementation Notes

### 8.1 File opening strategy

The log file does not need to remain open between writes. Open it in append mode, write the line, and close it. This is slightly less efficient than keeping it open, but simpler and avoids holding a file lock that could interfere with external log viewers.

### 8.2 Timestamp precision

Use the local system time with millisecond precision. The millisecond component is important for distinguishing events that happen in rapid succession (e.g. multiple windows being restored at startup).

### 8.3 Thread safety

In a single-threaded application (which PinIt is), there are no thread safety concerns for the log. In a Tauri app where the backend may have multiple threads, the log writer should be protected by a mutex.

### 8.4 Log macros

Provide `info!` and `warning!` macros (or equivalent) that format the timestamp and severity tag automatically. The rest of the codebase should never call the file I/O directly — all log writes should go through these macros.

Pseudocode for the macro:
```
macro info(message):
    timestamp = format_local_time_with_ms(now())
    line = "{timestamp} [I] {message}\r\n"
    append_to_log(line)
    check_rotation()
```

### 8.5 First write creates the directory

The first time a log entry is written, if the `%LOCALAPPDATA%\PinIt` directory does not exist, create it. Do not fail silently if the directory cannot be created — but also do not crash. A diagnostic tool must not prevent the app from working.
