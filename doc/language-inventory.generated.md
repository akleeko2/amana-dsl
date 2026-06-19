# Amana Language Inventory (Generated)

This file is generated from compiler implementation files, excluding tests. Do not edit it by hand; run `python scripts/language_inventory.py --write`.

## Scanned Source Files

- `src/ast/mod.rs`
- `src/codegen/express/hooks.rs`
- `src/codegen/express/static_files/app.rs`
- `src/codegen/express/static_files/default_hooks.rs`
- `src/codegen/express/static_files/engine.rs`
- `src/codegen/express/static_files/hooks_worker.rs`
- `src/codegen/express/static_files/package.rs`
- `src/codegen/express/static_files/security.rs`
- `src/codegen/express/static_files.rs`
- `src/codegen/express/theme.rs`
- `src/codegen/express/views.rs`
- `src/codegen/express.rs`
- `src/codegen/html.rs`
- `src/codegen/mod.rs`
- `src/codegen/sql.rs`
- `src/formatter.rs`
- `src/lexer/mod.rs`
- `src/main.rs`
- `src/parser/css.rs`
- `src/parser/design.rs`
- `src/parser/expressions.rs`
- `src/parser/mod.rs`
- `src/parser/styles.rs`
- `src/parser/top_level.rs`
- `src/parser/views.rs`
- `src/semantic/ir.rs`
- `src/semantic/ir_gen.rs`
- `src/semantic/mod.rs`
- `src/semantic/optimizer.rs`
- `src/semantic/schema.rs`
- `src/semantic/scope.rs`
- `src/semantic/suggestions.rs`
- `src/semantic/types.rs`
- `src/semantic/views.rs`

## CLI Surface

- Commands: `check`, `build`, `fmt`, `inspect-design`, `dev`, `lsp`

- Aliases: `--help`, `help`, `inspect-design`, `design-report`, `lsp`, `language-server`

## Top-Level Language Nodes

- `App`
- `Theme`
- `Seed`
- `Model`
- `Route`
- `View`
- `Component`
- `Variant`
- `Tokens`

- Import preprocessor syntax: `import "./relative-file.amana"`

## Lexer

- Keywords: `app`, `model`, `route`, `view`, `component`, `protected`, `server`, `client`, `render`, `state`, `form`, `if`, `else`, `for`, `in`, `permit`, `fetch`, `style`, `variant`, `slot`, `optional`, `tokens`

- Data types: `str`, `int`, `float`, `bool`, `email`, `password`, `datetime`, `money`

- Literals: `true`, `false`, `null`

- Word operators: `and`, `or`, `not`

- Symbols: `+`, `-`, `*`, `/`, `==`, `!=`, `>`, `<`, `>=`, `<=`, `=`, `?`, `:`, `.`, `,`, `->`, `%`, `(`, `)`, `[`, `]`

- String forms: `"text"`, `"""multi-line"""`, `f"Hello {name}"`

## Parser Blocks

- View blocks: `protected`, `server`, `client`, `render`, `style`, `canvas`

- Component blocks: `render`, `style`, `variants`

- Design blocks: `compose`, `visual`, `type`, `motion`, `creative`, `brand`, `art`, `responsive`, `interaction`, `a11y`, `component`, `tokens`, `states`

- Form settings: `connect`, `redirect`, `default`, `where`, `ui`, `submit`, `field`

- Form field options: `label`, `placeholder`, `type`, `help`, `required`

- Resource options: `item`, `empty`, `loading`, `error`, `filters`, `sort`

- State persistence values parsed by syntax: `memory`, `cookie`, `session`, `local`

## Theme Keys

- `primary`
- `accent`
- `mode`
- `direction`
- `language`
- `radius`
- `density`
- `surface`
- `canvas`
- `background`
- `bg`
- `base`
- `surface_base`
- `muted_surface`
- `surface_muted`
- `elevated`
- `surface_elevated`
- `text`
- `ink`
- `muted`
- `subtle`
- `border`
- `glass`
- `glass_bg`
- `glass_border`
- `gradient_primary`
- `gradient`
- `gradient_accent`
- `gradient_hero`
- `radius_2xl`
- `font_provider`
- `font_family`
- `body_font`
- `font`
- `heading_font_family`
- `heading_font`
- `display_font`
- `arabic_font_family`
- `arabic_font`
- `success`
- `warning`
- `danger`
- `gradient_mesh`
- `gradient_aurora`
- `gradient_spotlight`

### Closed Theme Values

- `mode`: `dark`, `night`, `day`, `light`

- `direction`: `ltr`, `rtl`

- `radius`: `none`, `sharp`, `soft`, `round`, `pill`

- `density`: `compact`, `comfortable`, `spacious`

- `font_provider`: `system`, `google`

## Design Grammar

### `a11y`

`contrast`, `reduce-motion`, `focus`, `reduce_motion`, `screenreader`

### `art`

