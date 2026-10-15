# 13 — Theme and Styling

## 1. Overview

PinIt uses a warm "paper" palette rather than the generic blue/grey found in most Windows apps. The palette has a light variant and a dark variant. The app follows Windows' colour mode and switches automatically when the user changes between light and dark mode in Windows Settings.

---

## 2. Colour Mode Detection

### 2.1 Reading the Windows colour mode

Windows stores the colour mode in the registry:

**Key:** `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize`
**Value:** `AppsUseLightTheme` (REG_DWORD)
- `1` = light mode
- `0` = dark mode

Read this value at startup to determine the initial mode.

### 2.2 Listening for changes

Windows does not send a message when the colour mode changes. The correct approach is to subscribe to the system's colour scheme change notification:

- **Win32:** Listen for `WM_SETTINGCHANGE` with `lParam` pointing to the string `"ImmersiveColorSet"`.
- **Tauri / React:** The `prefers-color-scheme` CSS media query can be used in the frontend, combined with a `matchMedia` listener. Tauri also surfaces the colour scheme via the `window.theme()` API.

When the mode changes, rebuild the UI with the new palette. In React this means switching a CSS variable set or a class on the root element.

---

## 3. Colour Palette

All colours are defined as design tokens. The UI references tokens by name, never by raw hex value. This makes switching between light and dark a single token swap.

### 3.1 Light palette

| Token | Hex value | Usage |
|-------|-----------|-------|
| `--color-window` | `#f8f6f2` | Window / dialog background |
| `--color-card` | `#ffffff` | Card background, button background |
| `--color-card-border` | `rgba(0,0,0,0.08)` | Card border |
| `--color-border` | `rgba(0,0,0,0.12)` | Button borders, key chip borders |
| `--color-control-border` | `#8a8378` | Checkbox outline (3:1 contrast against window) |
| `--color-text` | `#2a2622` | Primary text |
| `--color-text-muted` | `#6b6760` | Secondary text, descriptions |
| `--color-text-subtle` | `#6b655d` | "+" separators, unpin icon |
| `--color-key-bg` | `#f0ede6` | Key chip background, button hover |
| `--color-accent` | `#b07c4a` | Slider fill, non-text accents |
| `--color-accent-strong` | `#96653a` | Primary button, focus rings (4.5:1 contrast) |
| `--color-accent-hover` | `#855832` | Primary button hover state |
| `--color-on-accent` | `#ffffff` | Text on primary button |
| `--color-on-avatar` | `#2a2622` | Initial letter on avatar badge |
| `--color-danger-hover-bg` | `#f6e3da` | Unpin button hover background |
| `--color-track` | `#e6e2da` | Slider groove (unfilled portion) |
| `--color-warning-bg` | `#fbeed3` | Warning banner background |
| `--color-warning-border` | `#e2c27d` | Warning banner border |
| `--color-warning-text` | `#5c4307` | Warning banner text |
| `--color-status-bg` | `#2a2622` | In-window toast background |
| `--color-status-text` | `#f8f6f2` | In-window toast text |

### 3.2 Dark palette

| Token | Hex value | Usage |
|-------|-----------|-------|
| `--color-window` | `#1f1d1a` | Window / dialog background |
| `--color-card` | `#2a2724` | Card background |
| `--color-card-border` | `rgba(255,255,255,0.08)` | Card border |
| `--color-border` | `rgba(255,255,255,0.16)` | Button borders, key chip borders |
| `--color-control-border` | `#8d867b` | Checkbox outline |
| `--color-text` | `#f1ede6` | Primary text |
| `--color-text-muted` | `#b8b1a6` | Secondary text |
| `--color-text-subtle` | `#aba498` | "+" separators, unpin icon |
| `--color-key-bg` | `#38342f` | Key chip background, button hover |
| `--color-accent` | `#d0a06c` | Slider fill, non-text accents |
| `--color-accent-strong` | `#d9a871` | Primary button (4.5:1 contrast with `--on-accent`) |
| `--color-accent-hover` | `#e6ba88` | Primary button hover |
| `--color-on-accent` | `#1f1d1a` | Text on primary button |
| `--color-on-avatar` | `#2a2622` | Initial on avatar badge (same as light) |
| `--color-danger-hover-bg` | `#4a3328` | Unpin button hover |
| `--color-track` | `#45403a` | Slider groove |
| `--color-warning-bg` | `#4a3a17` | Warning banner background |
| `--color-warning-border` | `#7a6124` | Warning banner border |
| `--color-warning-text` | `#f5deb0` | Warning banner text |
| `--color-status-bg` | `#f1ede6` | In-window toast background |
| `--color-status-text` | `#1f1d1a` | In-window toast text |

### 3.3 Shared flash overlay colours (not in palette, hardcoded)

| Usage | Hex RGB |
|-------|---------|
| Pin flash outline | `#C49464` |
| Unpin flash outline | `#8A857C` |

---

## 4. Typography

### 4.1 Font family

Use the system font stack: `"Segoe UI", system-ui, sans-serif`. Segoe UI is the Windows UI font and is always available on Windows 10 and 11. Do not bundle a custom font.

### 4.2 Font sizes

