# Amana Language Reference (DSL v2.1)

Amana is a single-source domain-specific language (DSL) for building hardened full-stack web applications. An Amana source graph is compiled directly into a Node.js/Express backend with pre-compiled EJS templates, an automated SQLite migration/seed engine, and Alpine.js client-side interactivity.

---

## 🏛️ Program Structure & File Shape

An Amana application consists of top-level blocks. These blocks can be organized in a single file or modularized across multiple files using relative imports:

```amana
# 1. Imports (Optional, can be declared anywhere in the file)
import "./models/user.amana"
import "./views/dashboard.amana"

# 2. Application Definition
app CyberConsole:
    title: "Vortex Command Center"
    db_path: "cyber_console.db"
    auth_model: User
    capabilities:
        - auth
        - api.rest

# 3. Global Styling Theme
theme:
    mode: dark
    direction: ltr
    language: en
    font_provider: google
    font_family: "Space Mono"
    heading_font_family: "Orbitron"
    primary: "#10b981"      # Emerald
    accent: "#ff007f"       # Neon Pink
    canvas: "#020813"
    radius: soft
    surface: glass
    density: comfortable

# 4. Data Models & Seeding
model Ticket:
    title: str required min 3 max 100
    user_id: int required foreign_key User(id) on_delete CASCADE
    severity: str default "low"

seed Ticket:
    row:
        title: "Database flux loops"
        user_id: 1
        severity: "critical"

# 5. Route Mapping
route / -> view Home
route /tickets/[id] -> view TicketDetails

# 6. View Declaration
view Home:
    canvas:
        layout: column
        surface: glass
        density: comfortable
    client:
        state active_tab = "overview"
    server:
        fetch active_tickets = Ticket.filter(severity: "critical")
    render:
        div.app-shell:
            Navbar(brand: "Vortex")
            # Component layout and nodes...
```

---

## 📦 Modular Architecture & Imports

Multi-file systems are managed using the `import` statement.

### Flexible Import Placement
- **Location Independence**: The compiler preprocessor scans and extracts `import` directives from *anywhere* in the source file before parsing. Although placing imports at the top is a standard styling convention, they can safely appear after `app` or `theme` definitions (as seen in modular portal entrypoints).
- **Syntax**: `import "./path/to/file.amana"`
- **Relative Resolution**: All imports are resolved relative to the directory of the file containing the import statement.
- **Cycle Prevention**: Duplicate imports of the same file are deduplicated automatically when the dependency graph is constructed.

### Compilation Behavior
- **Checking**: Running `amana check main.amana` parses and semantically validates the entire imported graph.
- **Formatting**: Running `amana fmt main.amana --all` formats the entry file and recursively traverses and formats all imported files in place.
- **Building**: Compiling the entrypoint merges all model schemas, seeds, routes, and views into a single unified Express application target.

---

## ⚙️ App Configuration

The `app` block defines the system metadata and active features of the generated app:

- `title`: The `<title>` tag of all rendered HTML headers, as well as the default branding string.
- `db_path`: The filename of the auto-generated SQLite database (e.g. `"app.db"`).
- `auth_model`: Points to the model class used to handle user sessions and authentication logic.
- `capabilities`: Active feature layers. Supported values:
  - `auth`: Automatically generates secure registration, login, logout, and session middleware.
  - `api.rest`: Generates automatic CRUD REST API endpoints (`/api/v1/ModelName/`) secured by default.

---

## 🎨 Theme System & Visual Configuration

The `theme` block controls the compiler's generated design variables. These settings are compiled into root CSS custom properties (`var(--token-name)`) and passed to EJS layouts:

- `mode`: Color scheme layout (`light`, `dark`, `day`, `night`).
- `direction`: Text layout direction (`ltr` or `rtl`).
- `language`: Locale string (e.g., `en`, `ar`). Enforces correct document lang properties.
- `font_provider`: Font fetching backend (`system` or `google`).
- `font_family`: Body font family.
- `heading_font_family`: Header (`h1`, `h2`, etc.) font family.
- `arabic_font_family`: Arabic-fallback text rendering font family (injected dynamically when `direction: rtl` or `language: ar`).
- `primary` & `accent`: Base hex codes or CSS color strings.
- `canvas`, `base`, `elevated`, `text`, `muted`, `border`: System colors mapping directly to UI components.
- `radius`: Standard element curves (`sm`, `md`, `lg`, `xl`, `2xl`, or aliases like `soft` / `round` / `sharp`).
- `surface`: Default backdrop visual layer style (`base`, `elevated`, `glass`, `layered`, `glass-layered`, etc.).
- `density`: Layout margins and padding scale (`compact`, `comfortable`, `spacious`).
- `gradient_hero`: Custom CSS background linear/radial gradient definition for main views.

---

