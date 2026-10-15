# 05 — Opacity and Click-Through

## 1. Overview

Two optional modifiers can be applied to any pinned window beyond the always-on-top flag:

1. **Opacity** — reduce the window's transparency from fully opaque down to 20%.
2. **Click-through** — make mouse clicks pass through the window to whatever is behind it.

Both features use the Windows **layered window** model. Understanding layered windows is essential to implementing these features correctly.

---

## 2. Layered Windows

A layered window is a window that has the `WS_EX_LAYERED` flag in its extended style. Layered windows participate in the Windows compositor and can have a per-pixel alpha channel or a uniform transparency value applied to them.

PinIt uses the uniform transparency approach via `SetLayeredWindowAttributes`:

```
SetLayeredWindowAttributes(hwnd, colorKey, alpha, LWA_ALPHA)
```

- `colorKey` — not used for PinIt's purposes (set to 0).
- `alpha` — a byte value from 0 (fully transparent) to 255 (fully opaque).
- `LWA_ALPHA` — flag indicating the alpha parameter is valid.

### 2.1 Adding the layered style

Before calling `SetLayeredWindowAttributes`, the window must have `WS_EX_LAYERED` set. If it does not already have it:

```
currentStyle = GetWindowLongW(hwnd, GWL_EXSTYLE)
SetWindowLongW(hwnd, GWL_EXSTYLE, currentStyle | WS_EX_LAYERED)
```

### 2.2 Important: was the window already layered?

Before setting `WS_EX_LAYERED`, check whether the window already had it. Store this as `was_layered`. This is critical for the restore step on unpin:

- If `was_layered` is `true`: the application itself manages its own transparency (some video players, some overlay tools, etc.). PinIt must not remove the `WS_EX_LAYERED` style when restoring.
- If `was_layered` is `false`: PinIt added the style. On unpin, PinIt should remove it and call `SetWindowPos` with `SWP_FRAMECHANGED` to force Windows to redraw the window without the layered style.

---

## 3. Setting Opacity

### 3.1 The user-facing model

- Opacity is shown as a percentage: 100% (fully opaque) down to 20% (nearly invisible).
- At 100%, no transparency is applied (alpha = 255).
- Below 100%, the layered style is set and `SetLayeredWindowAttributes` is called with the corresponding alpha.
- The minimum is 20% because windows become unusable at very low opacity.

### 3.2 Opacity step

- The hotkeys increment/decrement by 10 percentage points.
- The UI slider allows arbitrary values within the 20–100 range.
- After a hotkey change, the opacity clamps to [20, 100] before applying.

### 3.3 The conversion formulas

See `03-data-and-state.md` section 6 for the exact formulas. They must be used consistently everywhere:

- To save: `alpha = (percent × 255 + 50) / 100`
- To load: `percent = (alpha × 100 + 127) / 255`

These use integer arithmetic with rounding to guarantee a lossless round-trip.

### 3.4 Tracking whether opacity was changed

PinIt records an `opacity_changed` flag per pin in the in-memory live list. This flag is set to `true` the first time PinIt calls `SetLayeredWindowAttributes` on a window. It is used on unpin:

- If `opacity_changed` is `false`, PinIt never changed the window's opacity, so there is nothing to restore.
- If `opacity_changed` is `true`, PinIt must restore the original appearance.

This distinction matters for windows that already had `WS_EX_LAYERED` before PinIt touched them. Without this flag, unpinning such a window would reset an application-managed transparency to 100%.

### 3.5 Debouncing saves

The opacity slider in the UI fires many change events while being dragged. PinIt must not write `pinned.json` on every event. The correct approach:

- Maintain a debounce timer of 600 milliseconds.
- Each opacity change restarts the timer.
- The file is written only when the timer fires (i.e. the user has stopped dragging for 600 ms).

If any other event triggers an immediate save before the timer fires, cancel the timer and include the current opacity in that save.

---

## 4. Restoring Opacity on Unpin

When unpinning a window, if `opacity_changed` is `true`:

1. Call `SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA)` to restore full opacity.
2. Check `was_layered`:
   - If `true` (window already had layered style): stop here. The window manages its own transparency; leave `WS_EX_LAYERED` set.
   - If `false` (PinIt added the layered style): remove `WS_EX_LAYERED` from the extended style and call `SetWindowPos` with `SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED` to force Windows to redraw the window without the layered attribute.