| Role | Size | Weight |
|------|------|--------|
| Window title (header) | 17 px | 700 (bold) |
| Section label | 11 px | 700, letter-spacing: 1 px, uppercase |
| Body text (pin title) | 13 px | 600 (semibold) |
| Secondary text (process name, descriptions) | 12 px | 400 (regular) |
| Key chips | 11 px | 700 |
| Opacity percentage label | 11 px | 400 |
| Button text | 12 px | 400 (ghost), 700 (primary) |
| Toast text | 12 px | 400 |

### 4.3 Text colour rules

- Primary text: `--color-text`
- Secondary / muted text: `--color-text-muted`
- Separators and subtle glyphs: `--color-text-subtle`
- Text on the primary button: `--color-on-accent`
- Text on the toast: `--color-status-text`

---

## 5. Spacing and Layout

### 5.1 Base unit

All spacing uses a 4 px base unit. Common values: 4, 8, 12, 16, 24 px.

### 5.2 Pin row

- Height: 44 logical pixels.
- Horizontal padding: 8 px left, 5 px right.
- Gap between elements: 7 px.
- The avatar is vertically centred within the row.

### 5.3 Sheet

- Slides up from the bottom.
- Has its own internal padding: 16 px on all sides.
- A 1 px top border in `--color-border`.

### 5.4 Card

- Rounded corners: 12 px radius.
- Border: 1 px solid `--color-card-border`.

### 5.5 Buttons

- Border radius: 8 px.
- Padding: 7 px vertical, 14 px horizontal.
- The primary button uses a 2 px border (always present in the fill colour when not focused) so that the focus ring (which replaces the border colour) does not cause a layout shift.

### 5.6 Key chips

- Border radius: 5 px.
- Padding: 3 px vertical, 9 px horizontal.
- Border: 1 px solid `--color-border`.
- Background: `--color-key-bg`.

---

## 6. Motion and Animation

### 6.1 Durations

| Name | Duration | Usage |
|------|----------|-------|
| Fast | ~100 ms | Hover state transitions, immediate feedback |
| Medium | ~200 ms | Sheet slide-in/slide-out |
| Slow | ~300 ms | Row fade-in on add |

### 6.2 Easing

- Slide-in: ease-out (fast start, slow end — feels like something landing).
- Fade-in: ease-out.
- Flash overlay: quadratic ease-out for the fade.

### 6.3 Animation gate

As described in `11-flash-overlay.md`, always check `SPI_GETCLIENTAREAANIMATION` before animating. If animations are disabled:
- Sheet transitions are instant (no slide).
- Row additions are instant (no fade).
- Flash overlay is shown without fade.

---

## 7. App Avatar Colour Assignment

The process initial avatar uses a deterministic pastel colour based on the process name. This ensures the same app always shows the same colour across sessions and across users.

Algorithm:
1. Lowercase the process name.
2. Compute a simple hash: fold each byte with `hash = hash * 31 + byte`.
3. Select `TINTS[hash mod 6]` from the palette below.

Pastel palette:

| Index | Hex | Appearance |
|-------|-----|-----------|
| 0 | `#D8C6F0` | Lavender |
| 1 | `#FFE08A` | Yellow |
| 2 | `#C3E4CF` | Mint green |
| 3 | `#F6CAB8` | Peach |
| 4 | `#BEDCF2` | Sky blue |
| 5 | `#F2CFE2` | Pink |

The initial letter displayed is the first alphanumeric character of the process name, uppercased. If there is no alphanumeric character, show "?".

---

## 8. Windows 11 Title Bar Integration

### 8.1 Caption colour

Set the title bar (caption) background to match the window background using:
```
DwmSetWindowAttribute(hwnd, DWMWA_CAPTION_COLOR, &colorref, sizeof(colorref))
```

Where `colorref` is the window background colour as a `COLORREF` (packed `0x00BBGGRR`).

Light mode: `0x00F2F6F8` (note COLORREF byte order is BGR, so `#f8f6f2` becomes `0x00F2F6F8`).
Dark mode: `0x001A1D1F`.

This makes the title bar blend with the content, eliminating the visual break between the title bar and the window body.

### 8.2 Dark title bar

On dark mode, also set:
```
DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &TRUE, sizeof(TRUE))
```

This tells Windows to draw the title bar controls (close/minimize buttons, text) in their dark style.

### 8.3 When to apply

Apply title bar styling:
- On startup, after the window is created.
- When the colour mode changes.

In Tauri, the window HWND can be obtained from the raw window handle after setup. Call `DwmSetWindowAttribute` directly from the Rust backend using the `windows` crate.

---

## 9. Accessibility Colour Contrast

The palette is designed to meet WCAG 2.1 AA minimum contrast ratios:

| Pair | Minimum contrast |
|------|----------------|
| `--color-text` on `--color-window` | 4.5:1 |
| `--color-text` on `--color-card` | 4.5:1 |
| `--color-accent-strong` on `--color-card` (non-text accents) | 3:1 |
| `--color-on-accent` on `--color-accent-strong` | 4.5:1 |
| `--color-control-border` on `--color-window` | 3:1 |

Do not change colour values without verifying that these contrast ratios are maintained.

Note: full WCAG compliance verification requires manual testing with assistive technologies; the contrast ratios above are a necessary but not sufficient condition.
