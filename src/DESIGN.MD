---
version: 1.0.0
name: Solid-Monochrome-Engineering-System
description: A high-precision, solid black and pure white design language tailored for modern engineering and workspace management. Built with crisp, smaller corner radii (rounded-lg for cards, rounded-md for inputs and buttons), eliminating bubbly pill shapes, cream canvases, and pastel greens in favor of high-contrast solid monochrome architecture. Powered by trending Google Fonts: Plus Jakarta Sans for headings, Geist for body typography, and Geist Mono for identifiers and metrics.

colors:
  primary: "#000000"
  ink: "#000000"
  on-primary: "#ffffff"
  background: "#ffffff"
  foreground: "#000000"
  canvas-light: "#ffffff"
  canvas-dark: "#000000"
  surface-card: "#ffffff"
  surface-subtle: "#f4f4f5"
  surface-muted: "#e4e4e7"
  border-subtle: "#e4e4e7"
  border-default: "#d4d4d8"
  border-strong: "#000000"
  zinc-50: "#fafafa"
  zinc-100: "#f4f4f5"
  zinc-200: "#e4e4e7"
  zinc-300: "#d4d4d8"
  zinc-400: "#a1a1aa"
  zinc-500: "#71717a"
  zinc-600: "#52525b"
  zinc-700: "#3f3f46"
  zinc-800: "#27272a"
  zinc-900: "#18181b"
  zinc-950: "#09090b"

typography:
  cssVariables:
    font-heading: "var(--font-heading)"
    font-sans: "var(--font-sans)"
    font-mono: "var(--font-mono)"
  fonts:
    heading: "Plus Jakarta Sans, sans-serif"
    body: "Geist, sans-serif"
    mono: "Geist Mono, monospace"

  display-xl:
    fontFamily: "var(--font-heading)"
    fontSize: 40px
    fontWeight: 700
    lineHeight: 1.15
    letterSpacing: -0.03em
  heading-lg:
    fontFamily: "var(--font-heading)"
    fontSize: 28px
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: -0.02em
  heading-md:
    fontFamily: "var(--font-heading)"
    fontSize: 20px
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: -0.015em
  heading-sm:
    fontFamily: "var(--font-heading)"
    fontSize: 16px
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: -0.01em
  body-md:
    fontFamily: "var(--font-sans)"
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: -0.006em
  body-strong:
    fontFamily: "var(--font-sans)"
    fontSize: 14px
    fontWeight: 600
    lineHeight: 1.5
    letterSpacing: -0.006em
  caption:
    fontFamily: "var(--font-sans)"
    fontSize: 12px
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: 0
  code-badge:
    fontFamily: "var(--font-mono)"
    fontSize: 11px
    fontWeight: 500
    lineHeight: 1.2
    letterSpacing: 0.02em

rounded:
  sm: "4px"
  md: "6px"
  lg: "8px"
  full: "9999px" # Strictly reserved for micro status dots and circular loading spinners

components:
  card:
    backgroundColor: "{colors.surface-card}"
    borderColor: "{colors.border-subtle}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.lg}"
    padding: "24px"
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    fontFamily: "var(--font-sans)"
    fontWeight: 500
    fontSize: "14px"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  button-outline:
    backgroundColor: "{colors.background}"
    borderColor: "{colors.border-subtle}"
    hoverBorderColor: "{colors.border-strong}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  input-text:
    backgroundColor: "{colors.background}"
    borderColor: "{colors.border-subtle}"
    focusBorderColor: "{colors.border-strong}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.md}"
    padding: "8px 12px"
  badge:
    backgroundColor: "{colors.zinc-100}"
    borderColor: "{colors.border-subtle}"
    textColor: "{colors.foreground}"
    fontFamily: "var(--font-mono)"
    fontSize: "11px"
    rounded: "{rounded.md}"
    padding: "2px 8px"
---

# Solid Monochrome Engineering Design System

## 1. Design Overview & Philosophy

The user interface utilizes a **solid black and pure white** visual system designed specifically for technical workflows, developer platforms, and engineering workspace management.

Key distinctions:

- **Zero Pastel & Zero Cream Canvas:** All surfaces are pure `#ffffff` and pure `#000000`. Colored fills (such as cream `#fbfbf5` or pastel aloe `#c1fbd4`) are prohibited.
- **Smaller, Crisp Corner Radii:** Eliminates bubbly `rounded-2xl`, `rounded-3xl`, and `rounded-full` cards. Outer containers use `rounded-lg` (8px / 0.5rem), while internal controls, inputs, and buttons use `rounded-md` (6px / 0.375rem).
- **High Contrast & Sharp Edges:** High-contrast 1px zinc borders (`border-zinc-200`) transition to solid black on active hover or focus.
- **Micro-Dots Only:** `rounded-full` is strictly reserved for animated spinners and 6px status indicators.

---

## 2. Typography & Font Architecture

The typographic system utilizes self-hosted Google Fonts bundled directly via `@fontsource-variable/plus-jakarta-sans` and `geist/font` to ensure 100% build reliability on containerized platforms (Railway, Docker, CI) without external CDN fetch vulnerabilities:

```tsx
// apps/web/app/layout.tsx
import "@fontsource-variable/plus-jakarta-sans";
import { GeistSans } from "geist/font/sans";
import { GeistMono } from "geist/font/mono";
```