`texture-opacity`, `direction`, `motif`, `lighting`, `texture`

### `brand`

`voice`, `personality`, `trust`, `colorway`

### `canvas`

`layout`, `composition`, `surface`, `density`, `rhythm`, `mode`, `palette`, `content-width`, `wide-width`, `readable-width`, `responsive-mobile`, `responsive-tablet`, `responsive-laptop`, `responsive-desktop`, `responsive.mobile`, `responsive.tablet`, `responsive.laptop`, `responsive.desktop`, `width`, `flow`, `content_width`, `responsive`

### `component`

`variant`, `chrome`, `shape`, `density`

### `compose`

`layout`, `columns`, `rows`, `gap`, `auto_place`, `responsive`, `rhythm`, `focus_path`, `density`, `ratio`, `align`, `visual_position`, `image_ratio`, `dominant`, `overlap`, `headline_span`, `aside_span`, `pull_quote`, `sidebar_width`, `sidebar_position`, `sticky_sidebar`, `axis`, `marker`, `alternate`, `sidebar`, `topbar`, `content_width`, `focal`, `balance`

### `creative`

`freedom`, `uniqueness`, `signature`, `avoid_repetition`, `reference`, `avoid-repetition`

### `interaction`

`focus-strength`, `feedback`, `affordance`, `focus_strength`, `cursor`

### `motion`

`speed`, `transition`, `transform`, `opacity`, `entrance`, `hover`

### `responsive`

`columns`, `desktop.columns`, `laptop.columns`, `tablet.columns`, `mobile.columns`, `desktop.padding`, `laptop.padding`, `tablet.padding`, `mobile.padding`, `desktop.gap`, `laptop.gap`, `tablet.gap`, `mobile.gap`, `mobile`, `collapse`

### `states`

`hover`, `focus`, `active`

### `tokens`

`colors`, `spacing`, `radius`, `shadows`

### `type`

`measure`, `weight`, `size`, `font-size`, `leading`, `line-height`, `tracking`, `letter-spacing`, `scale`, `align`, `contrast`

### `visual`

`depth`, `visual-weight`, `texture-opacity`, `glow-strength`, `primary`, `color.primary`, `brand.primary`, `accent`, `color.accent`, `brand.accent`, `background`, `bg`, `surface.bg`, `color.background`, `text`, `ink`, `color.text`, `muted`, `subtle`, `color.muted`, `border`, `border.color`, `stroke`, `outline`, `gradient.value`, `gradient-value`, `custom-gradient`, `radius`, `shape.radius`, `shadow`, `shadow.value`, `padding`, `space.padding`, `space.gap`, `gap`, `surface`, `palette`, `texture`, `frame`, `gradient`, `shape`

### Closed Design Values

- `density`: `compact`, `comfortable`, `spacious`

- `entrance`: `fade`, `slide-up`, `slide-down`, `zoom`, `blur`, `clip`, `stagger-up`, `none`

- `gradient`: `primary`, `accent`, `hero`, `mesh`, `aurora`, `spotlight`, `custom`, `brand`, `sunset`, `ocean`, `mesh-cyan-indigo`, `mesh-aurora`

- `hover`: `lift`, `glow`, `scale`, `lift-glow`, `none`

- `layout`: `row`, `column`, `stack`, `grid`, `center`, `inline`, `cluster`, `split`, `bento`, `split-diagonal`, `asymmetric`, `editorial`, `dashboard-shell`, `magazine`, `command-center`, `showcase-rail`, `masonry`, `sidebar`

- `shadow`: `sm`, `md`, `lg`, `xl`, `soft`, `floating`, `strong`, `smooth`, `none`

- `surface`: `base`, `muted`, `elevated`, `glass`, `custom`, `outline`, `flat`, `layered`, `glass-layered`

## CSS DSL

- Allowed properties: `display`, `position`, `top`, `right`, `bottom`, `left`, `inset`, `inset-inline`, `inset-inline-start`, `inset-inline-end`, `width`, `height`, `min-width`, `min-height`, `max-width`, `max-height`, `padding`, `padding-top`, `padding-right`, `padding-bottom`, `padding-left`, `padding-inline`, `padding-block`, `margin`, `margin-top`, `margin-right`, `margin-bottom`, `margin-left`, `margin-inline`, `margin-block`, `gap`, `row-gap`, `column-gap`, `grid-template-columns`, `grid-template-rows`, `grid-column`, `grid-row`, `grid-auto-flow`, `flex-direction`, `flex-wrap`, `justify-content`, `align-items`, `flex-grow`, `flex-shrink`, `flex-basis`, `background`, `background-color`, `background-image`, `background-size`, `background-position`, `background-repeat`, `color`, `font-family`, `font-size`, `font-weight`, `line-height`, `letter-spacing`, `text-align`, `text-transform`, `border`, `border-color`, `border-width`, `border-style`, `border-radius`, `border-top-left-radius`, `border-top-right-radius`, `box-shadow`, `opacity`, `transform`, `filter`, `backdrop-filter`, `z-index`, `overflow`, `overflow-x`, `overflow-y`, `transition`, `transition-property`, `transition-duration`, `transition-timing-function`, `transition-delay`, `animation`, `animation-name`, `animation-duration`, `animation-timing-function`, `animation-delay`, `animation-fill-mode`, `will-change`, `pointer-events`, `user-select`, `clip-path`, `align-self`, `justify-self`, `justify-items`, `layout`, `columns`, `radius`, `shadow`, `size`

