# Amana Language Reference

This document describes the Amana language as implemented in the compiler source. It does not use the old examples or tests as authority. For exhaustive generated lists, see [language-inventory.generated.md](language-inventory.generated.md).

## Compiler Pipeline

An `.amana` source graph is compiled through these stages:

1. Import resolution in `src/main.rs`.
2. Lexing in `src/lexer/mod.rs`.
3. Parsing in `src/parser/*`.
4. Semantic validation in `src/semantic/*`.
5. AST optimization and IR generation in `src/semantic/optimizer.rs` and `src/semantic/ir_gen.rs`.
6. Express/EJS/SQLite project generation in `src/codegen/*`.

The generated backend target is currently `express-node`.

## File Graph And Imports

Imports are handled before lexing/parsing each file.

```amana
import "./models.amana"
import "./views/home.amana"
```

Rules:

- Imports may appear anywhere in a file. The preprocessor strips import lines before parsing.
- Paths must be quoted strings.
- Relative imports are resolved from the importing file's directory.
- Duplicate imports are deduplicated by canonical path.
- Circular imports fail with an imports-stage diagnostic.
- Duplicate top-level public symbols across the resolved graph fail before IR generation. `model`, `view`, and `component` share one symbol namespace; route paths and variant targets are checked separately.
- `amana fmt --all` traverses the same import graph.

## Lexical Rules

Amana is indentation-sensitive.

- Spaces are required for indentation. Tabs are lexer errors.
- Blank lines and full-line comments are skipped.
- `#` starts a comment unless followed by an ASCII alphanumeric character, in which case it is lexed as a `HashColor`.
- Identifiers are ASCII letters, digits, and underscores; the first character must be a letter or underscore.
- Strings support `"single line"`, `"""multi-line"""`, and formatted strings like `f"Hello {name}"`.
- Numbers are parsed as `f64` internally.
- Route arrows use `->`; the lexer also recognizes the Unicode arrow if present.

Reserved words:

```text
app model route view component protected server client render state form
if else for in permit fetch style variant slot optional tokens
str int float bool email password datetime money true false null and or not
```

Operators and punctuation:

```text
+ - * / == != > < >= <= = ? : . , -> % ( ) [ ]
```

Ternary expressions using `condition ? then_value : else_value` are parsed and emitted through the runtime expression path.

## Top-Level Blocks

Implemented top-level parse nodes:

```text
app, theme, model, seed, route, view, component, variant, tokens
```

Current status:

| Block | Status |
| --- | --- |
| `app` | Parsed, validated indirectly, emitted into IR/runtime. |
| `theme` | Parsed, semantically validated, emitted into runtime CSS. |
| `model` | Parsed, validated through usage, emitted into SQLite schema. |
| `seed` | Parsed, semantically validated, emitted into runtime seed engine. |
| `route` | Parsed, validated through referenced views, emitted into Express routes. |
| `view` | Parsed, semantically validated, emitted into EJS views. |
| `component` | Parsed, semantically validated when used, inlined into view IR. |
| `variant` | Parsed, semantically validated, preserved in IR, and emitted as target-specific runtime CSS. |
| `tokens` | Parsed into AST, preserved in IR, and emitted as generated CSS variables. |
| `permit` inside `model` | Parsed into `ModelDecl.permissions`, emitted into IR, and enforced by the generated runtime for REST, forms, and server fetches. |

## App Block

```amana
app AdminPanel:
    title: "Admin Panel"
    db_path: "admin.db"
    auth_model: User
    capabilities:
        - auth
        - api.rest
        - time
```

Supported keys:

- `title`: string; defaults to app name.
- `db_path`: string; defaults to `app.db`.
- `auth_model`: identifier; defaults to `User`.
- `capabilities`: bracket list of strings or indented dash list. Dotted names such as `network.outbound` are supported in the dash form.

Recognized runtime capabilities:

- `auth`: enables auth standard library access and auth-oriented form actions against the configured auth model.
- `api.rest`: emits REST endpoints under `/api/<table>`.
- `time`: allows server fetches from `time.now()`.
- `network.outbound`: allows server fetches from `http.get(...)` and `http.post(...)`.

If no `app` block is present, the compiler uses:

```text
name = AmanaApp
title = Amana Generated App
db_path = app.db
auth_model = User
capabilities = []
```

## Theme Block