### CSS Variables Definition (`apps/web/app/globals.css`)

```css
@theme {
  --font-heading: var(--font-heading);
  --font-sans: var(--font-sans);
  --font-mono: var(--font-mono);
}
```

### Usage Roles

| Role                       | Font Family       | CSS Variable / Class                   | Primary Use Case                                                                               |
| -------------------------- | ----------------- | -------------------------------------- | ---------------------------------------------------------------------------------------------- |
| **Headings & Brand**       | Plus Jakarta Sans | `var(--font-heading)` / `font-heading` | Page titles (`h1`), section headers (`h2`-`h4`), brand marks, modal titles, and card headers.  |
| **Body & UI**              | Geist             | `var(--font-sans)` / `font-sans`       | Paragraphs, button labels, form labels, tooltips, navigation links, and descriptions.          |
| **Identifiers & Metadata** | Geist Mono        | `var(--font-mono)` / `font-mono`       | Timestamps, dates, task priority badges, pagination counters, role tags, and numerical counts. |

---

## 3. Color Tokens

### Primary Palette

- **Background (`--background`):** `#ffffff` (Pure White)
- **Foreground (`--foreground`):** `#000000` (Pure Black)
- **Primary Action (`--primary`):** `#000000`
- **Primary Text on Dark (`--primary-foreground`):** `#ffffff`
- **Surface Elevation (`--card`):** `#ffffff`
- **Card Text (`--card-foreground`):** `#000000`

### Zinc Grayscale Ladder

- **Zinc 50 / 100:** Subtle zebra-striping, table header backgrounds, secondary tag fills (`#fafafa` / `#f4f4f5`).
- **Zinc 200:** Standard 1px card and container borders (`#e4e4e7`).
- **Zinc 300 / 400:** Placeholder text and disabled border boundaries (`#d4d4d8` / `#a1a1aa`).
- **Zinc 500 / 600:** Subtitle descriptions, helper text, and secondary timestamps (`#71717a` / `#52525b`).
- **Zinc 900 / 950:** Dark accent surfaces and hover states (`#18181b` / `#09090b`).

---

## 4. Corner Radius Guidelines

The application strictly avoids large "bubbly" rounded aesthetics:

| Token          | Radius Value       | Component Mapping                                                                                                                                                |
| -------------- | ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rounded-lg`   | `8px` (`0.5rem`)   | Main dashboard stat cards, workspace containers, modal dialogs, project cards, table wrapper cards.                                                              |
| `rounded-md`   | `6px` (`0.375rem`) | Buttons, text inputs, textareas, select dropdowns, user avatars, status badges, priority badges, pagination buttons.                                             |
| `rounded-sm`   | `4px` (`0.25rem`)  | Code snippets, inline keyboard shortcuts (`kbd`), sub-tags.                                                                                                      |
| `rounded-full` | `9999px`           | **Strictly prohibited on cards, buttons, or dialogs.** Only permitted on circular status micro-indicators (`h-1.5 w-1.5`) and loading spinners (`animate-spin`). |

---

## 5. Standard Component Specifications

### A. Buttons (`components/ui/button.tsx`)

- **Default (Primary):** `bg-black text-white hover:bg-zinc-800 rounded-md font-sans text-sm font-medium transition-colors`
- **Outline:** `border border-zinc-200 bg-white text-black hover:border-black rounded-md font-sans text-sm font-medium transition-colors`
- **Ghost:** `text-zinc-600 hover:text-black hover:bg-zinc-100 rounded-md transition-colors`
- **Danger:** `bg-red-600 text-white hover:bg-red-700 rounded-md transition-colors`

### B. Cards (`components/dashboard-stat-cards.tsx`, `project-card.tsx`)

- Container: `bg-white rounded-lg border border-zinc-200 p-6 hover:border-black transition-colors`
- Title: `font-heading font-semibold text-black`
- Metric: `font-heading font-bold text-black text-2xl`
- Icon Box: `h-10 w-10 rounded-md bg-zinc-100 flex items-center justify-center text-black`

### C. Badges (`components/ui/badge.tsx`)

- Status Badge: `rounded-md font-mono text-[11px] font-semibold border px-2 py-0.5`
- Role Badge (Owner): `rounded-md font-mono text-[11px] font-semibold bg-black text-white px-2 py-0.5 border border-black`
- Role Badge (Member): `rounded-md font-mono text-[11px] font-medium bg-zinc-100 text-black px-2 py-0.5 border border-zinc-200`

### D. Form Inputs (`components/ui/auth-input.tsx`, modals)

- `rounded-md border border-zinc-200 bg-white py-2 px-3 text-sm text-black placeholder:text-zinc-400 outline-none focus:border-black focus:ring-1 focus:ring-black transition-colors`

---

## 6. Verification & Quality Checklist

1. **No Bubbly Cards:** All outer cards use `rounded-lg`, inner items use `rounded-md`.
2. **Solid B&W Palette:** Zero `#fbfbf5` canvas, zero `#c1fbd4` fills.
3. **Typography Standard:** All headers use `font-heading` (Plus Jakarta Sans), body text uses `font-sans` (Geist), and counts/dates use `font-mono` (Geist Mono).
4. **Build Integrity:** Clean compile with strict Next.js and Tailwind CSS validation.
