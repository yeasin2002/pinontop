# 07 — Global Hotkeys

## 1. Overview

Global hotkeys let the user control PinIt from anywhere on the system without switching to the PinIt window. They work while any other application is focused. There are four hotkeys, all customisable.

---

## 2. The Four Hotkeys

| ID | Action | Default shortcut |
|----|--------|-----------------|
| 1 (`HOTKEY_PIN`) | Pin or unpin the focused window | `Win + Ctrl + T` |
| 2 (`HOTKEY_OPACITY_UP`) | Increase focused window's opacity by 10% | `Win + Ctrl + =` |
| 3 (`HOTKEY_OPACITY_DOWN`) | Decrease focused window's opacity by 10% | `Win + Ctrl + -` |
| 4 (`HOTKEY_WINDOW`) | Show or hide the PinIt window | `Win + Ctrl + P` |

These IDs are stable. Never renumber them. A renumber would silently break existing shortcut configurations on restart.

---

## 3. Win32 Hotkey Registration

### 3.1 RegisterHotKey

```
RegisterHotKey(hwnd, id, modifiers, virtualKey)
```

- `hwnd` — the HWND that will receive `WM_HOTKEY` messages. This must be PinIt's hidden helper window (not NULL, because the helper window approach is more reliable and receives messages even when the message loop is not pumped on the main thread).
- `id` — one of the four constants above.
- `modifiers` — a bitmask of `MOD_WIN`, `MOD_CONTROL`, `MOD_ALT`, `MOD_SHIFT`, and optionally `MOD_NOREPEAT`.
- `virtualKey` — a Windows virtual key code (e.g. `'T'` for the T key, `VK_OEM_PLUS` for `=`).

### 3.2 MOD_NOREPEAT

The pin and window-toggle hotkeys use `MOD_NOREPEAT`. This prevents repeated `WM_HOTKEY` messages while the key is held down — holding `Win+Ctrl+T` should pin exactly once, not once per key-repeat interval.

The opacity hotkeys do NOT use `MOD_NOREPEAT`. Holding `Win+Ctrl+=` should smoothly increase opacity, one step per key-repeat event. This gives the user fine control when holding the key.

### 3.3 Return value

`RegisterHotKey` returns `false` if the hotkey combination is already registered by another application (or by a previous PinIt instance that did not clean up). In this case:
- Do not crash.
- Record the failure: the action's name goes into a list of "unavailable" hotkeys.
- This list is shown in the shortcut editor so the user knows to pick a different combination.

### 3.4 UnregisterHotKey

Before registering a hotkey (including on startup), call `UnregisterHotKey(hwnd, id)` first. This ensures a clean slate even if a previous run left a hotkey registered (unlikely, but possible after a crash).

On app exit, all hotkeys must be unregistered.

### 3.5 WM_HOTKEY handling

When the helper window's message loop receives `WM_HOTKEY`:
- `wParam` contains the hotkey ID.
- `lParam` contains the modifiers and virtual key (informational; use `wParam` to dispatch).

Dispatch to the appropriate handler:
- `HOTKEY_PIN` → get foreground window, toggle pin.
- `HOTKEY_OPACITY_UP` → get foreground window, adjust opacity by +10.
- `HOTKEY_OPACITY_DOWN` → get foreground window, adjust opacity by -10.
- `HOTKEY_WINDOW` → toggle PinIt window visibility.

---

## 4. The Shortcut String Format

Shortcuts are stored in `pinned.json` and passed between the UI and backend as strings. The format originated in the Tauri v1 era of PinIt and must be preserved exactly.

### 4.1 Format

```
<modifier>+<modifier>+...+<key>
```

Modifiers and key are joined by `+`. Order is: modifiers first, key last. The key is always last.

### 4.2 Modifier tokens

| Token | Win32 constant | Meaning |
|-------|---------------|---------|
| `super` | `MOD_WIN` | The Windows key |
| `ctrl` | `MOD_CONTROL` | The Ctrl key |
| `alt` | `MOD_ALT` | The Alt key |
| `shift` | `MOD_SHIFT` | The Shift key |

Modifiers are always lowercase in the string.

### 4.3 Key tokens

Key tokens follow the **Web API `KeyboardEvent.code`** naming convention. This is a code, not a character — it identifies the physical key, not what it types.

Examples:

| String token | Physical key | Win32 VK |
|-------------|-------------|----------|
| `KeyT` | T key | `'T'` (0x54) |
| `KeyP` | P key | `'P'` |
| `Equal` | = key (also +) | `VK_OEM_PLUS` |
| `Minus` | - key (also _) | `VK_OEM_MINUS` |
| `F9` | F9 function key | `VK_F9` |
| `F1`–`F24` | Function keys | `VK_F1`–`VK_F24` |
| `ArrowUp` | Up arrow | `VK_UP` |
| `ArrowDown` | Down arrow | `VK_DOWN` |
| `ArrowLeft` | Left arrow | `VK_LEFT` |
| `ArrowRight` | Right arrow | `VK_RIGHT` |
| `PageUp` | Page Up | `VK_PRIOR` |
| `PageDown` | Page Down | `VK_NEXT` |
| `Home` | Home | `VK_HOME` |
| `End` | End | `VK_END` |
| `Space` | Space bar | `VK_SPACE` |
| `Enter` | Enter/Return | `VK_RETURN` |
| `Tab` | Tab | `VK_TAB` |
| `Backspace` | Backspace | `VK_BACK` |
| `Delete` | Delete | `VK_DELETE` |
| `Insert` | Insert | `VK_INSERT` |
| `Escape` | Escape | `VK_ESCAPE` |
| `Digit0`–`Digit9` | Number row 0–9 | `'0'`–`'9'` |
| `Numpad0`–`Numpad9` | Numpad 0–9 | `VK_NUMPAD0`–`VK_NUMPAD9` |
| `NumpadAdd` | Numpad + | `VK_ADD` |
| `NumpadSubtract` | Numpad - | `VK_SUBTRACT` |
| `NumpadMultiply` | Numpad * | `VK_MULTIPLY` |
| `NumpadDivide` | Numpad / | `VK_DIVIDE` |

