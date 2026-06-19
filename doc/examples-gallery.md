# Examples Policy And Current Patterns

Verified examples are reference artifacts, not the implementation source of truth. Use this file as a pattern guide; the implementation reference remains [language.md](language.md) and [language-inventory.generated.md](language-inventory.generated.md).

> **Note on history:** Earlier versions of this repository shipped numbered examples (`01_saas_aura.amana`, `02_maison_luxe.amana`, ..., `09_multi_file_portal/`). Those were removed because they drifted from the current language surface and were not verified against the compiler. They are **not** authoritative; do not treat them as reference even if found in old commits or external copies. The only verified examples today are listed below.

## Verified Examples Gallery

The following examples exist in [`examples/`](../examples/) and are the only canonical examples:

| # | File | What it demonstrates | Build command |
| --- | --- | --- | --- |
| 1 | [examples/landing.amana](../examples/landing.amana) | A multi-column landing page: grids, layouts, forms, custom styled grids. | `cargo run -- build examples/landing.amana landing_dist` |
| 2 | [examples/royal_deck.amana](../examples/royal_deck.amana) | A daytime luxury console: standard elements with native HTML fallbacks, premium aesthetics, animations, responsive column rules, interactive Alpine bindings, Unsplash media. | `cargo run -- build examples/royal_deck.amana royal_deck_dist` |
| 3 | [examples/test_ternary.amana](../examples/test_ternary.amana) | A minimal test fixture exercising the ternary expression syntax (`cond ? a : b`). Not a full app; used as a language-feature smoke test. | `cargo run -- check examples/test_ternary.amana --json` |

> The multi-file `apps/` directory (e.g. `apps/02-customer-care-hub/`, `apps/03-ai-chat-workspace/`) contains larger integrated applications structured per [AMANA_DEVELOPMENT_STANDARDS.md](../AMANA_DEVELOPMENT_STANDARDS.md). These are real apps, not minimal examples — see their per-app `README.md` files.

## Example Acceptance Checklist

Before adding or restoring an example to this project:

1. Run `cargo run -- check <example>.amana --json`.
2. Run `cargo run -- build <example>.amana <dist> --json`.
3. Run `node --check <dist>/runtime/engine.js`.
4. Run `cargo run -- inspect-design <example>.amana --json`.
5. Add a row to the table above.
6. Do not reference the example in docs until all commands pass.

## Pattern Reference (extracts)

These short patterns are illustrative. Always validate against the full [language.md](language.md).

### Minimal App

```amana
app MinimalApp:
    title: "Minimal"
    db_path: "minimal.db"

model User:
    email: email unique
    password: password

route / -> view Home

view Home:
    render:
        div.page:
            h1: "Home"
```

### Auth-Protected Data View

```amana
app ProjectApp:
    title: "Projects"
    auth_model: User
    capabilities:
        - auth

model User:
    email: email unique required
    password: password required min 8

model Project:
    name: str required
    owner_id: int foreign_key User(id) on_delete CASCADE

route /projects -> view Projects

view Projects:
    protected:
        allow: User.current != null
        deny: -> /login
        unauthenticated: -> /login
    server:
        fetch projects = Project.filter(owner_id: User.current.id, limit: 50)
    render:
        div.page:
            for project in projects:
                p: project.name
```

### Form With Ownership Default

```amana
form [name]:
    connect Project.create
    default owner_id = User.current.id
    submit: "Create"
    redirect success -> /projects
    field name:
        label: "Project name"
        required: true
```

This form must be inside a protected view because it uses `User.current`.

### Design-Driven Section

```amana
section.hero:
    compose:
        layout: bento
        columns: 3
        gap: lg
    visual:
        surface: glass
        gradient: hero
    responsive:
        mobile.columns: 1
    Hero(title: "Launch", subtitle: "Build with Amana"):
        Button(label: "Start", href: "/projects")
```

### Resource Grid Pattern

```amana
ResourceGrid(projects):
    item ProjectCard(project)
    empty:
        EmptyState(title: "No projects")
    loading:
        p: "Loading"
    error:
        Alert(tone: "warning", message: "Could not load projects")
```

Current EJS output renders the item loop plus lifecycle blocks at runtime.
