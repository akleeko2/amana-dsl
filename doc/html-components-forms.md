# HTML Rendering, Components, And Forms

This document describes the render tree and form system implemented by `src/parser/views.rs`, `src/codegen/html.rs`, and `src/semantic/views.rs`.

## Render Elements

```amana
render:
    div.page:
        h1: "Title"
        p.lead: f"Hello {User.current.email}"
        Button(label: "Open", click: open = true)
```

Rules:

- Lowercase tags render as HTML tags.
- Uppercase tags are treated as standard or custom components.
- Classes use dot syntax: `div.card.featured`.
- Hyphenated tags/classes are accepted through minus tokens.
- Attributes use `name: expression`.
- Text after `:` can be a string literal or expression.
- Nested children require a trailing `:` and indentation.
- A component with no children should be self-closing: `Navbar()`, not `Navbar():`.

Blocked lowercase tags:

```text
script iframe object embed applet link meta base style noscript
```

## Standard Components

The current codegen recognizes 58 built-in components:

### Layer 1: Layout Primitives
`Container`, `Section`, `Stack`, `Grid`, `Split`, `Cluster`, `Sidebar`, `Center`, `Cover`, `Reel`, `Masonry`

### Layer 2: Basic UI & Interaction Primitives
`Button`, `Card`, `FeatureCard`, `PricingCard`, `FormField`, `Modal`, `Alert`, `Badge`, `Kpi`, `Stat`, `Icon`, `Accordion`, `EmptyState`, `Tabs`, `Skeleton`, `LoadingState`, `ErrorState`, `OfflineState`, `Toast`, `Banner`

### Layer 3: Application Blocks
`Navbar`, `Hero`, `Footer`, `Timeline`, `TimelineItem`, `LogoCloud`, `TestimonialCard`, `Slides`

### Layer 4: Patterns (Core Shells & Pages)
`DashboardShell`, `AuthPage`, `PricingSection`

### Navigation & Data Experience
`Breadcrumb`, `Dropdown`, `CommandPalette`, `SearchBar`, `FilterBar`, `Paginator`, `DataTable`

### Advanced Interaction & Pages
`FileUpload`, `RichEditor`, `ColorPicker`, `HeroSection`, `SettingsPage`, `StatsSection`, `FAQSection`, `BlogSection`, `TestimonialsSection`, `ContactSection`


### Component Attributes

`Button`

- `href`: renders `<a>` when present, otherwise `<button type="button">`.
- `label` or `text`: button text when no children are provided.
- `variant`: CSS variant class suffix; default `primary`.
- `size`: CSS size class suffix; default `md`.
- `intent`: intent class suffix; default `default`.
- `icon`: rendered through Iconify when in `prefix:name` form, otherwise fallback text.

`Card`, `FeatureCard`, `PricingCard`

- `eyebrow`, `badge`, `title`, `subtitle`, `description`, `price`, `meta`
- `action_label`, `action_href`
- `density`: default `comfortable`
- Children render inside the card body.

`Container`

- `width`: class suffix such as `default`, `wide`, or `readable`.

`Section`

- `eyebrow`, `title`, `subtitle`, `description`
- Children render after the generated section header.

`Grid`

- `min`: CSS variable `--grid-min`; default `16rem`.
- `columns`: compiles to `--dg-columns`. If specified as a raw number (e.g., `"3"`), it compiles to `repeat(3, minmax(0, 1fr))`. If specified as `"1"`, it compiles to `minmax(0, 1fr)`. Responsive columns (e.g. `responsive: mobile: columns: 1` nested block) behave similarly, compiling to `--bp-mobile-columns:minmax(0, 1fr)`. No raw numeric columns reach the compiled output.

`Stack`

- `gap`: stack gap class suffix; default `md`.

`FormField`

- `name`, `label`, `placeholder`, `type`, `help`, `required`.
- If `type: "textarea"` is supplied, or inside form options `field name: type: textarea` is used, the component compiles to a `<textarea class="amana-form-control" rows="4">` instead of a standard input tag.

`Navbar`

- `brand`: default EJS title.
- `sticky`: `true` adds sticky class.
- `variant`: class suffix.
- `links`: comma-separated `Text /path` entries. If omitted, children become nav links/body.

`Hero`

- `eyebrow`, `title`, `subtitle`, `media`, `proof`.
- Children render inside the hero action area.

`Alert`

- `tone`: default `info`.
- `message`: used when no children are provided.

`Footer`, `Timeline`, `Split`, `Cluster`, `Sidebar`

- Wrapper components around children.

`Icon`

- `name` or `icon`.

`Modal`

- `open`: Alpine expression controlling `x-show`; default `modal_open`.
- `title`: Optional title text.
- `closable`: Optional boolean (default `true`) to render a close button.

`Badge`

- `label`: uses children when omitted.
- `tone`: default `neutral`.

`Kpi`, `Stat`

