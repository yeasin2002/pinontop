# 11 — Flash Overlay

## 1. What It Is

The flash overlay is a brief coloured rectangular outline drawn around a window when it is pinned or unpinned. It gives the user immediate visual confirmation that the hotkey worked and shows exactly which window was affected — especially useful when multiple windows are on screen.

There is also a persistent hover variant: when the user hovers their mouse over a pin's row in the PinIt window, that same outline appears on the corresponding live window so the user can identify it.

---

## 2. Behaviour

### 2.1 Pin flash (timed)

- Colour: orange (`#C49464` in hex RGB).
- Appears instantly when a window is pinned.
- Stays visible for **300 milliseconds**.
- Fades out over **250 milliseconds** (quadratic ease-out).
- Total duration: 550 ms from trigger to fully gone.

### 2.2 Unpin flash (timed)

- Colour: grey (`#8A857C`).
- Same timing as the pin flash.

### 2.3 Hover highlight (persistent)

- Colour: orange (same as pin flash).
- Appears when the mouse enters a pinned window's row in the UI.
- Stays visible with no fade timer — it is persistent.
- Disappears immediately (no fade) when the mouse leaves the row, or when the PinIt window loses focus.

### 2.4 Animation gate

Before playing any animation, check whether Windows client area animations are enabled (`SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, ...)`). If animations are off (accessibility setting or high-contrast mode), skip the fade and hide the overlay immediately after the hold period.

---

## 3. The Overlay Window

The overlay is a separate invisible Win32 window with specific properties:

| Property | Value | Reason |
|----------|-------|--------|
| Style | `WS_POPUP` | No title bar, no border |
| Extended style | `WS_EX_LAYERED` | Enables per-pixel alpha via `UpdateLayeredWindow` |
| Extended style | `WS_EX_TRANSPARENT` | Mouse clicks pass through to the window behind |
| Extended style | `WS_EX_TOPMOST` | Always on top of everything, including the pinned window |
| Extended style | `WS_EX_TOOLWINDOW` | Not shown in the taskbar or Alt+Tab |
| Extended style | `WS_EX_NOACTIVATE` | Does not steal focus when shown |
| Size | Matches the target window's frame | |
| Visibility | Hidden when not in use | |

The overlay must be `WS_EX_TOPMOST` and positioned ABOVE the pinned window in Z-order. The window it outlines is also always-on-top, so the overlay must be on top of that.

---

## 4. Positioning

The overlay is positioned to match the **visible frame** of the target window, not the window's total rectangle. On Windows 10 and 11, every window has an invisible resize border around it (approximately 8 px on each side) that is not visible to the user. If the overlay used `GetWindowRect`, it would appear larger than the visual window.

To get the visible frame:
```
DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &rect, sizeof(rect))
```

This returns the rectangle of the visible painted area, excluding the invisible border. If this fails (DWM not active or other error), fall back to `GetWindowRect`.

The overlay is positioned to exactly match this rectangle. Its size is the same as the rectangle (not inside it). The outline is drawn at the very edge of the overlay's surface.

### 4.1 DPI awareness

The overlay window must be DPI-aware. On high-DPI displays (e.g. 4K with 200% scaling), physical pixels differ from logical pixels. When calling `UpdateLayeredWindow`, work in physical pixels. The DPI scale for the target window can be obtained with `GetDpiForWindow(hwnd)`.

---

## 5. Drawing the Outline

The overlay draws a rounded-rectangle outline. It is drawn by writing directly into a pixel buffer that is applied via `UpdateLayeredWindow` with premultiplied BGRA pixels.

### 5.1 Outline style

- Shape: rounded rectangle.
- Corner radius: approximately 8 physical pixels on Windows 11 (Windows 11 uses rounded window corners), 0 on Windows 10. Detecting the Windows version accurately for this: use `RtlGetVersion` (not `GetVersionEx` which is faked by the compatibility layer).
- Stroke width: approximately 3–4 physical pixels.
- The outline is drawn at the outer edge of the overlay.

### 5.2 Anti-aliasing using a Signed Distance Field

