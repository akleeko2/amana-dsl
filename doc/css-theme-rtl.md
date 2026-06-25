# Amana CSS & Design Tokens Architecture (RTL & SSoT)

This document explains the unified stylesheet generation architecture, design token layers, logical properties, and theme capabilities in the Amana compiler.

---

## 1. Single Source of Truth (SSoT)

Previously, core CSS rules were duplicated across three separate files:
1. `src/codegen/express.rs` (as `BASE_CSS_CLASSES`)
2. `src/codegen/express/views.rs` (as `BASE_CSS_CLASSES`)
3. `src/codegen/express/static_files/engine.rs` (embedded inline in JS raw string)

This duplication was replaced by a single reference point:
- [src/codegen/express/base_css.rs](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/src/codegen/express/base_css.rs)

Any updates to core classes, resets, layout grids, or component shells should be made only in `base_css.rs`. The compiler and runtime engine read from this file directly.

---

## 2. Design Token Layers

Amana uses a three-tier design tokens hierarchy:

### Layer 1: Primitives (Raw Values)
Defined in [src/codegen/express/tokens.rs](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/src/codegen/express/tokens.rs), these specify the raw values for color scales, spacing, border-radii, shadows, and typography:
- **Color Scales**: `INDIGO`, `EMERALD`, `ZINC`
- **Spacing Scale**: From `xs` (0.25rem) to `xl4` (6rem)
- **Radii Scale**: `none`, `sharp`, `soft`, `xl`, `xl2`, `round`, `pill`
- **Shadow Scale**: `none`, `soft`, `smooth`, `floating`, `strong`

### Layer 2: Semantic (Purpose-Driven Variables)
These map primitive values to meaningful functions in light and dark modes:
- **Surfaces**: `base`, `canvas`, `muted`, `elevated`, `glass_bg`, `glass_border`
- **Borders**: `subtle`, `strong`, `focus`
- **Text**: `primary`, `secondary`, `muted`

### Layer 3: Component-Scoped Variables
These map semantic tokens to specific component parts (e.g. Button background colors, Card radii and paddings, Modal overlays and z-indices).

---

## 3. Auto Appearance (`mode: auto`)

Amana now supports system-based auto appearance (`mode: auto`) in DSL files. When configured, it detects user preferences automatically using system media queries and applies the appropriate light or dark themes.

Example DSL configuration:
```amana
app {
  theme {
    mode: "auto"
    primary: "indigo"
    accent: "cyan"
  }
}
```

This compiles into:
```css
@media (prefers-color-scheme: dark) {
  :root.dg-mode-auto {
    --bg-primary: #111827;
    --bg-secondary: #050816;
    --text-primary: #f8fafc;
    /* ... semantic dark overrides ... */
  }
}
```
And adds the `.dg-mode-auto` class guard to the `<html>` element.

---

## 4. Logical Properties for Bilingual Layouts (LTR/RTL)

Instead of physical properties (which are hardcoded to left/right and break when changing language direction), Amana uses CSS Logical Properties:

| Physical Property | Logical Property | Description |
|---|---|---|
| `margin-left` / `margin-right` | `margin-inline-start` / `margin-inline-end` | Margin dynamically calculated based on text direction. |
| `padding-left` / `padding-right` | `padding-inline-start` / `padding-inline-end` | Padding calculated based on direction. |
| `left` / `right` | `inset-inline-start` / `inset-inline-end` | Position offset for elements. |
| `border-left` / `border-right` | `border-inline-start` / `border-inline-end` | Border applied to the start or end edge. |

Using logical properties ensures bilingual (Arabic/English) layout layouts scale correctly without duplication or manual direction overrides.

---

## 5. Theme Presets

Amana supports predefined theme presets in the `theme:` block to quickly apply visual guidelines:
- **`luxury`**: A premium, high-contrast dark theme with gold/amber highlights (`rgba(234,179,8,0.25)`), elevated modal inputs with custom golden glows, and Outfit typography.
- **`linear`**: A clean, technical dark/light theme with subtle zinc borders, compact padding, and Inter typography.
- **`stripe`**: A modern SaaS theme featuring indigo and cyan brand accents, soft radii, and rounded cards.

Example:
```amana
theme:
    preset: luxury
    radius: soft
    density: comfortable
```

---

## 6. Component Variants

Variants enable component-specific customization targeting base styles, hover states, slots, and responsive breakpoints (`desktop`, `tablet`, `mobile`).

### Global Variants
Global variants specify style overrides for standard components:
```amana
variant Card.glass:
    base:
        background: glass
        radius: soft
    hover:
        shadow: floating
```

### Local Component Variants
Local variants are defined directly inside a custom component declaration:
```amana
component CardShell:
    variants:
        compact:
            base:
                padding: sm
```

---

## 7. Arabic Canvas Rendering & RTL Flow

Standard HTML5 Canvas does not shape Arabic letters or align them right-to-left. To support charts and drawings, Amana integrates the `arabic-reshaper` library and a custom javascript text segmenter at runtime:
1. **Contextual Reshaping**: Translates isolated letters into correct medial, final, and initial cursive glyphs.
2. **Segment Reordering**: Splits mixed strings (combining Arabic, English, numbers) into segments, reordering the characters of individual Arabic words to LTR sequences, and reversing the words order so they draw correctly on standard LTR canvas renderers.

