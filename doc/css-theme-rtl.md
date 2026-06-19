# CSS, Theme, RTL, And Design Grammar

This document describes the styling surface implemented by `src/parser/css.rs`, `src/parser/styles.rs`, `src/parser/design.rs`, `src/semantic/mod.rs`, and `src/codegen/express/theme.rs`.

## Theme

```amana
theme:
    mode: dark
    direction: rtl
    language: ar
    font_provider: google
    font_family: "Inter"
    heading_font_family: "Inter"
    arabic_font_family: "Tajawal"
    primary: "#4f46e5"
    accent: "#06b6d4"
    canvas: "#0f172a"
    text: "#f8fafc"
    radius: soft
    density: comfortable
```

Theme keys are allowlisted. Unknown keys fail semantic validation.

Closed values:

- `mode`: `dark`, `night`, `day`, `light`.
- `direction`: `ltr`, `rtl`.
- `radius`: `none`, `sharp`, `soft`, `round`, `pill`.
- `density`: `compact`, `comfortable`, `spacious`.
- `font_provider`: `system`, `google`.

Safety rules:

- Values containing `javascript:`, `expression(`, `<script`, `</style`, `behavior:`, `;`, `{`, or `}` are rejected.
- Font values are limited to 80 characters.
- Other values are limited to 260 characters.

For the full key list, use [language-inventory.generated.md](language-inventory.generated.md).

> [!IMPORTANT]
> No new custom theme system or component registry is supported. The baseline fallback theme remains `indigo`/`cyan`.

## Runtime Theme Output

The generated Express/EJS runtime emits root CSS variables for:

- Primary/accent colors (defaulting to indigo/cyan if not specified).
- Surface/canvas/text colors.
- Radius scale.
- Spacing scale affected by density.
- Font stacks.
- Gradients.
- Glass and elevation variables.

If `font_provider: google`, the runtime emits a Google Fonts import using the configured body, heading, and Arabic font families.

### Phase 2 Visual Preset & Styles
Visual features are compiled to dedicated utility and component classes as CSS overrides and presets:
- **Neo-Bento/Glass Surface**: Implemented via CSS variables and selectors (e.g. backdrop filter and border-radius styles) as standard improvements, not as a separate theme.
- **Navbar Glass**: The `Navbar(variant: "glass")` tag compiles to include `.amana-navbar-glass` which adds a backdrop-filter blur.
- **Timeline RTL Markers**: The timeline markers adjust alignment dynamically based on direction. In LTR mode, `.amana-timeline-item::before` is aligned to the left, and in RTL mode, `[dir="rtl"] .amana-timeline-item::before` shifts the marker to the right.
- **PricingCard Featured Variant**: Compiled to `.amana-pricing-card.amana-variant-featured`.
- **Button Focus & Active States**: Emitted with `.amana-btn:focus-visible` and `.amana-btn:active` styling.
- **Card Hover Transitions**: Styled with transition offsets under `.amana-card:hover`.
- **KPI Styling**: Emitted under `.amana-kpi-value` classes for structured data displays.
- **Preview Engine Synchronization**: The runtime preview engine (`engine.js`) matches all visual/layout CSS variables and class rules exactly to guarantee identical visual rendering between dev mode and build output.
- **State Scope Wrapper**: Every EJS page with client-side state is compiled inside a `.amana-state-scope` wrapper to isolate dynamic states and maintain `height: 100%` viewport sizing constraints.
- **Mobile DashboardShell Layout Contract**: Under mobile viewports (`max-width: 720px`), sidebars (`.side-rail`) and settings tab bars (`.settings-nav`) automatically convert into compact, touch-scrollable horizontal swipe rails (`flex-wrap: nowrap; overflow-x: auto;`). Multi-column layout grids automatically stack vertically (`grid-template-columns: 1fr !important`), and padding is compacted to `1rem` to maximize mobile viewport utilization.
- **Mobile Density Contract**: In mobile viewports, card spacing, headings, grid gap variables, and components are down-scaled. Secondary long lists and logging panels are constrained with max-height limits (`max-height: 380px` or `max-height: 400px`) and vertical auto-scroll behaviors to minimize vertical page scroll lengths without hiding critical user information.

## RTL And Language

`direction: rtl` changes the document direction and runtime CSS direction variables. `language: ar` configures the document language and encourages Arabic font fallback in the generated font stack.

The current implementation primarily emits direction-aware page-level CSS and font stack behavior. The compiler does not perform a complete source-to-source rewrite of every physical CSS property into logical CSS properties.