A clean rounded-rectangle outline is drawn using an SDF (Signed Distance Field) approach:

For each pixel at position `(px, py)` in the overlay:
1. Compute the distance from the pixel to the nearest point on the rounded rectangle boundary.
2. Map the distance to an alpha value: pixels close to the boundary are fully opaque; pixels far from it are transparent; there is a 1–2 pixel feather zone for smoothness.
3. Write the pixel as premultiplied BGRA: `(B × alpha, G × alpha, R × alpha, alpha)`.

This produces a smooth, crisp outline without needing a graphics library.

### 5.3 Premultiplied alpha

`UpdateLayeredWindow` with `ULW_ALPHA` requires premultiplied alpha (each colour channel is pre-multiplied by the alpha value divided by 255). A fully opaque orange pixel `(0xC4, 0x94, 0x64, 0xFF)` becomes `(0xC4, 0x94, 0x64, 0xFF)` (unchanged at full alpha). A semi-transparent pixel at 50% alpha `(0xC4, 0x94, 0x64, 0x80)` becomes `(0x62, 0x4A, 0x32, 0x80)`.

---

## 6. Fade Animation

The fade-out animation runs on a timer:

1. After the hold period (300 ms), start fading.
2. On each timer tick (every ~16 ms for 60 fps):
   a. Advance the animation by the elapsed time.
   b. Compute the current opacity as a fraction of the total fade duration (250 ms).
   c. Apply a quadratic ease-out: `opacity = 1.0 - (t / duration)²` where `t` is elapsed time.
   d. Scale all pixel alpha values by this opacity factor and call `UpdateLayeredWindow` with the new pixel buffer.
3. When `t >= duration`, hide the overlay window.

A quadratic ease-out starts fast and decelerates toward the end, giving the impression of the outline "dissolving".

---

## 7. Showing and Hiding

### 7.1 Showing

1. Compute the target window's frame rectangle using `DwmGetWindowAttribute`.
2. Render the outline into the pixel buffer at full opacity.
3. Position and size the overlay to match the frame rectangle.
4. Call `UpdateLayeredWindow` to apply the pixel buffer.
5. Show the overlay window (`ShowWindow(SW_SHOWNOACTIVATE)` — does not steal focus).
6. Start the hold timer (300 ms) for timed flashes.

### 7.2 New flash while one is already showing

If a flash is already being shown (including the fade phase) when a new one is triggered:
- Cancel the current timer/fade.
- Start the new flash immediately from full opacity.
- This can happen if the user pins/unpins quickly.

### 7.3 Hiding

Call `ShowWindow(SW_HIDE)` on the overlay window. For timed flashes, this happens at the end of the fade animation. For hover highlights, this happens immediately when the hover ends.

---

## 8. Reusing the Overlay Window

The overlay window is created once at startup and reused for every flash. It is not created and destroyed on each use. Creating a new Win32 window has measurable overhead; reusing a hidden window and repositioning it is essentially free.

---

## 9. Overlay vs Target Window Z-Order

When the overlay is shown over a window that is `WS_EX_TOPMOST` (as all pinned windows are), the overlay must be placed above it in Z-order. Simply having `WS_EX_TOPMOST` is enough — all topmost windows are drawn above non-topmost windows, and among topmost windows, they are ordered by their last `SetWindowPos` call. The overlay should be the most recently positioned topmost window when it appears.

After showing the overlay, call `SetWindowPos` to ensure it is on top of the target window:
```
SetWindowPos(overlayHwnd, targetHwnd, ..., SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)
```
This positions the overlay just above the target window in Z-order.

---

## 10. Summary of Flash Kinds

| Kind | Colour | Trigger | Duration | Dismissal |
|------|--------|---------|----------|-----------|
| Pin | Orange `#C49464` | Window pinned | 300 ms hold + 250 ms fade | Automatic |
| Unpin | Grey `#8A857C` | Window unpinned | 300 ms hold + 250 ms fade | Automatic |
| Hover | Orange `#C49464` | Mouse enters pin row | Persistent | Mouse leaves row |