```amana
theme:
    mode: dark
    direction: rtl
    language: ar
    font_provider: google
    font_family: "Inter"
    arabic_font_family: "Tajawal"
    primary: "#4f46e5"
    accent: "#06b6d4"
    radius: soft
    density: comfortable
```

Theme keys are allowlisted. Unknown keys fail semantic validation and may include a spelling suggestion.

> [!IMPORTANT]
> No new theme system or custom component registry is supported. The baseline fallback theme remains `indigo`/`cyan`.

Closed-value keys:

- `mode`: `dark`, `night`, `day`, `light`.
- `direction`: `ltr`, `rtl`.
- `radius`: `none`, `sharp`, `soft`, `round`, `pill`.
- `density`: `compact`, `comfortable`, `spacious`.
- `font_provider`: `system`, `google`.

Theme value safety:

- Values containing `javascript:`, `expression(`, `<script`, `</style`, `behavior:`, `;`, `{`, or `}` are rejected.
- Font values have an 80-character limit and restricted characters.
- Other values have a 260-character limit and restricted characters.

See the generated inventory for the complete key list.

## Models

```amana
model User:
    email: email unique required
    password: password required min 8
    role: str default "member"

model Project:
    name: str required max 120
    owner_id: int foreign_key User(id) on_delete CASCADE
```

Types:

- `str`: SQLite `TEXT`.
- `int`: SQLite `INTEGER`.
- `float`: SQLite `REAL`.
- `bool`: SQLite `INTEGER`.
- `email`: SQLite `TEXT`; treated as string-compatible.
- `password`: SQLite `TEXT`; runtime hashes submitted values with Argon2.
- `datetime`: SQLite `TEXT`.
- `money`: SQLite `REAL`.
- Custom identifiers are parsed as `DataType::Custom` and emitted as `TEXT` in SQL.

Field modifiers:

- `primary_key`
- `unique`
- `required`
- `min <number>` or `min: <number>`
- `max <number>` or `max: <number>`
- `default "value"`, `default 10`, `default true`
- `foreign_key Model(field)`
- `on_delete CASCADE`
- `on_delete SET NULL`

Legacy bracket modifiers are also parsed:

```amana
model User:
    email: email [unique, required]
    age: int [min: 13, max: 130]
    team_id: int [foreign_key: Team.id, on_delete: CASCADE]
```

SQL generation:

- If no explicit primary key exists, codegen adds `"id" INTEGER PRIMARY KEY AUTOINCREMENT`.
- Field names and table names are lowercased in generated SQL.
- Table and column names are quoted to avoid reserved-word collisions.
- `required` emits `NOT NULL` unless the field has a default or is primary key.
- `min` and `max` emit SQL `CHECK` constraints. Text-like fields use `length(field)`.

Model permissions:

```amana
model Project:
    name: str
    secret: str
    owner_id: int
    permit Manager read Project where owner_id = User.current.id fields [name]
    permit Manager update Project where owner_id = User.current.id fields [name]
```

- A model with any `permit` rule is default-deny for that model.
- `where` is row-level policy evaluated against the current row, submitted values, request scope, and current principal.
- `fields` on `read` masks output fields. Allowed rows still include `id`; other fields are returned only when listed.
- `fields` on `create` and `update` is a write allowlist. Submitted fields outside the matching rules are rejected.
- `manage` and `*` match all actions; action aliases such as `list`, `find`, `write`, and `edit` are normalized by the runtime.

## Seeds

Two seed shapes are accepted.

```amana
seed PricingPlan:
    row:
        name: "Starter"
        price: 0
    row:
        name: "Pro"
        price: 19
```

Single-row shorthand:

```amana
seed PricingPlan:
    name: "Starter"
    price: 0
```

Validation:

- The model must exist.
- Fields must exist on the model.
- Duplicate fields in a row are rejected.
- Required fields without defaults must be present.
- Seed expression types must be compatible with field types.
- `<auth_model>.current`, `User.current`, and equivalent auth-current expressions are forbidden in seeds because seeds run without a request session.

Runtime behavior:

- Seeds run by default outside production.
- In production, seeds are skipped unless `AMANA_RUN_SEEDS=true`.
- Password fields are hashed before insertion.

## Routes

Simple route:

```amana
route / -> view Home
route /projects/[id] -> view ProjectDetails
```

