# Amana

<div align="center">

**A declarative full-stack DSL. Write `.amana`, get a production-ready Node.js + Express + EJS + SQLite app.**

[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](https://www.rust-lang.org)
[![Node](https://img.shields.io/badge/Node-18%2B-green.svg)](https://nodejs.org)
[![Tests](https://img.shields.io/badge/tests-64%20passing-brightgreen.svg)](#verification)

**النسخة الحالية / Current release: `v3.0`**

</div>

---

Amana is a Rust compiler for a declarative `.amana` DSL. You describe your data models, routes, and views in one file (or a small graph of files), and the compiler generates a complete, runnable web application — with auth, CSRF protection, password hashing, REST API, and a responsive RTL-aware UI built in.

```amana
app NotesApp:
    title: "Notes"
    auth_model: User
    capabilities:
        - auth

model User:
    email: email unique required
    password: password required min 8

model Note:
    title: str required max 120
    body: str
    owner_id: int foreign_key User(id) on_delete CASCADE

route /notes -> view Notes

view Notes:
    protected:
        allow: User.current != null
        deny: -> /login
    server:
        fetch notes = Note.filter(owner_id: User.current.id, limit: 50)
    render:
        div.page:
            h1: "My Notes"
            for note in notes:
                article.card:
                    h3: note.title
                    p: note.body
```

> 📖 **New to Amana?** Start with **[doc/getting-started.md](doc/getting-started.md)** — a bilingual onboarding guide that walks you from install to a working auth-enabled app.
>
> 🆕 **What's new in v3.0?** See **[CHANGELOG.md](CHANGELOG.md)** and the **[What's New in v3.0](#whats-new-in-v30)** section below.

---

## Table of Contents

- [What's New in v3.0](#whats-new-in-v30)
- [Quick Start](#quick-start)
- [The Mental Model](#the-mental-model)
- [Compiler Surface](#compiler-surface)
- [CLI](#cli)
- [Security](#security)
- [Verification](#verification)
- [Documentation](#documentation)
- [Project Layout](#project-layout)
- [Known Issues](#known-issues)
- [License](#license)

---

## What's New in v3.0

v3.0 is a **production-ready layout components and design tokens** release. It implements Layer 0 (Design Tokens & Theme presets), Layer 1 (Accessibility & Variants), Layer 2 (Core Shells & Navigation), and Layer 3 (Advanced Data components), bringing the entire roadmap to 100% completion.

### ✅ Problems Solved

| Problem | Resolution |
| --- | --- |
| **Arabic Canvas Text Disjointed & Reversed**: HTML5 canvas did not support Arabic ligatures nor RTL text direction. | **Custom Arabic Canvas Reshaper**: Injected `arabic-reshaper` and a JS segmenter to shape characters, reverse Arabic words, and order segments correctly on LTR canvas. |
| **Blank second Chart ("أداء المبيعات")**: Identical data sources on a single page view generated colliding canvas element IDs. | **Unique Canvas ID Generation**: Integrated thread-local `CHART_COUNTER` (compile-time) and random suffixing (runtime) to prevent collisions. |
| **Recessed Black Hole Modal Inputs**: Luxury theme modal inputs had flat, dark grey backgrounds with poor visibility and interactive feedback. | **Premium Luxury Styling Overrides**: Applied elevated backgrounds, gold/amber borders (`rgba(234,179,8,0.25)`), golden hover/focus glow effects, and structured spacing. |
| **Compiler Compilation Warnings**: Unused design tokens and static templates generated build warnings. | **Localized Suppressions & Cleanup**: Applied targeted inner allow attributes locally inside submodules and removed unused HashMap imports. |

### ➕ Added

- **Design Tokens & Theme Presets** (`luxury`, `stripe`, `linear`) supporting responsive switching of CSS variables.
- **Layout Components** (`Center`, `Cover`, `Reel`, `Masonry`).
- **State System Components** (`Skeleton` shimmer loading, `LoadingState`, `ErrorState`, `OfflineState`).
- **Feedback System** (`Toast`, `Banner`).
- **Core Page Shells** (`DashboardShell`, `AuthPage`, `PricingSection`).
- **Navigation Components** (`Breadcrumb`, `Dropdown`, `CommandPalette` focus traps).
- **Advanced Interaction** (`FileUpload`, `RichEditor`, `ColorPicker`).
- **Content Sections** (`HeroSection`, `SettingsPage`, `StatsSection`, `FAQSection`, `BlogSection`, `TestimonialsSection`, `ContactSection`).
- **DX Tools** (`amana generate` boilerplates, `amana dev/watch` live-reloader, `amana check`).


---

## Quick Start

### Prerequisites

- **Rust** (stable) — to build the compiler. <https://rustup.rs>
- **Node.js 18+ and npm** — to run the generated app.
- **Python 3** *(optional)* — only to regenerate the language inventory.

### Build & Run

```powershell
# 1. Clone and build the compiler
git clone https://github.com/akleeko2/amana-dsl
cd amana-dsl
cargo build --release

# 2. Check your first app
amana check examples/landing.amana --json

# 3. Build the generated web app
amana build examples/landing.amana dist
cd dist
npm install
npm run dev
```

Open <http://localhost:3000>.

### Or use the live dev server

```powershell
amana dev examples/landing.amana dist
```

It watches the import graph, rebuilds on change, and validates `runtime/engine.js` with `node --check` after each rebuild.

---

## The Mental Model

```
amana source (.amana)
        │
        ▼
  ┌───────────┐
  │ Amana CLI │   amana build app.amana dist
  └───────────┘
        │
        ▼
Node/Express/EJS/SQLite app
        │
        ▼
  http://localhost:3000
```

Three building blocks cover ~90% of any Amana app:

| Block | Purpose |
| --- | --- |
| `model` | A database table + its fields and permissions. |
| `route` | A URL path mapped to a view. |
| `view` | An HTML page: protected access, server fetches, client state, render tree, styles. |

---

## Compiler Surface

- **Syntax & Parsing** — Indentation-sensitive `.amana` parser with multi-file imports. Parses `app`, `theme`, `model`, `seed`, `route`, `view`, `component`, `variant`, `tokens`, plus interactive layout primitives (`Tabs`, `Accordion`, `[collapsible: true]`).
- **Validation** — Full semantic checks for models, views, forms, seeds, themes, design grammar, type rules, and standard-library capability gating.
- **Codegen** — Express/EJS/SQLite project generation; `.amana-state-scope` wrapper to isolate dynamic states and guarantee layout heights.
- **Visual Presets** — Neo-Bento/Glass surfaces, Navbar glass variant, Timeline RTL markers, PricingCard featured variants, Button active/focus states, Card hover lifts, KPI layouts. Grid numeric/responsive columns compile to `repeat`/`minmax`. Mobile DashboardShell and Mobile Density contracts (grid stacking, compacted spacing, auto-scrolling secondary lists).
- **Security & Runtime** — CSRF middleware, rate limiting, Argon2 password hashing, EJS/Alpine dynamic `ResourceGrid`/`ResourceTable` states (loading, error, empty, client-side filtering/sorting), REST API behind `api.rest`, parameterized queries.
- **Developer Tooling** — Formatter, JSON diagnostics, IR snapshots (write/verify), design inspection with score, semantic LSP (completion, hover, go-to-definition), and live-rebuild dev workflow.

See [doc/language.md](doc/language.md) for the complete reference and [doc/language-inventory.generated.md](doc/language-inventory.generated.md) for the source-derived inventory.

---

## CLI

```powershell
amana check    <source.amana> [--json] [--snapshot-ir [path]] [--verify-ir-snapshot [path]]
amana build    <source.amana> [output-dir] [--json] [--snapshot-ir [path]]
amana dev      <source.amana> [output-dir] [--no-install] [--no-watch]
amana fmt      <source.amana> [--check] [--json] [--all]
amana inspect-design <source.amana> [--json]      (alias: design-report)
amana lsp                                       (alias: language-server)
```

Full details: [doc/cli-dx.md](doc/cli-dx.md).

---

## Security

The generated runtime is secure by default:

- **Sessions** — `express-session`; production requires a strong `SESSION_SECRET` (≥32 chars), `secure` cookies, `httpOnly`, `sameSite: lax`.
- **Passwords** — Argon2 hashing on seed insert, REST create/update, and form create/update/register.
- **CSRF** — per-session token injected into every generated form; POST requests rejected on mismatch.
- **Production hardening** — `trust proxy`, HTTPS redirect, Helmet, rate limiting.
- **Authorization** — `permit` rules enforce role/action/resource, row-level `where`, read field masking, and write field allowlists across REST, forms, and server fetches. A model with any `permit` rule is default-deny.
- **Static rejection** — unsafe raw HTML tags, unsafe CSS selectors/properties/values, missing standard-library capabilities, server-only calls inside `render`, and current-user writes outside protected views are all rejected at compile time.

Full details: [doc/node-runtime-security.md](doc/node-runtime-security.md).

---

## Verification

```powershell
cargo test                                      # 67 tests, all passing
python scripts/language_inventory.py --check    # inventory up to date
cargo run -- check examples/landing.amana --json
```

Every public language feature must be represented in parser → AST → semantic → IR → codegen → runtime → docs → tests before it is documented as production-safe.

---

## Documentation

The authoritative documentation lives in [doc/](doc/). It is maintained from compiler implementation files, **not** from old examples or tests.

| You are… | Start here |
| --- | --- |
| Building an Amana app | [doc/getting-started.md](doc/getting-started.md) |
| Learning the full language | [doc/language.md](doc/language.md) |
| Checking components & forms | [doc/html-components-forms.md](doc/html-components-forms.md) |
| Styling, theme, RTL | [doc/css-theme-rtl.md](doc/css-theme-rtl.md) |
| Security & runtime | [doc/node-runtime-security.md](doc/node-runtime-security.md) |
| Multi-file & LSP | [doc/multi-file-lsp.md](doc/multi-file-lsp.md) |
| Avoiding design mistakes | [doc/design-anti-patterns.md](doc/design-anti-patterns.md) |
| Knowing what works vs. known bugs | [doc/language-runtime-trust-plan.md](doc/language-runtime-trust-plan.md) → Known Issues |
| Release history | [CHANGELOG.md](CHANGELOG.md) |
| Contributing to the compiler | [CONTRIBUTING.md](CONTRIBUTING.md), [doc/roadmap.md](doc/roadmap.md) |

---

## Project Layout

```
amana-dsl/
├── src/                      # The Rust compiler
│   ├── lexer/  parser/  ast/ # Front-end
│   ├── semantic/             # Validation, types, IR
│   └── codegen/              # Express/EJS/SQLite generation
│       └── express/static_files/engine.rs   # runtime/engine.js generator
├── doc/                      # Authoritative documentation
├── examples/                 # Verified minimal examples
├── apps/                     # Larger multi-file reference applications
├── scripts/                  # Inventory & search tooling
├── amana-live-compiler/      # Live in-browser compiler playground
├── CHANGELOG.md              # Release history
└── README.md                 # This file
```

---

## Known Issues

All major layout, visual, and responsiveness bugs (including tablet breakpoints, laptop content overflow, textareas inside modals, and RTL Arabic canvas rendering) have been fully resolved in v3.0.0. The compiler is warning-free and passes 100% of the regression test suites.

Any new issues or feature gaps discovered are tracked dynamically in **[doc/language-runtime-trust-plan.md → Known Issues](doc/language-runtime-trust-plan.md)**.

---

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