- `label`, `value`, `trend`.
- KPIs feature structured values and wrapper styles compiled to `.amana-kpi-value`.

### Phase 2 Visual Preset overrides on Standard Components
These styles are compiled as baseline CSS overrides rather than a new theme system:
- **Navbar**: `variant: "glass"` emits `.amana-navbar-glass` with backdrop-filter blur support.
- **Timeline**: Direction-aware timeline markers are rendered dynamically. Under LTR, `.amana-timeline-item::before` aligns left; under RTL, `[dir="rtl"] .amana-timeline-item::before` shifts markers to the right.
- **PricingCard**: `variant: "featured"` compiles to `.amana-pricing-card.amana-variant-featured`.
- **Button**: Handles focus/active states with `.amana-btn:focus-visible` and `.amana-btn:active` rules.
- **Card**: Hover transition lifts styled under `.amana-card:hover`.

`LogoCloud`

- `title`.

`TestimonialCard`

- `quote`, `author`, `role`.

`TimelineItem`

- `title`, `meta`.

`EmptyState`

- `title`, `description`, `action_label`, `action_href`.

`Slides`

- `autoplay`: `true` enables interval rotation.
- `height`: default `400px`.
- `effect`: `fade` or slide-style transition.

### New Components (v3.0)

#### Layout Components
- **`Center`**: Vertically and horizontally centers content. Takes `max_width` (e.g. `60rem`).
- **`Cover`**: Renders full-height blocks (default `min_height: 100vh`).
- **`Reel`**: Horizontal scrolling container for list items. Takes `gap` (default `1.5rem`).
- **`Masonry`**: Renders dynamic staggered columns. Takes `columns` (default `3`) and `gap` (default `1.5rem`).

#### State & Feedback Systems
- **`Skeleton`**: Loading shimmers. Takes `type` (`text`, `avatar`, `card`) and `lines` (for text, default `1`).
- **`LoadingState`**: General data loading placeholder with spinner. Takes `rows` of skeleton lines.
- **`ErrorState`**: Error message block with try-again action button. Takes `code`, `title`, `description`, `action_label`, and `action_href`.
- **`OfflineState`**: Connection status indicator using Alpine.js. Takes `title`, `description`, and `force`.
- **`Toast`**: Client-side auto-dismiss toast alerts. Takes `message`, `tone` (`success`, `info`, `warning`, `danger`), `show` variable, and `duration` (default `3000`).
- **`Banner`**: Inline persistent warning banner. Takes `tone` (`success`, `info`, `warning`, `danger`) and `closable` (boolean).

#### Core Page Shells & Sections
- **`DashboardShell`**: Layout shell for administrative portals. Takes `brand`, `user`, `logo`, with slots `sidebar:` and `main:`.
- **`AuthPage`**: Pre-configured login/signup card template. Takes `type` (`login` or `signup`), `title`, and `logo`.
- **`PricingSection`**: Pricing comparison section. Takes `title` and `billing` default toggle ("monthly" or "yearly"), and supports child `plan` tags.

#### Navigation & Data Experience
- **`Breadcrumb`**: Breadcrumb history navigation path.
- **`Dropdown`**: Simple toggle dropdown panel.
- **`CommandPalette`**: Triggerable modal command/search box with keyboard traps.
- **`SearchBar`**: Text search bar with debounce query bindings.
- **`FilterBar`**: Responsive layout for aligning dropdown/list filters in one line.
- **`Paginator`**: Pagination toolbar for tables.
- **`DataTable`**: Interactive client-side sorted table with select-all rows checkboxes.

#### Advanced Forms & Page Sections
- **`FileUpload`**: Drag-and-drop file upload with preview.
- **`RichEditor`**: Simple formatting rich text box editor.
- **`ColorPicker`**: Custom palette color chooser.
- **`HeroSection`, `SettingsPage`, `StatsSection`, `FAQSection`, `BlogSection`, `TestimonialsSection`, `ContactSection`**: Production-ready content sections.

## Custom Components

```amana
component ProjectCard(project):
    style:
        .card:
            surface: elevated
            padding: lg
    render:
        article.card:
            h3: project.name
            slot footer optional
```

Rules:

- Parameters may be typed: `title: str`.
- Parameters with defaults are optional.
- Required parameters must be supplied at call sites.
- Component bodies are inlined during IR generation.
- Component styles are aggregated into every view that uses the component.
- Required slots must be supplied by a matching child block.

Slot examples:

```amana
slot:
slot footer
slot actions optional
```

Call with named slot:

```amana
ProjectCard(project: selected):
    footer:
        Button(label: "Open", href: "/projects")
```

## Alpine Bindings

Event attributes:

```text
click submit change input keydown keyup focus blur mouseenter mouseleave
```

Special attributes:

