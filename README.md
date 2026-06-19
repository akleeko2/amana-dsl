# Amana

<div align="center">

**A declarative full-stack DSL. Write `.amana`, get a production-ready Node.js + Express + EJS + SQLite app.**

[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](https://www.rust-lang.org)
[![Node](https://img.shields.io/badge/Node-18%2B-green.svg)](https://nodejs.org)
[![Tests](https://img.shields.io/badge/tests-67%20passing-brightgreen.svg)](#verification)

**النسخة الحالية / Current release: `v2.5`**

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
> 🆕 **What's new in v2.5?** See **[CHANGELOG.md](CHANGELOG.md)** and the **[What's New in v2.5](#whats-new-in-v25)** section below.

---

## Table of Contents

- [What's New in v2.5](#whats-new-in-v25)
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

## What's New in v2.5

v2.5 is a **repository integrity, documentation accuracy, and developer onboarding** release. It does not change the language surface or generated runtime — instead it makes the project trustworthy: the compiler can now be built from a fresh clone, the docs no longer overpromise, and there is a clear path from zero to a running app.

### ✅ Problems Solved

| Problem | Resolution |
| --- | --- |
| **The compiler could not be built from a fresh clone.** Twenty source files (~7,964 lines) and the runtime engine generator (`engine.rs`, 3,628 lines) were never committed to git. | All 21 missing compiler source files are now tracked. The build is reproducible from `git clone`. |
| **Docs claimed features as "Implemented" while an internal audit listed 5 active bugs.** This is exactly the false-advertising pattern the trust plan forbade. | A canonical **Known Issues** table now lives in `doc/language-runtime-trust-plan.md`. `roadmap.md` and `language.md` point to it; a feature with an active bug can no longer be described as fully `implemented`. |
| **68 generated build artifacts** (`apps/*/dist/`, including `.log`, `.pid`, `.db` files) were committed to the repo by mistake. | All 68 are untracked (kept on disk). `.gitignore` expanded to cover `dist/`, `*_dist/`, `examples_dist_test/`, `scratch/`, `المستقبل/`, `task.md`, `.amana_live_dist/`, etc. |
| **Duplicate design docs** (`design-bugs-prevention.md` in English and `design-mistakes-avoidance.md` in Arabic, ~70% overlap) drifted out of sync. | Merged into one bilingual `doc/design-anti-patterns.md`, with every rule tagged `SOLVED` / `ACTIVE RISK` / `ENFORCED` so you know what is history and what is still a risk. |
| **`examples-gallery.md` referenced deleted examples** (`01_saas_aura` … `09_multi_file_portal`). | Rewritten to reflect the actual `examples/` directory (`landing`, `royal_deck`, `test_ternary`). The old numbered examples are explicitly marked non-authoritative. |
| **No end-user onboarding path.** Every doc was written for compiler maintainers, not app authors. | New `doc/getting-started.md` — bilingual, install → first app → auth + data, with troubleshooting. |
| **No release history.** IR version and feature tags drifted with no record. | New `CHANGELOG.md` reconciling commit tags (`v2.x.y`) with language changes, plus a `[Unreleased]` section documenting this very cleanup. |
| **Ambiguous `n/a` cells** in the coverage matrix looked like "not supported". | Clarified: `n/a` = *Not Applicable by design*, not *Not Available*. |

### ➕ Added

- **`doc/getting-started.md`** — bilingual end-user onboarding guide.
- **`doc/design-anti-patterns.md`** — unified bilingual design anti-patterns reference.
- **`CHANGELOG.md`** — full release history with maintenance rules.
- **Known Issues table** in the trust plan — the single source of truth for active bugs.
- **Documentation Conventions** section in `doc/README.md` (status words are binding; two languages, one truth).
- `.gitignore` coverage for `scratch/`, `المستقبل/`, `task.md`, `*_dist/`, `.amana_live_dist/`, and planning artifacts.

### 🔄 Changed

- `roadmap.md` "Implemented" scope narrowed to core language/runtime; visual component gaps moved to Known Issues.
- `language.md` warns that not every listed component is production-safe yet.
- `cli-dx.md` absorbs the former `live-preview.md` (Dev Server section).
- `doc/README.md` splits reading order into *app authors* vs *compiler contributors*.

### ❌ Removed

- `doc/design-bugs-prevention.md`, `doc/design-mistakes-avoidance.md` (merged into `design-anti-patterns.md`).
- `doc/live-preview.md` (merged into `cli-dx.md`).
- 68 committed build artifacts from `apps/*/dist/`.

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

Amana is honest about its gaps. The canonical, maintained list of active bugs lives in **[doc/language-runtime-trust-plan.md → Known Issues](doc/language-runtime-trust-plan.md)**. Highlights:

- **`Modal`** (high severity) compiles to a raw `div` — no overlay, backdrop filter, scroll lock, or focus trap.
- **`Grid`** cards stretch to the tallest column on desktop (`align-items: stretch` default).
- **Tablet breakpoints** are missing; three-column layouts squeeze between 1024px and 720px.
- **Laptop overflow** — fixed sidebars cause horizontal scrollbars at 1280px.
- **`FormField textarea`** lacks height/overflow constraints inside modals.

Each entry has a severity, impact, and workaround. If you hit one of these, it is a known issue — not your code.

---

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