Block route:

```amana
route /admin:
    guard User.current != null else redirect /login
    fetch users = User.all(limit: 50)
    view Admin
```

Rules:

- Route paths must start with `/`.
- Paths may contain identifiers, numbers, `-`, `/`, and bracket parameters such as `[id]`.
- Bracket parameters become Express params and are readable through `params.id`.
- Simple routes map directly to a view.
- Block routes may declare `guard`, route-level `fetch`, and `view`.
- If a view also has `protected:`, the view-level guard is used in IR; otherwise the first route-level guard is used.

## Views

```amana
view Projects:
    protected:
        allow: User.current != null
        deny: -> /login
        unauthenticated: -> /login

    canvas:
        layout: column
        density: comfortable

    server:
        fetch projects = Project.filter(owner_id: User.current.id, limit: 50)

    client:
        state filter_open = false [persist: local]

    render:
        div.page:
            h1: "Projects"

    style:
        .page:
            layout: column
            gap: lg
```

Supported view blocks:

- `protected`
- `canvas`
- `server`
- `client`
- `render`
- `style`

### Protected

```amana
protected:
    allow: User.current != null
    deny: -> /forbidden
    unauthenticated: -> /login
```

`allow` must evaluate to `bool`. `deny` and `unauthenticated` paths are used by runtime guards.

### Server Fetches

```amana
server:
    fetch all_projects = Project.all(limit: 100)
    fetch page_projects = Project.all(limit: 20, page: 2)
    fetch project = Project.find(params.id)
    fetch active = Project.filter(status: "active", offset: 40)
    fetch total = Project.count(status: "active")
```

Query methods:

- `all(limit:, offset:, page:)`: accepts only pagination arguments.
- `find(id)`: requires one positional id argument.
- `filter(field: value, limit:, offset:, page:)`: accepts named field filters and pagination.
- `count(field: value)`: accepts named field filters.

Pagination:

- Default limit is `100`.
- `offset` and `page` cannot be used together.
- `page` compiles to `OFFSET ((page - 1) * limit)`.

Standard library fetches:

```amana
server:
    fetch now = time.now()
    fetch payload = http.get(env("API_URL"))
    fetch valid = auth.verify(hash_value, password_value)
```

Required capabilities:

- `time.now()` requires `time`.
- `http.get(...)` and `http.post(...)` require `network.outbound`.
- `auth.verify(...)` and `auth.hash(...)` require `auth`.

Standard libraries are not allowed inside `render:` blocks. They must be used through server fetches.

### Client State

```amana
client:
    state count = 0
    state open = false [persist: local]
```

The parser records `persist` values as `memory`, `cookie`, `session`, or `local`. Generated EJS uses Alpine `x-data`; non-memory states hydrate from and watch browser storage.

### Render Tree

Lowercase names render as HTML tags unless blocked by security validation. Uppercase names are treated as standard or custom component calls.

```amana
render:
    div.page:
        h1: "Dashboard"
        p: f"Hello {User.current.email}"
        Button(label: "Open", click: open = true)
```

Element rules:

- Classes use dot syntax: `div.card.featured`.
- Hyphenated tags and classes are supported through minus tokens: `main-shell.hero-card`.
- Attributes use parentheses: `a(href: "/docs", click: count = count + 1)`.
- Anchors and IDs: Native ID and anchor link paths are supported (e.g. `section(id: "features")` and `a(href: "#features")`). They translate to EJS-scoped HTML attributes (`id="<%= "features" %>"` and `href="<%= "#features" %>"`).
- A colon starts child or text content.
- A component with no children should be written without a trailing colon: `Navbar()`.
- `amana fmt` removes empty trailing colons from self-closing component calls.

Blocked raw HTML tags:

```text
script iframe object embed applet link meta base style noscript
```

### Interactive DSL Layout Primitives

Amana features compiler-supported layout primitives that compile to dynamic, responsive EJS/Alpine.js structures. Unlike standard components, these primitives have dedicated parser syntax and validation:

1. **Tabs Primitive (`Tabs:`)**: Group related layout panels under click-switchable tab headers.
   - *Syntax*: Declared using `Tabs:` block with nested `tab "Tab Title":` child elements.
   - *Restrictions*: The compiler checks that all children of a `Tabs:` block are valid `tab` declarations.
   - *Example*:
     ```amana
     Tabs:
         tab "Overview":
             p: "Dashboard summary"
         tab "Performance":
             p: "Performance chart"
     ```