The `SWP_FRAMECHANGED` flag is necessary. Without it, the window may continue to render as if it is still layered even after the style flag is removed.

---

## 5. Click-Through Mode

### 5.1 What it is

Click-through mode makes mouse clicks pass through the window to whatever window is behind it. The window remains visible. The user can see it but cannot click into it.

The Win32 mechanism is the `WS_EX_TRANSPARENT` flag in the extended window style.

### 5.2 The dependency on layered windows

Windows only honours `WS_EX_TRANSPARENT` on a window that also has `WS_EX_LAYERED`. If `WS_EX_TRANSPARENT` is set on a non-layered window, it has no effect on most Windows versions.

Therefore: when enabling click-through on a window that is not already layered, PinIt must first apply the layered style and set an alpha value. The current opacity of the window is used. If the window is currently at 100%, PinIt still applies the layered style with alpha=255 (which makes the window look no different visually, but satisfies Windows' requirement).

### 5.3 Enabling click-through

1. Check if the window is already layered.
2. If not layered: apply `WS_EX_LAYERED` and call `SetLayeredWindowAttributes` with the current opacity.
3. Read the current extended style.
4. Set `WS_EX_TRANSPARENT`: `newStyle = oldStyle | WS_EX_TRANSPARENT`.
5. Call `SetWindowLongW(hwnd, GWL_EXSTYLE, newStyle)`.
6. Verify by reading back: `GetWindowLongW(hwnd, GWL_EXSTYLE) & WS_EX_TRANSPARENT` should be set.
7. Update the in-memory pin entry's `click_through` to `true`.
8. Trigger a debounced save.

### 5.4 Disabling click-through

1. Read the current extended style.
2. Remove `WS_EX_TRANSPARENT`: `newStyle = oldStyle & ~WS_EX_TRANSPARENT`.
3. Call `SetWindowLongW(hwnd, GWL_EXSTYLE, newStyle)`.
4. Update the in-memory pin entry's `click_through` to `false`.
5. Trigger a debounced save.

### 5.5 Restoring click-through on unpin

If `click_through` is `true` AND `was_click_through` is `false`:
- Remove `WS_EX_TRANSPARENT` (the application did not have it originally).

If `was_click_through` is `true`:
- Leave `WS_EX_TRANSPARENT` as-is. The application managed its own click-through.

---

## 6. Interaction Between Opacity and Click-Through

The two features interact in one important way: click-through requires layered, and layered requires an alpha set. If the user enables click-through at full opacity (100%), the window becomes transparently click-through — it looks opaque but clicks pass through it. This is intentional and useful (e.g. a clock overlay the user never needs to click).

The opacity change triggered by enabling click-through on a fully-opaque window sets the `opacity_changed` flag just like any other opacity change, so the layered style will be correctly cleaned up on unpin.

---

## 7. Opacity Hotkey Behaviour

The opacity hotkeys adjust the **currently focused window** — but only if it is pinned.

If the user presses `Win+Ctrl+=` while a non-pinned window is focused:
- Show an error: "Pin this window first to change its opacity."
- Do not change anything.

If the focused window is pinned:
- Increase/decrease its opacity by 10 percentage points.
- Clamp to [20, 100].
- Apply immediately via `SetLayeredWindowAttributes`.
- Update the in-memory opacity value.
- Debounce the save.

The opacity hotkeys use auto-repeat (they do NOT have `MOD_NOREPEAT` set on `RegisterHotKey`). This allows the user to hold the key and smoothly fade a window up or down.

---

## 8. Saving and Loading Opacity

Opacity is saved to `pinned.json` as an 8-bit alpha value in the `opacity` field of the pin entry (see `03-data-and-state.md`).

On load, when re-pinning a saved window:
- If `opacity` is 255: do not apply any transparency. The window is fully opaque.
- If `opacity` is less than 255: apply the layered style and call `SetLayeredWindowAttributes` with the stored alpha value.
- Compute the percentage for display: `percent = (alpha × 100 + 127) / 255`.

---

## 9. Animations Detection

Before showing any animated UI (such as the flash overlay fade), check whether Windows animations are enabled via `SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, ...)`. If the user has turned off animations (via Ease of Access settings or high-contrast mode), skip animations and make transitions instant.