## 🗄️ Database Models & Schema Definition

Amana models map directly to SQLite tables using a secure compiler schema validator.

### Primitive Types
- `str`: Text column.
- `int`: 64-bit integer column.
- `float`: Double-precision float column.
- `bool`: Boolean column.
- `email`: Text column validated against email formats.
- `password`: Secure text column automatically hashed using **Argon2** inside form/auth middleware.
- `datetime`: SQLite timestamp representation.
- `money`: Formatted financial numeric decimal column.

### Field Constraints
- `required`: Field is non-nullable.
- `unique`: Generates unique index constraints.
- `min <number>`: Minimum string length or minimum numeric value.
- `max <number>`: Maximum string length or maximum numeric value.
- `default <expression>`: Default fallback value (e.g. `default "active"` or `default 0`).
- `foreign_key Model(field)`: Configures database relational links.
- `on_delete CASCADE`: Cascade deletion behavior for relational foreign key links.

---

## 🌱 Database Seeding

The `seed` block populates the SQLite database with starting entries during development compilation:

```amana
seed ModelName:
    row:
        field_1: "Value 1"
        field_2: 12.5
    row:
        field_1: "Value 2"
        field_2: 45.0
```

### Static Evaluation Rule
Seed expressions must be statically evaluatable at compile time.
- **Forbidden**: Seeds cannot call dynamic runtime variables, request parameters, or context states (e.g. `<Model>.current` or `params.id`).
- **Production Guard**: Seeds apply automatically in development. In production (`NODE_ENV=production`), seeds are disabled by default and require launching the server with `AMANA_RUN_SEEDS=true`.

---

## 🛣️ Routing

Routes map URL endpoints directly to view render pipelines:

```amana
route / -> view Home
route /profile -> view Profile
route /projects/[id] -> view ProjectPage
```

- **Static Routes**: Match exact URL segments.
- **Dynamic Segment Parsing**: Brackets (`/[id]`, `/posts/[slug]`) translate to Express request path parameters (`req.params.id`).
- Inside `server:` blocks, these parameters are accessible via the `params` namespace (e.g. `Project.find(params.id)`).

---

## 📺 View Declarations

Views define a route's visual structure, dynamic client state, server-side data fetching, and local styles:

### 1. `canvas` block
Sets default view settings:
```amana
canvas:
    layout: column
    surface: glass
    density: comfortable
```

### 2. `client` block
Declares client-side reactive state variables backed by **Alpine.js**:
```amana
client:
    state active_tab = "overview"
    state counter = 0
    state show_modal = false
```

### 3. `server` block
Fetches database records before rendering the view:
```amana
server:
    fetch items = Item.all(limit: 10, page: 1)
    fetch item = Item.find(params.id)
    fetch critical_issues = Issue.filter(status: "critical")
```
- Supported query types: `all()`, `find()`, `filter()`, `count()`.
- Supports pagination parameters: `limit`, `offset`, `page`. If omitted, default query limits are applied for database performance.

### 4. `render` block
Contains the HTML node structure and component call tree.

### 5. `style` block
Declares local scoped CSS styling. The CSS is compiled into scoped `@layer components, overrides` rules.

---

## 🛠️ Custom Components

Amana allows declaring custom, reusable visual components with parameter attributes and named slots:

```amana
component PortfolioCard(title: str, category: str = "Design", price: money = 0.0):
    style:
        .card:
            surface: glass
            radius: lg
            padding: xl
    render:
        div.card:
            span.category: category
            h3: title
            slot content
            slot footer optional
```

### Slot Distribution Rules
- `slot`: Default unnamed slot. Collects all children called on the component that do not match named slots.
- `slot name`: Required named slot. The compiler throws a validation error if this slot is missing when the component is called.
- `slot name optional`: Optional named slot. Will render empty if omitted.
- **Usage**:
  ```amana
  PortfolioCard(title: "Horology Chrono", category: "Luxury"):
      content:
          p: "Fine mechanical movements crafted from gold."
      footer:
          Button(): "Purchase"
  ```

---

## 🎭 Style Variants

Style variants allow extending built-in or custom components with custom visual parameters:

```amana
variant Card.glass:
    base:
        background: glass
        border: "1px solid rgba(255,255,255,0.1)"
    hover:
        shadow: floating
    responsive:
        mobile:
            padding: md
        desktop:
            padding: xl
```

---

## 📐 Design Grammar Blocks

Design blocks check visual attributes inside views, custom components, and sections. 