- `bind`: binds an input to a server/render variable or client state.
- `model`: emits `x-model`.
- `show`: emits `x-show`.
- `text`: emits `x-text`.
- `init`: emits `x-init`.
- `disabled`, `checked`, `selected`, `readonly`: emitted as dynamic boolean bindings.

Example:

```amana
view Counter:
    client:
        state count = 0
        state open = false
    render:
        div:
            Button(label: "Add", click: count = count + 1)
            p(text: count): ""
            div(show: open):
                p: "Visible"
```

## Forms

```amana
form [email, password]:
    connect User.login
    submit: "Sign in"
    redirect success -> /dashboard
    field email:
        label: "Email"
        type: email
        required: true
```

Supported settings:

- `connect Model.action`
- `redirect success -> /path`
- `default field = expression`
- `where field = expression`
- `ui: value`
- `submit: value`
- `field name:` with `label`, `placeholder`, `type`, `help`, `required`

Supported actions:

```text
create update delete login register logout
```

Validation:

- The connected model must exist.
- All listed fields must exist on the model, except implicit `id`.
- `login` must include `email` and `password`.
- `login`, `register`, and `logout` must target the configured `auth_model`.
- `where` constraints are allowed only for `update` and `delete`.
- `update` and `delete` require an `id` field.
- Defaults and constraints using `User.current` require a protected view.

Runtime:

- Forms post to `/form-submit/<model>/<action>`.
- CSRF token inputs are injected automatically.
- Password fields are hashed with Argon2.
- `default` expressions are evaluated at submit time.
- `where` expressions become authorization filters for update/delete.

## Resource Blocks

```amana
ResourceTable(projects):
    item ProjectRow(project)
    empty:
        p: "No projects"
    loading:
        p: "Loading"
    error:
        p: "Error"
    filters:
        - status
    sort:
        - name
```

Rules:

- Resource expression must be a list.
- `item` component must be a declared custom component.
- `empty`, `loading`, `error`, `filters`, and `sort` are fully compiled and rendered at runtime.
- The compiled EJS generates Alpine JS bindings (`x-data="amanaResource()"`) containing state machines for `loading` and `error` and evaluates user-input search queries (`x-model.debounce.200ms="filters.<field>"`) and sort priorities (`x-model="sortField"`) on the client side directly over the loaded dataset.

## Interactive DSL Layout Primitives

Amana's interactive layout primitives compile directly into EJS templates with Alpine.js reactive bindings:

### 1. Tabs Primitive (`Tabs:`)
- **Structure**: Renders a dynamic header list of tab buttons followed by the corresponding tab panels.
- **Compiled Target**:
  - HTML Wrapper: `<div x-data="{ active_tab: 0 }" class="amana-tabs">`
  - Headers: A list of `<button>` tags with click handler `@click="active_tab = <index>"` and dynamic styling `:class="{ 'active': active_tab === <index> }"`.
  - Body: A series of panel container `<div>` tags, each with layout display toggled via `x-show="active_tab === <index>"`.
- **Reactivity**: Tab switching runs entirely on the client, avoiding server roundtrips.

### 2. Accordion Primitive (`Accordion:`)
- **Structure**: Renders an expandable/collapsible list of panel items.
- **Compiled Target**:
  - HTML Wrapper: `<div x-data="{ active_panel: null }" class="amana-accordion">`
  - Item Containers: Each panel is wrapped in a `.amana-accordion-item` container.
  - Header Toggle: A `<button class="amana-accordion-header">` with click handler `@click="active_panel = (active_panel === <index> ? null : <index>)"`. Features an SVG chevron rotated using `:class="{ 'rotate-180': active_panel === <index> }"`.
  - Content Panel: An inner `<div x-show="active_panel === <index>" class="amana-accordion-content">`.
- **Reactivity**: Opening a panel automatically collapses the previously opened panel.

### 3. Collapsible Sections (`[collapsible: true]`)
- **Structure**: Converts any layout block (like `section` or custom cards) into a collapsible unit.
- **Compiled Target**:
  - HTML Wrapper: `<div x-data="{ open: <default_open_boolean> }" class="amana-collapsible">`
  - Clickable Header Toggle: Automatically selects the first child element of the block as the toggle header, attaching `@click="open = !open"` and appending a transition-capable chevron marker.
  - Collapsible Body: Wraps all subsequent children inside a container `<div x-show="open" class="amana-collapsible-body">` controlled by Alpine's reactive visibility.

## State Scope Wrapper

To prevent viewports from stretching and breaking nested container scrollbars, the compiler generator wraps every EJS view containing client states in a classed wrapper:
```html
<div class="amana-state-scope" x-data="...">
    <!-- View elements -->
</div>
```
- **CSS Isolation**: Styled under `.amana-state-scope` with flex column layouts (`height: 100%; min-height: 0; display: flex; flex-direction: column;`). This prevents the EJS output from collapsing to `height: auto` and isolates it from the outer layout grids, ensuring scrollable child elements (like tables or logs) scroll correctly within their viewport boundaries.
