# CSS DSL v2 and Theme System

Amana CSS is a design DSL, not handwritten CSS-first code. It compiles styling declarations into highly optimized, responsive, and RTL-compliant CSS rules.

## Theme Keys

Configure the global theme in the `theme` block of your `.amana` file:

```amana
theme:
    mode: dark
    direction: rtl
    language: ar
    font_provider: google
    font_family: "Noto Sans Arabic"
    heading_font_family: "Space Grotesk"
    arabic_font_family: "Tajawal"
    primary: emerald
    accent: "#f97316"
    canvas: "#f8fafc"
    base: "#ffffff"
    elevated: "#ffffff"
    text: "#0f172a"
    muted: "#475569"
    border: "rgba(15,23,42,0.10)"
    radius: soft
    surface: elevated
    density: comfortable
    gradient_hero: "linear-gradient(135deg, #ecfdf5 0%, #f8fafc 48%, #fff7ed 100%)"
```

### Supported Properties:
- `mode`: The color mode fallback (`light`, `dark`, `day`, `night`).
- `direction`: Text and layout direction (`ltr` or `rtl`). First-class support for RTL guarantees correct logical property mappings (e.g. padding-inline-start/end, margin-inline-start/end, flex alignments).
- `language`: Locale string (e.g., `en`, `ar`).
- `font_provider`: `system` or `google` (generates Google Fonts import dynamically).
- `font_family`, `heading_font_family`, `arabic_font_family`: Configure custom fonts.
- `primary`, `accent`: Colors used for interactive states, key buttons, and gradients.
- `canvas`, `base`, `elevated`, `text`, `muted`, `border`: Define the main interface palette.
- `radius`: Set standard border-radius curve (e.g. `soft`, `round`, `sharp`).
- `surface`: Base background style (`elevated`, `canvas`, `glass`, etc.).
- `density`: Spacing density (`compact`, `comfortable`, `spacious`).

---

## 5-Level Radius Scale

Amana implements a standard 5-level border-radius scale. These are compiled to CSS variables and mapped dynamically based on theme requirements:

| Token | CSS Variable | Default Value | Description |
| --- | --- | --- | --- |
| `sm` | `--radius-sm` | `10px` | Small elements (badges, indicators, tags) |
| `md` | `--radius-md` | `16px` | Medium elements (form fields, small buttons, inner cards) |
| `lg` | `--radius-lg` | `22px` | Large elements (standard cards, alerts, tab containers) |
| `xl` | `--radius-xl` | `28px` | Extra large elements (main cards, sidebars, dashboard panels) |
| `2xl` | `--radius-2xl` | `36px` | Hero sections, modals, and container shells |

### Alias Mappings:
To maintain compatibility and ease of design, the compiler automatically registers aliases that map to these tokens:
- `--radius-small` maps to `--radius-sm`
- `--radius-medium` maps to `--radius-md`
- `--radius-large` maps to `--radius-lg`
- `--radius-soft` maps to `--radius-md`

---

## Supported Design Tokens

Amana generates a comprehensive design token system exposed as custom CSS properties (`var(--token-name)`). Below are the tokens guaranteed to be available across all generated stylesheets:

### 1. Spacing Scale
Used to maintain layout rhythm and consistent padding/gap heights:
- `--space-xs`: `0.25rem` (4px)
- `--space-sm`: `0.5rem` (8px)
- `--space-md`: `1rem` (16px) or dynamically adjusted based on `density`
- `--space-lg`: `1.5rem` (24px) or dynamically adjusted based on `density`
- `--space-xl`: `2rem` (32px) or dynamically adjusted based on `density`
- `--space-2xl`: `3rem` (48px)
- `--space-3xl`: `4.5rem` (72px)
- `--space-4xl`: `6rem` (96px)

### 2. Typography Sizing
Font size tokens based on a curated typographic scale:
- `--text-xs`: `0.75rem` (12px)
- `--text-sm`: `0.875rem` (14px)
- `--text-md`: `1rem` (16px)
- `--text-lg`: `1.125rem` (18px)
- `--text-xl`: `1.35rem` (21.6px)
- `--text-2xl`: `1.75rem` (28px)
- `--text-3xl`: `2.4rem` (38.4px)

### 3. Shadows Scale
Control elevation depth:
- `--shadow-sm`: `0 1px 3px rgba(15,23,42,0.08), 0 1px 2px rgba(15,23,42,0.04)`
- `--shadow-md`: `0 4px 6px -1px rgba(15,23,42,0.10), 0 2px 4px -1px rgba(15,23,42,0.06)`
- `--shadow-lg`: `0 10px 24px -8px rgba(15,23,42,0.18)`
- `--shadow-xl`: `0 20px 40px -12px rgba(15,23,42,0.28)`
- `--shadow-smooth`: `0 4px 6px -1px rgba(0,0,0,0.1), 0 2px 4px -1px rgba(0,0,0,0.06)` (overridden dynamically in dark mode for soft shadows)
- `--shadow-floating`: High-depth shadow for floating modals/cards.
- `--shadow-strong`: Maximum depth shadow for overlays.

### 4. Interactive Transitions
Smooth hover and state transitions:
- `--transition-fast`: `all 0.12s ease-in-out`
- `--transition-smooth`: `all 0.2s ease-in-out`

### 5. Semantic Colors
- `--color-success`: Success state color (default green).
- `--color-warning`: Warning state color (default amber/yellow).
- `--color-danger`: Error/danger state color (default red).
- `--border-color`: Maps directly to the configured border color.

### 6. Page Widths
- `--content-width`: Max width of standard page grids (`1120px`).
- `--wide-width`: Max width of wide section containers (`1360px`).
- `--readable-width`: Max width of text blocks for reading comfort (`72ch`).

---

## Mesh Gradients & Dynamic Themes

The `gradient: mesh` property applies a premium, organic background gradient. Instead of compiling to a static color combination, it dynamically inherits the primary and accent colors from the current theme configuration:

```css
--gradient-mesh: radial-gradient(circle at 10% 20%, var(--color-primary-soft), transparent 34%), 
                 radial-gradient(circle at 80% 0%, var(--color-accent-soft), transparent 38%), 
                 var(--surface-base);
```

This allows templates to switch between light and dark modes or change their brand accent without breaking gradient harmony.

---

## Design Grammar Blocks and Layout Helpers

Styles can be written using design-grammar blocks within component definitions:

```amana
.card:
    surface: elevated
    radius: xl
    shadow: floating
    hover: lift
    transition: smooth

.hero-box:
    gradient: mesh
    glow: primary
    columns: responsive 24rem
```

### standard layout structures:
- `Container()`: Centers content within `--content-width` (or `--wide-width` / `--readable-width`).
- `Grid(min: "...")`: Responsive CSS Grid template.
- `Split()`: Two-column proportional layout.
- `Stack()`: Vertical stack of elements.
- `Cluster()`: Horizontal wrap of inline elements.
- `Sidebar()`: Master-detail side column layout.