2. **Accordion Primitive (`Accordion:`)**: Render an expandable and collapsible list of panels.
   - *Syntax*: Declared using `Accordion:` block with nested `panel "Panel Title":` child elements.
   - *Restrictions*: The compiler checks that all children of an `Accordion:` block are valid `panel` declarations.
   - *Example*:
     ```amana
     Accordion:
         panel "General Information":
             p: "SLA details"
         panel "Logs":
             p: "System logs"
     ```

3. **Collapsible Sections (`[collapsible: true]`)**: Convert any element or component call into a collapsible section.
   - *Syntax*: Add bracketed attribute `[collapsible: true]` (along with optional `default: "open"` or `default: "closed"`).
   - *Behavior*: The first child of the collapsible container is treated as the clickable toggle header. All subsequent children are wrapped in the collapsible body.
   - *Example*:
     ```amana
     section.card [collapsible: true, default: "open"]:
         div.card-header:
             h3: "Section Title"
         div.card-body:
             p: "Collapsible content here"
     ```

### Control Flow

```amana
if User.current != null:
    p: "Signed in"
else:
    p: "Guest"
```

The condition must typecheck as `bool`.

```amana
for project in projects:
    p: project.name
```

The list expression in current parser syntax is an identifier. It must resolve to a list, usually from `Model.all(...)` or `Model.filter(...)`.

## Expressions

Primary expressions:

- identifiers
- numbers
- strings
- booleans
- `null`
- grouped expressions: `(expr)`
- unary `not expr`
- unary `-expr`

Binary operators:

```text
* /        arithmetic
+ -        arithmetic or string concatenation for +
== != > < >= <=
and or
=          assignment in client/event contexts
```

Member access and calls:

```amana
User.current.email
Project.find(params.id)
env("SESSION_SECRET", "dev")
```

Semantic type rules:

- Arithmetic requires numeric operands unless `+` is used with a string.
- Comparisons require equal types or compatible numeric/null types.
- `and` and `or` require booleans.
- Assignment left side must be an identifier or member access.
- `env(...)` accepts one or two string arguments and compiles to `process.env[...] || fallback`.

Ternary expressions are production syntax and compile as `condition ? a : b` across parser, semantic analysis, JS codegen, optimizer, and runtime evaluation.

## Forms

```amana
form [name, description]:
    connect Project.create
    default owner_id = User.current.id
    ui: card
    submit: "Create project"
    redirect success -> /projects
    field name:
        label: "Project name"
        placeholder: "Apollo"
        required: true
```

Supported settings:

- `connect Model.action`
- `redirect success -> /path`
- `default field = expression`
- `where field = expression`
- `ui: value`
- `submit: value`
- `field name:` with `label`, `placeholder`, `type`, `help`, `required`. Specifying `type: textarea` inside a form field option renders a native HTML `<textarea>` with Class `amana-form-control` and 4 rows (e.g. `field message: type: textarea`). This behaves identically to standalone `FormField(name: "message", type: "textarea")` calls.

Supported actions:

- `create`
- `update`
- `delete`
- `login`
- `register`
- `logout`

Rules:

- The connected model must exist.
- Listed fields must exist, except implicit `id`.
- `login` requires `email` and `password`.
- `login`, `register`, and `logout` must target the configured `auth_model`.
- `where` constraints are allowed only for `update` and `delete`.
- `update` and `delete` form actions must include `id`.
- `default` and `where` expressions using `<auth_model>.current` require a protected view. `User.current` is the compatible spelling when `auth_model: User`.
- Field option blocks must reference a field listed in the form field list.

Runtime behavior:

- Forms POST to `/form-submit/<model>/<action>`.
- CSRF hidden fields are injected automatically.
- Password fields are hashed with Argon2.
- Constraints are evaluated on submit and become authorization filters.

## ResourceGrid And ResourceTable

```amana
ResourceGrid(projects):
    item ProjectCard(project)
    empty:
        p: "No projects"
    loading:
        p: "Loading"
    error:
        p: "Could not load projects"
    filters:
        - status
    sort:
        - name
```

Rules:

- The resource expression must typecheck as a list.
- `item ComponentName(arg)` must reference a declared custom component.
- `empty`, `loading`, and `error` blocks are parsed.
- Current EJS codegen renders the list, `empty`, `loading`, `error`, `filters`, and `sort` blocks into Alpine-powered runtime behavior.
- `inspect-design` warns when resource blocks lack lifecycle handlers.

## Components

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

Parameters:

```amana
component Badge(label: str, tone: str = "neutral"):
```

Rules:

- Parameters may have optional type annotations.
- Defaults make parameters optional.
- Required parameters must be supplied at call sites.
- Type annotations are checked for `str`, `int`, `float`, `bool`, and custom types.
- Component bodies are inlined into view IR.
- Component styles are aggregated into views that use the component.

Slots:

```amana
slot:
slot footer
slot actions optional
```

- `slot:` declares the default slot.
- `slot name` declares a required named slot.
- `slot name optional` declares an optional named slot.
- Required slots must be supplied by matching child elements.

## Built-In Components

The current EJS codegen recognizes 58 built-in components:

```text
Button, Card, FeatureCard, PricingCard, Container, Section, Grid, Stack,
FormField, Navbar, Hero, Alert, Footer, Icon, Modal, Tabs, Badge, Kpi,
Stat, LogoCloud, TestimonialCard, Timeline, TimelineItem, EmptyState,
Split, Cluster, Sidebar, Slides, Center, Cover, Reel, Masonry, Skeleton,
LoadingState, ErrorState, OfflineState, Toast, Banner, DashboardShell,
AuthPage, PricingSection, Breadcrumb, Dropdown, CommandPalette, SearchBar,
FilterBar, Paginator, DataTable, FileUpload, RichEditor, ColorPicker,
HeroSection, SettingsPage, StatsSection, FAQSection, BlogSection,
TestimonialsSection, ContactSection
```

> [!IMPORTANT]
> The standard component library is warning-free and verified against visual E2E regression tests. For any future active bugs or feature gap checklists, see the [Known Issues](language-runtime-trust-plan.md) section of the trust plan.

Attributes are component-specific and mostly optional. See [html-components-forms.md](html-components-forms.md) for the focused component reference.

### Grid Numeric Columns Behavior
- `Grid(columns: "3")` compiles to `--dg-columns:repeat(3, minmax(0, 1fr))`
- `Grid(columns: "1")` compiles to `--dg-columns:minmax(0, 1fr)`
- Responsive mobile column configurations such as `responsive: mobile: columns: 1` nested block compile to `--bp-mobile-columns:minmax(0, 1fr)`.
- No raw numeric columns reach the compiled output (e.g. `--bp-mobile-columns:1` is never output, preventing CSS display issues).

## Alpine/Event Attributes

Raw elements and components can use these event attributes:

```text
click submit change input keydown keyup focus blur mouseenter mouseleave
```

Special attributes:

- `bind`: binds a server value or client state into an input.
- `model`: emits Alpine `x-model`.
- `show`: emits `x-show`.
- `text`: emits `x-text`.
- `init`: emits `x-init`.
- `disabled`, `checked`, `selected`, `readonly`: emitted as Alpine boolean bindings.
- `style`: merged with design-generated inline style variables.

## Style Block

```amana
style:
    .card:
        layout: column
        gap: lg
        surface: glass
        radius: soft
        shadow: floating
```

The style parser accepts complex selector blocks and declaration blocks. Values are normalized and compiled through the CSS DSL.

Selectors can include:
- Class and element names.
- Pseudo-classes and functional pseudo-classes (e.g. `:has()`, `:not()`, `:nth-child()`).
- Combinators (e.g. child combinator `>` and adjacent sibling combinator `+`).
- Safe attribute selectors (e.g. `[data-active="true"]`).

Mathematical Expressions:
- Operators (`+`, `-`, `*`, `/`) inside math functions (`calc()`, `min()`, `max()`, `clamp()`) are parsed and automatically formatted with correct spacing (e.g. `calc(100vh - 2.5rem)`) to comply with CSS syntax standards.

Security:

- Blocked selectors include `body`, `html`, `*`, `script`, `iframe`, `object`, `embed`, `link`, `meta`, `base`.
- Attribute selectors like `[onclick]` are rejected.
- Properties are allowlisted.
- Values containing unsafe script/style patterns are rejected.

## Design Grammar

