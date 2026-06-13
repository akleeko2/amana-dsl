# Amana Language Reference

This document describes the current language surface supported by the compiler.

## File Shape

```amana
import "./models/user.amana"

app MyApp:
    title: "My App"
    db_path: "my_app.db"
    auth_model: User
    capabilities:
        - auth
        - api.rest

theme:
    mode: dark
    direction: rtl
    language: ar
    font_provider: google
    font_family: "Noto Sans Arabic"
    heading_font_family: "Space Grotesk"
    primary: indigo
    accent: cyan
    radius: soft
    surface: glass
    density: comfortable

model User:
    name: str required min 2 max 80
    email: email unique required
    password: password required min 8

route / -> view Home
route /projects/[id] -> view ProjectPage
```

## app

`app` defines application metadata:

- `title`
- `db_path`
- `auth_model`
- `capabilities`

## imports

Use `import "./file.amana"` for multi-file projects. Imports resolve relative to the importing file.

## theme

The theme block feeds the CSS generator and runtime variables.

Supported keys include:

- `mode`: `light`, `dark`, `day`, `night`
- `direction`: `ltr`, `rtl`
- `language`: locale like `en` or `ar`
- `font_provider`: `system` or `google`
- `font_family`
- `heading_font_family`
- `arabic_font_family`
- `primary`
- `accent`
- `canvas`
- `text`
- `radius`
- `surface`
- `density`

`font_provider: google` enables Google Fonts import generation. The family names are still configurable.

## models

```amana
model Project:
    owner_id: int required
    name: str required min 2 max 100
    status: str default "active"
    created_at: datetime default "CURRENT_TIMESTAMP"
```

Supported types:

- `str`
- `int`
- `float`
- `bool`
- `email`
- `password`
- `datetime`
- `money`

Supported field flags:

- `required`
- `unique`
- `min <number>`
- `max <number>`
- `default "value"`
- `foreign_key Model(field)`
- `on_delete CASCADE`

## routes

```amana
route / -> view Home
route /projects/[id] -> view ProjectPage
```

Bracket segments become Express params. For example:

- `/projects/[id]` becomes `req.params.id`
- `Project.find(params.id)` is valid in `server:` blocks

## views

```amana
view ProjectsPage:
    server:
        fetch projects = Project.all(limit: 20, page: 1)
        fetch project = Project.find(params.id)

    render:
        div.page:
            Navbar(brand: "ProjectHub", sticky: true)
            Container():
                Grid(min: "18rem"):
                    for project in projects:
                        Card(title: project.name, subtitle: project.status)

    style:
        .page:
            background: canvas
            color: text
            min-height: screen
```

### server fetch

Supported methods:

- `all`
- `find`
- `filter`
- `count`

Pagination keys:

- `limit`
- `offset`
- `page`

`all` and `filter` default to a safe limit if none is provided.

## component calls

If a component has no children, write it without `:`

```amana
Navbar(brand: "Amana")
Footer()
```

If it has children, use `:`:

```amana
Card(title: "Hello"):
    p: "Content"
```

## standard components

Current standard library components include:

`Button`, `Card`, `Container`, `Section`, `Grid`, `Stack`, `Navbar`, `Hero`, `FeatureCard`, `PricingCard`, `FormField`, `Alert`, `Footer`, `Modal`, `Tabs`, `Badge`, `Kpi`, `Stat`, `LogoCloud`, `TestimonialCard`, `Timeline`, `TimelineItem`, `EmptyState`, `Split`, `Cluster`, `Sidebar`, `Icon`

Key updates:
- **Navbar** accepts `links: "Label /path, Label2 /path2"` for clean navigation links layout.
- **FormField** accepts `required: true` (adds asterisk and aria-required) and `help: "Helper description text"`.

## security tag restrictions

Raw HTML tags that present security risks (like `<script>`, `<iframe>`, `<style>`, `<link>`, `<meta>`, `<base>`, `<object>`, `<embed>`, `<applet>`, `<noscript>`) are disallowed inside Amana views. The compiler will reject these elements with a `Security` error to prevent injection vulnerabilities.

## custom components (Amana v2)

Define custom, reusable UI components with local styling and parameters:

```amana
component FeatureCard(title: str, icon: str = "star", label: str = ""):
    style:
        .card:
            layout: stack
            gap: md
            surface: elevated
            radius: xl
            padding: lg
    render:
        div.card:
            Badge(label: label)
            h3: title
            slot content
            slot actions optional
```

## slots resolution

Distribute content dynamically inside custom components using named and optional slots.

When declaring slots in a component body:
- `slot name`: Declares a required slot.
- `slot name optional`: Declares an optional slot.
- `slot:` or `slot`: Declares a legacy default unnamed slot.

When calling the component, supply slot fillers by matching child tag names:

```amana
FeatureCard(title: "أحدث الطلبات", icon: "shopping-bag"):
    content:
        p: "عرض وتدقيق كافة المعاملات الأخيرة الواردة للمتجر."
    actions:
        Button(href: "/orders"): "التفاصيل"
```

## variants styling registry

Customize standard or custom component styles under different aesthetic conditions:

```amana
variant Card.glass:
    base:
        background: glass
        radius: large
        border: smooth
    hover:
        shadow: smooth
    responsive:
        mobile:
            padding: md
        desktop:
            padding: xl
```

## layout-specific grammars

Layout blocks check configuration properties strictly. Supported layout engines and their allowed settings:

- **bento**: `layout`, `columns`, `rows`, `gap`, `auto_place`, `responsive`, `rhythm`, `focus_path`, `density`
- **masonry**: `layout`, `columns`, `image_ratio`, `gap`
- **split**: `layout`, `ratio`, `align`, `visual_position`
- **asymmetric**: `layout`, `rhythm`, `dominant`, `overlap`
- **magazine**: `layout`, `columns`, `headline_span`, `aside_span`, `pull_quote`
- **sidebar**: `layout`, `sidebar_width`, `sidebar_position`, `sticky_sidebar`
- **timeline**: `layout`, `axis`, `marker`, `alternate`
- **dashboard-shell**: `layout`, `sidebar`, `topbar`, `content_width`, `density`

## custom css & 4-layer sanitizer

Custom component, view, and variant CSS blocks are normalized and rewritten by the compiler to prevent style pollution and secure applications:

1. **Selector Safety Validator**: Globally blocked tags (`body`, `html`, `*`, `script`, etc.) and attribute selectors (`[onclick]`) are rejected.
2. **Property Safety Allowlist**: Restricted to modern, safe layout, sizing, typography, and painting properties.
3. **Value Sanitizer**: Strips dangerous patterns (`javascript:`, `expression(`, `url(...)`, etc.).
4. **CSS Layers Isolation**: Output is scoped and isolated into `@layer components, variants, overrides` to ensure predictable rendering order.

### Parser & Syntax Enhancements (Amana v2.0.3)

- **Hyphenated Element Tags**: You can use elements and custom elements containing hyphens (e.g. `iconify-icon`) directly inside views. The parser automatically scans and reconstructs hyphenated element tags.
- **Multi-line Grouped Selectors**: Grouped CSS selectors separated by commas can span across multiple lines to improve readability (e.g. `.class1, \n .class2:`). The parser consumes the trailing newline automatically.
- **Keyframe and Media Query Nesting Constraint**: Nested blocks (like `@keyframes` percentages or `@media` queries) are **not supported** in the flat `.amana` stylesheet block due to the single-level selector-declaration mapping. Keyframes or media-specific behaviors should be implemented using global overrides, pre-built utility classes, or custom plugins if required.