Letters A–Z are represented as `KeyA`–`KeyZ` with the corresponding ASCII uppercase byte as the VK code.

### 4.4 Parsing a shortcut string

To convert `"super+ctrl+KeyT"` into `(modifiers, vk)`:

1. Split the string by `+`.
2. For each part except the last:
   - `"super"` → add `MOD_WIN` to modifiers.
   - `"ctrl"` → add `MOD_CONTROL`.
   - `"alt"` → add `MOD_ALT`.
   - `"shift"` → add `MOD_SHIFT`.
3. The last part is the key token. Look it up in the key table above to get the VK code.
4. If the key token is unrecognised, preserve it (do not fail) — it may be from a future version. Log a warning but do not crash.

### 4.5 Building a shortcut string

To convert `(modifiers, vk)` and a key label into a string:

1. Start with an empty list.
2. If `MOD_WIN` is set → append `"super"`.
3. If `MOD_CONTROL` is set → append `"ctrl"`.
4. If `MOD_ALT` is set → append `"alt"`.
5. If `MOD_SHIFT` is set → append `"shift"`.
6. Append the key token (from the reverse lookup in the key table).
7. Join with `"+"`.

The order of modifiers in the built string is always: super, ctrl, alt, shift. This canonical order ensures that `"super+ctrl+KeyT"` and `"ctrl+super+KeyT"` produce the same string.

### 4.6 Display tokens

For showing the shortcut as keyboard-chip badges in the UI:

| Modifier token | Display label |
|---------------|--------------|
| `super` | `Win` |
| `ctrl` | `Ctrl` |
| `alt` | `Alt` |
| `shift` | `Shift` |

Key tokens are displayed as-is for most keys, with some translations for readability:

| Key token | Display label |
|----------|--------------|
| `ArrowUp` | `↑` or `Up` |
| `ArrowDown` | `↓` or `Down` |
| `ArrowLeft` | `←` or `Left` |
| `ArrowRight` | `→` or `Right` |
| `PageUp` | `Page Up` |
| `PageDown` | `Page Down` |
| `Equal` | `=` |
| `Minus` | `-` |
| `Space` | `Space` |
| `Enter` | `Enter` |
| `Escape` | `Esc` |
| `Backspace` | `Backspace` |
| `Delete` | `Del` |

---

## 5. Safe Modifier Rule

A shortcut must include at least one of: `Win`, `Ctrl`, or `Alt`. `Shift` alone is not sufficient, and bare keys with no modifiers are rejected entirely.

The reason: a global hotkey intercepts the key press system-wide. A shortcut like `Shift+A` would prevent the user from typing capital A in any application. Win, Ctrl, and Alt are considered "safe" because they are not used alone to produce characters in normal typing.

Validation rules:
- If the shortcut has no modifiers at all → reject.
- If the only modifier is `Shift` → reject.
- Any combination that includes Win, Ctrl, or Alt → accept.

This validation runs in the shortcut editor before the user saves. An invalid shortcut should display a warning inline.

---

## 6. Conflict Detection

When a shortcut is registered via `RegisterHotKey` and it returns failure, another application already owns that combination. The failure is recorded as a list of action names that are unavailable.

The shortcut editor shows these unavailable actions with a warning badge. The user must choose a different combination for those actions.

On startup, failed registrations are logged (with action names, not key combinations — the key combination may be sensitive).

---

## 7. Applying New Shortcuts

When the user saves changes in the shortcut editor:

1. For each of the four actions:
   a. Call `UnregisterHotKey(hwnd, id)`.
   b. Parse the new shortcut string → `(modifiers, vk)`.
   c. Validate: has a safe modifier? Valid key?
   d. If valid: call `RegisterHotKey(hwnd, id, modifiers | MOD_NOREPEAT, vk)` (or without `MOD_NOREPEAT` for opacity actions).
   e. If `RegisterHotKey` fails: record the failure.
2. Save the new shortcuts to `pinned.json`.
3. Return the list of any failed registrations to the UI.

If any registration fails, the shortcut editor remains open and shows the conflicts. The user must resolve them before saving.

---

## 8. Restoring Default Shortcuts

The shortcut editor provides a "Restore defaults" button that resets all four shortcuts to their default values and re-registers them. The same apply-new-shortcuts procedure above is used.

---

## 9. Edge Cases

### 9.1 Hotkey fires while PinIt window is open

The pin hotkey (`HOTKEY_PIN`) checks the foreground window. If the user presses it while the PinIt window is focused, the foreground window IS the PinIt window. `is_pinnable` will reject PinIt's own windows (by PID comparison), so nothing happens — or a friendly message is shown.

### 9.2 Show/hide hotkey

`HOTKEY_WINDOW` toggles the PinIt window. The logic:
- If the window is visible AND PinIt's window is the foreground window → hide the window.
- Otherwise → show the window and bring it to the foreground.

This allows pressing the hotkey to both open PinIt (from anywhere) and close it (when it's already open and focused).

### 9.3 Hotkey fires during session end

During session shutdown, Windows is closing all applications. If a hotkey fires at this moment (unlikely but possible), the app should do nothing — do not attempt to pin windows while the session is ending.