## Style Block DSL

```amana
style:
    .panel:
        layout: column
        gap: lg
        surface: glass
        radius: soft
        shadow: floating
        transition: fade 300ms ease
```

Style blocks parse selector blocks and declarations. Declarations are compiled by `compile_css_decl`.

Important mapped properties:

- `layout`: `row`, `column`, `stack`, `grid`, `center`, `inline`, `cluster`, `split`.
- `center`: `both`, `x`, `y`.
- `columns`, `rows`, `responsive-columns`.
- `position`: `sticky`, `fixed`, `absolute`, `relative`, `static`.
- `layer`, `z`, `z-index`.
- `direction`, `dir`.
- `radius`, `border-radius`.
- `shadow`, `box-shadow`.
- `border`.
- `surface`.
- `background`, `background-color`.
- `gradient`.
- `glow`.
- `hover`.
- `color`, `text`, `ink`, `fill`, `stroke`, `outline`.
- `opacity`, `blur`, `blend`, `shape`.
- `font`, `size`, `font-size`, `weight`, `leading`, `tracking`.
- `transition`, `scroll`, `hide`.

Spacing tokens:

```text
none, 0, xs, sm, small, md, medium, lg, large, xl, 2xl, xxl, 3xl, 4xl
```

Size tokens:

```text
full, screen, fit, min, max, content, readable, wide,
fluid-xs, fluid-sm, fluid-md, fluid-lg, fluid-xl, fluid-2xl, fluid-3xl
```

Color tokens:

```text
primary, primary-soft, accent, success, warning, danger,
surface, surface-muted, surface-elevated, ink, subtle, canvas-soft,
custom-primary, custom-accent, custom-bg, custom-text,
canvas, text, muted, secondary, border,
indigo, cyan, violet, emerald, rose, slate
```

## CSS Security

Semantic validation checks style rules before codegen.

Blocked selectors:

```text
body html * script iframe object embed link meta base
```

Attribute selector manipulation such as `[onclick]` or `[on...]` is rejected.

Blocked value patterns include:

```text
javascript:
expression(
behavior:
url(data:
url(http:
url(https:
<script
</style
-moz-binding
binding:
```

Properties are allowlisted. See the generated inventory for the full allowlist.

## Design Grammar

Design blocks are declarative metadata and token controls placed inside render trees or view-level `canvas:`.

```amana
section.hero:
    compose:
        layout: bento
        columns: 3
        gap: lg
    visual:
        surface: glass
        gradient: hero
    motion:
        hover: lift
    responsive:
        mobile.columns: 1
```

Supported blocks:

```text
canvas compose visual type motion creative brand art responsive interaction a11y component tokens states
```

Closed-value properties:

- `layout`
- `surface`
- `hover`
- `entrance` / `reveal`
- `gradient`
- `density`
- `shadow`

Open metadata fields are accepted after safety checks and length limits. Examples:

```text
voice, colorway, direction, motif, lighting, texture, feedback, cursor,
contrast, screenreader, uniqueness, freedom, signature
```

Key names normalize underscores and dashes for validation. Nested paths such as `responsive.mobile` and `mobile.columns` are accepted when the base key is allowed.

## Layout-Specific Compose Rules

When `compose.layout` is one of the following, additional key restrictions apply:

| Layout | Allowed keys |
| --- | --- |
| `bento` | `layout`, `columns`, `rows`, `gap`, `auto_place`, `responsive`, `rhythm`, `focus_path`, `density` |
| `masonry` | `layout`, `columns`, `image_ratio`, `gap` |
| `split` | `layout`, `ratio`, `align`, `visual_position` |
| `asymmetric` | `layout`, `rhythm`, `dominant`, `overlap` |
| `magazine` | `layout`, `columns`, `headline_span`, `aside_span`, `pull_quote` |
| `sidebar` | `layout`, `sidebar_width`, `sidebar_position`, `sticky_sidebar` |
| `timeline` | `layout`, `axis`, `marker`, `alternate` |
| `dashboard-shell` | `layout`, `sidebar`, `topbar`, `content_width`, `density`, `rhythm` |

## Top-Level Tokens Block

The parser accepts:

```amana
tokens:
    colors:
        brand: "#4f46e5"
    spacing:
        section: "4rem"
    radius:
        card: "18px"
    shadows:
        card: "0 20px 40px rgba(0,0,0,.2)"
```

Current status: parsed into `TokenConfigBlock`, preserved in IR, and emitted into generated theme CSS as root custom properties.