Design blocks are parsed inside `render:` elements and `canvas:` at view level.

```amana
section.hero:
    compose:
        layout: bento
        gap: lg
    visual:
        surface: glass
        gradient: hero
    brand:
        voice: technical
    responsive:
        mobile:
            columns: 1
```

Allowed design blocks:

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

Most descriptive metadata fields are open-ended but still sanitized and length-limited. Examples: `voice`, `colorway`, `direction`, `motif`, `lighting`, `texture`, `feedback`, `cursor`, `contrast`, `screenreader`.

`compose` layout-specific key restrictions are enforced for these layouts:

- `bento`
- `masonry`
- `split`
- `asymmetric`
- `magazine`
- `sidebar`
- `timeline`
- `dashboard-shell`

## Variants

Global variant:

```amana
variant Card.glass:
    base:
        background: glass
        radius: soft
    hover:
        shadow: floating
    responsive:
        mobile:
            padding: md
```

Component-local variants:

```amana
component CardShell:
    variants:
        compact:
            base:
                padding: sm
```

Variant sections:

- `base`
- `hover`
- `slots`
- `responsive`

Targets must be a known standard component or a declared custom component. Responsive breakpoints are `desktop`, `tablet`, and `mobile`.

Application rules:

- Standard components apply variants through a static `variant` attribute such as `Card(variant: "glass")`.
- Standard components also accept nested design blocks such as `component: variant: glass`.
- Custom components receive stable runtime classes after inlining: `amana-component-<component>`, `amana-variant-<variant>`, and `dg-component-variant-<variant>`.
- Variant CSS supports `base`, `hover`, `slots`, and `responsive` sections. CSS declarations use the same safe token lowering as `style:` blocks.

## Generated Output

`amana build` emits:

```text
app.js
package.json
amana_ir.json
runtime/engine.js
middleware/security.js
middleware/hooks-worker.js
custom/hooks.js
views/<view>.ejs
views/login.ejs when no /login route exists
```

Generated Node package:

- Runtime dependencies: `express`, `express-session`, `sqlite3`, `ejs`, `argon2`, `express-rate-limit`, `helmet`.
- Dev dependency: `nodemon`.
- Scripts: `npm start`, `npm run dev`.

### Current Implementation Boundaries

These boundaries are intentionally documented so the reference does not overpromise:

- `tokens:` blocks reach IR and generated theme CSS as CSS custom properties such as `--color-brand`, `--space-tight`, `--radius-panel`, and `--shadow-card`.
- `permit` rules reach IR and are enforced by the generated Express runtime for REST reads/writes, form create/update/delete, and server-side model fetches. Row-level `where` expressions, read field masking, and write field allowlists are runtime-enforced. Integration coverage exercises separate sessions against REST and forms with a non-`User` `auth_model`.
- `Chart(data, type, x, y)` is parsed and rendered through the existing Chart.js runtime path. The current parser form accepts identifier arguments.
- Ternary expressions using `cond ? then : else` are parsed and emitted through semantic/codegen/runtime expression paths.
- `ResourceGrid` and `ResourceTable` render server-fetched rows with Alpine lifecycle state, loading/error/empty blocks, client-side filters, and client-side sort over the loaded result set.
- Client state `persist: memory/local/session/cookie` is represented as an enum and non-memory modes hydrate/watch browser storage in generated EJS.
- Configurable `auth_model` is used for `<auth_model>.current` in semantic checks, EJS expression output, route guards, form defaults/constraints, REST authorization, and default login table selection.
- Component variants are parsed, validated, preserved in IR, and emitted as target-specific CSS in generated EJS.
- Phase 2 Visual styles (Neo-Bento/Glass surfaces, Timeline RTL markers, PricingCard featured variants, Button active/focus states, Card hover transition lifts, Navbar glass variant, and KPI layout styles) are fully integrated as CSS presets and standard component visual classes. The preview engine matches the build engine's layout and style rules.
- **Interactive DSL Layout Primitives** (`Tabs`, `Accordion`, and collapsible elements) are fully implemented, validated semantically, and emitted as Alpine.js active state toggles in generated EJS views.
- **State Scope Wrapper**, **Mobile DashboardShell Layout Contract**, and **Mobile Density Contract** are fully enforced at the code-generation and runtime-CSS levels, automatically handling viewport height boundaries and mobile scrolling/density behaviors.