- Spacing tokens: `none`, `0`, `xs`, `sm`, `small`, `md`, `medium`, `lg`, `large`, `xl`, `2xl`, `xxl`, `3xl`, `4xl`

- Size tokens: `full`, `screen`, `fit`, `min`, `max`, `content`, `readable`, `wide`, `fluid-xs`, `fluid-sm`, `fluid-md`, `fluid-lg`, `fluid-xl`, `fluid-2xl`, `fluid-3xl`

- Color tokens: `primary`, `primary-soft`, `accent`, `success`, `warning`, `danger`, `surface`, `surface-muted`, `surface-elevated`, `ink`, `subtle`, `canvas-soft`, `custom-primary`, `custom-accent`, `custom-bg`, `custom-text`, `canvas`, `text`, `muted`, `secondary`, `border`, `indigo`, `cyan`, `violet`, `emerald`, `rose`, `slate`

## Semantic Surface

- Query methods: `all`, `find`, `filter`, `count`

- Form actions: `create`, `update`, `delete`, `login`, `register`, `logout`

- Standard library capabilities: `time`, `network.outbound`, `auth`

- `time` fetch methods: `now`

- `http` fetch methods: `get`, `post`

- `auth` fetch methods: `verify`, `hash`

- Runtime/global expression names: `env`, `params`, `query`, `body`, `csrfToken`

## Codegen Surface

- Backend: `express-node`

- Standard components: `Button`, `Card`, `FeatureCard`, `PricingCard`, `Container`, `Section`, `Grid`, `Stack`, `FormField`, `Navbar`, `Hero`, `Alert`, `Footer`, `Icon`, `Modal`, `Tabs`, `Badge`, `Kpi`, `Stat`, `LogoCloud`, `TestimonialCard`, `Timeline`, `TimelineItem`, `EmptyState`, `Split`, `Cluster`, `Sidebar`, `Slides`

- Alpine/event attributes: `click`, `submit`, `change`, `input`, `keydown`, `keyup`, `focus`, `blur`, `mouseenter`, `mouseleave`

- Generated files:

- `views/<view>.ejs`
- `custom/hooks.js`
- `package.json`
- `middleware/security.js`
- `middleware/hooks-worker.js`
- `app.js`
- `views/login.ejs`
- `runtime/engine.js`
- `amana_ir.json`

- Node package scripts: `start`, `dev`

- Runtime dependencies: `express`, `express-session`, `sqlite3`, `ejs`, `argon2`, `express-rate-limit`, `helmet`

- Dev dependencies: `nodemon`

## IR

- IR version: `1.0.0`

- IR target capabilities: `sqlite_sql`, `ejs_views`, `express_routing`, `sandboxed_hooks_v1`

- IR structs: `ModelFieldIR`, `ThemeIR`, `ModelIR`, `FetchIR`, `GuardIR`, `FormActionIR`, `RouteIR`, `ViewIR`, `AppIR`, `SeedIR`, `IRVersion`, `AmanaIR`

## Feature Status Notes

- `tokens`: Implemented. Top-level tokens blocks are parsed into AST, preserved in IR, and emitted into generated theme CSS.

- `permit`: Implemented. Model permit rules are parsed into ModelDecl.permissions, preserved in IR, and enforced in generated Express REST routes, form mutations, and server fetch filtering.

- `chart`: Implemented. Chart(data, type, x, y) has parser, AST, semantic, EJS/runtime support.

- `ternary`: Implemented. Ternary expressions parse as condition ? then_value : else_value and flow through semantic/codegen/runtime.

- `persist`: Implemented. memory/local/session/cookie are parsed as PersistMode and non-memory modes emit browser persistence behavior.

- `resources`: Implemented. ResourceGrid/Table lifecycle, filters, and sort are emitted into generated EJS over server-fetched rows.

- `variants`: Implemented. Variants are parsed, validated, preserved in IR, and emitted as target-specific generated CSS for base, hover, slot, and responsive rules.

## Search Recipes

- `scripts/search-language.ps1 -Area lexer`

- `scripts/search-language.ps1 -Area parser`

- `scripts/search-language.ps1 -Area semantic`

- `scripts/search-language.ps1 -Area codegen`

- `scripts/search-language.ps1 -Area runtime`

- `python scripts/language_inventory.py --write`