> [!IMPORTANT]
> **Open vs. Closed Settings Validation**:
> Only properties **`layout:`**, **`surface:`**, **`hover:`**, **`entrance:`** (or `reveal:`), **`gradient:`**, **`density:`**, and **`shadow:`** are strictly validated against closed token enums.
> All other metadata properties (such as `uniqueness:`, `freedom:`, `voice:`, `colorway:`, `direction:`, `motif:`, `lighting:`, `texture:`, `scale:`, `align:`, `feedback:`, `cursor:`, `contrast:`, `screenreader:`) are **open-ended metadata fields**. They are not checked against a closed list and accept any free-form identifier or string (under 240 characters) sanitized for safety tags (e.g., `uniqueness: facebook-clone` or `direction: pixel-perfect` are completely valid).

### 1. `compose`
Layout distribution properties:
- `layout` (Closed): `row`, `column`, `stack`, `grid`, `center`, `inline`, `cluster`, `split`, `bento`, `split-diagonal`, `asymmetric`, `editorial`, `dashboard-shell`, `magazine`, `command-center`, `showcase-rail`, `masonry`, `sidebar`.
- `ratio` (Open): Proportional columns (e.g. `"1:1"`, `"2:1"`).
- `gap` (Closed): Space scale spacing size (`xs` to `4xl`).

### 2. `visual`
Background and boundary properties:
- `surface` (Closed): `base`, `muted`, `elevated`, `glass`, `custom`, `outline`, `flat`, `layered`, `glass-layered`.
- `border` (Open): CSS border property string.
- `gradient` (Closed): `primary`, `accent`, `hero`, `mesh`, `aurora`, `spotlight`, `custom`, `brand`, `sunset`, `ocean`, `mesh-cyan-indigo`, `mesh-aurora`.

### 3. `creative`
Brand art style choices:
- `uniqueness` (Open): Uniqueness class name (e.g. `signature`, `standard`, `custom`, `facebook-clone`).
- `freedom` (Open): Layout freedom style (e.g. `high`, `medium`, `low`, `custom`).

### 4. `brand`
Editorial style voice:
- `voice` (Open): Brand editorial voice (e.g. `technical`, `luxury`, `friendly`, `artistic`, `professional`, `authentic`).
- `colorway` (Open): Theme color scheme (e.g. `"forest"`, `"dark-blue"`, `"cyan-neon-pink"`).

### 5. `art`
Visual assets motif:
- `direction` (Open): Artistic direction (e.g. `cyberpunk`, `classical`, `organic`, `expressive`, `modern`, `pixel-perfect`).
- `motif` (Open): Design texture motif (e.g. `reactor-interface`, `clean`, `social-feed`).
- `lighting` (Open): Theme lighting properties (e.g. `dark-neon`, `light-airy`).
- `texture` (Open): Surface texture layout (e.g. `translucent`, `opaque`).

### 6. `responsive`
Adaptive sizing:
- `desktop_columns` / `mobile_columns` (Open): CSS column rules.
- `desktop` / `mobile` (Open): Sub-blocks declaring local responsive property overrides (e.g., `padding`, `gap`, `columns`).

### 7. `type`
Typography alignment:
- `scale` (Open): Typographic height scale (e.g. `spacious`, `balanced`, `compact`).
- `align` (Open): Text orientation (e.g. `left`, `center`, `right`).

### 8. `motion`
Transitions and hover animations:
- `entrance` (Closed): `fade`, `slide-up`, `slide-down`, `zoom`, `blur`, `clip`, `stagger-up`, `none`.
- `hover` (Closed): `lift`, `glow`, `scale`, `lift-glow`, `none`.

### 9. `interaction`
Pointer interface:
- `feedback` (Open): Interactive click feedback (e.g. `tactile`, `ripple`, `hover-highlight`).
- `cursor` (Open): Pointer styling (e.g. `default`, `pointer`).

### 10. `a11y`
Accessibility and colors:
- `contrast` (Open): Color contrast standards (e.g. `high`, `aaa`, `dark-accessible`).
- `screenreader` (Open): Accessibility reader bindings (e.g. `ready`, `structured`).

---

## 🚫 Layout Constraints

The `compose` design block enforces layout configuration constraints. Only validated keys are allowed depending on the configured layout engine:

| Layout (`layout:`) | Allowed Keys |
| :--- | :--- |
| `bento` | `layout`, `columns`, `rows`, `gap`, `auto_place`, `responsive`, `rhythm`, `focus_path`, `density` |
| `masonry` | `layout`, `columns`, `image_ratio`, `gap` |
| `split` | `layout`, `ratio`, `align`, `visual_position` |
| `asymmetric` | `layout`, `rhythm`, `dominant`, `overlap` |
| `magazine` | `layout`, `columns`, `headline_span`, `aside_span`, `pull_quote` |
| `sidebar` | `layout`, `sidebar_width`, `sidebar_position`, `sticky_sidebar` |
| `timeline` | `layout`, `axis`, `marker`, `alternate` |
| `dashboard-shell` | `layout`, `sidebar`, `topbar`, `content_width`, `density`, `rhythm` |
