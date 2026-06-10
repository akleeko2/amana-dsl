<div align="center">

<br/>

```
 █████╗ ███╗   ███╗ █████╗ ███╗   ██╗ █████╗
██╔══██╗████╗ ████║██╔══██╗████╗  ██║██╔══██╗
███████║██╔████╔██║███████║██╔██╗ ██║███████║
██╔══██║██║╚██╔╝██║██╔══██║██║╚██╗██║██╔══██║
██║  ██║██║ ╚═╝ ██║██║  ██║██║ ╚████║██║  ██║
╚═╝  ╚═╝╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝
```

### The Declarative DSL that compiles design intent into full-stack reality.

<br/>

[![Build](https://img.shields.io/badge/build-passing-brightgreen?style=flat-square&logo=rust)](https://github.com/akleeko2/amana-dsl)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blueviolet?style=for-the-badge)](LICENSE)
[![Rust 1.75+](https://img.shields.io/badge/rustc-1.75%2B-orange?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![Node Runtime](https://img.shields.io/badge/runtime-Node.js%2FExpress-green?style=for-the-badge&logo=node.js)](#)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-ff69b4?style=for-the-badge)](CONTRIBUTING.md)

<br/>

> **Write once in `.amana`. Get a production-ready Express app, a live SQLite database, secured forms, and beautiful responsive layouts — all compiled automatically.**

<br/>

[✨ See Examples](#-visual-showcase) · [🚀 Quick Start](#-quick-start) · [📖 Docs](#-documentation) · [🤝 Contributing](#contributing)

</div>

---

## 🌟 What is Amana?

Amana is an **opinionated, full-stack Design-to-Code compiler** written in Rust. You write a single `.amana` file — a human-readable declarative specification — and the compiler generates:

| Output | Technology |
|---|---|
| 🌐 HTTP server with routing | Express.js |
| 🗄️ Database schema + migrations + seeds | SQLite (better-sqlite3) |
| 🔒 Server-side validated form pipelines | Express middleware |
| 🎨 Premium responsive layouts + themes | Custom CSS (Grid / Flex) |
| 🖼️ Templated views | EJS |
| 🔍 JSON diagnostic output | Compiler error reports |

No boilerplate. No configuration hell. No placeholder data.

---

## ✨ Visual Showcase

Amana ships with **5 premium examples**, each representing a different design direction. All are compiled from a single `.amana` source file.

<br/>

<table>
<tr>
<td align="center" width="20%">
<b>01 · SaaS Aura</b><br/>
<sub>Dark Neon Glass · Bento Metrics</sub>
</td>
<td align="center" width="20%">
<b>02 · Maison Luxe</b><br/>
<sub>Editorial Gold · Magazine Grid</sub>
</td>
<td align="center" width="20%">
<b>03 · Vortex Console</b><br/>
<sub>Obsidian DevOps · Terminal UI</sub>
</td>
<td align="center" width="20%">
<b>04 · Nova Creative</b><br/>
<sub>Deep Orchid · Blob Animations</sub>
</td>
<td align="center" width="20%">
<b>05 · Cura Wellness</b><br/>
<sub>Mint Teal · Healing Layout</sub>
</td>
</tr>
<tr>
<td align="center">

```
layout: split
layout: bento
layout: asymmetric
```
</td>
<td align="center">

```
layout: split-diagonal
layout: magazine
layout: editorial
```
</td>
<td align="center">

```
layout: command-center
layout: masonry
```
</td>
<td align="center">

```
layout: asymmetric
layout: showcase-rail
```
</td>
<td align="center">

```
layout: split
layout: sidebar
layout: bento
```
</td>
</tr>
</table>

▶ See [`examples/`](examples/) and [`doc/examples-gallery.md`](doc/examples-gallery.md) for full specs.

---

## 🔥 The Amana Language at a Glance

A complete, full-stack application in fewer lines than a typical `index.html`:

```amana
app Analytics:
    title: "Analytics Dashboard"
    db_path: "analytics.db"
    capabilities:
        - auth
        - api.rest

theme:
    mode: dark
    primary: "#6366f1"
    accent: "#06b6d4"
    surface: glass
    radius: soft

model Metric:
    kpi_name: str unique required
    value:     str required
    trend:     str default "+0.0%"

seed Metric:
    row: { kpi_name: "Active Users", value: "12,482", trend: "+8.3%" }
    row: { kpi_name: "Uptime",       value: "99.98%", trend: "+0.1%" }

route / -> view Dashboard

view Dashboard:
    server:
        fetch metrics = Metric.all()
    render:
        div.page:
            Navbar(brand: "Analytics", sticky: true)
            section.hero:
                compose:
                    layout: bento
                    rhythm: steady
                for m in metrics:
                    Kpi(label: m.kpi_name, value: m.value, trend: m.trend)
            Footer()
    style:
        .page:
            background: canvas
            color: text
            min-height: screen
```

**That's it.** `amana build app.amana ./dist` → full Express server, migrations, live database, premium UI. ✅

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    .amana  Source File                           │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                    ┌───────▼────────┐
                    │   Lexer (Rust) │  ← Tokenization
                    └───────┬────────┘
                            │
                    ┌───────▼────────┐
                    │  Parser (Rust) │  ← AST Construction
                    └───────┬────────┘
                            │
                   ┌────────▼─────────┐
                   │ Semantic Analyzer │  ← Type checking, layout
                   │     (Rust)        │    validation, scope analysis
                   └────────┬─────────┘
                            │
                   ┌────────▼─────────┐
                   │  Code Generator  │  ← Express routes, EJS views,
                   │     (Rust)       │    CSS themes, SQL migrations
                   └────────┬─────────┘
                            │
          ┌─────────────────┼──────────────────┐
          │                 │                  │
   ┌──────▼──────┐  ┌───────▼──────┐  ┌───────▼──────┐
   │ server.js   │  │  views/*.ejs │  │ schema.sql   │
   │ (Express)   │  │  (EJS + CSS) │  │ (SQLite)     │
   └─────────────┘  └──────────────┘  └──────────────┘
```

### The 4-Layer CSS Security Pipeline

Amana's CSS sanitizer enforces strict security at compile time — not runtime:

```
Layer 1: Selector Safety  →  Blocks body, html, *, [onclick], etc.
Layer 2: Property Allowlist →  Only safe layout/paint properties pass
Layer 3: Value Sanitizer  →  Strips javascript:, expression(), url(data:...)
Layer 4: CSS Layer Scope  →  @layer components, variants, overrides
```

---

## 🚀 Quick Start

### Prerequisites

- [Rust 1.75+](https://rustup.rs/) (for the compiler)
- [Node.js 18+](https://nodejs.org/) (for the generated runtime)

### 1. Clone & Build

```bash
git clone https://github.com/akleeko2/amana-dsl.git
cd amana-dsl
cargo build --release
```

### 2. Run the Test Suite

```bash
cargo test
```

All tests must pass on a clean clone. ✅

### 3. Check a Source File

```bash
# Validate syntax and semantics
cargo run -- check examples/01_saas_aura.amana

# Get structured JSON diagnostics (AI-ready)
cargo run -- check examples/01_saas_aura.amana --json
```

### 4. Compile to a Full-Stack App

```bash
# Build the SaaS Aura example
cargo run -- build examples/01_saas_aura.amana .amana_saas_dist

# Enter generated app and start
cd .amana_saas_dist
npm install
npm run dev
```

Open [http://localhost:3000](http://localhost:3000) 🎉

---

## 🛠️ CLI Reference

```bash
# Compile a source file to a target directory
amana build  <file.amana> [output-dir]

# Check syntax, semantics, and design validation
amana check  <file.amana> [--json]

# Format source files (formatter)
amana fmt    <file.amana> [--all] [--check]

# Analyze layout diversity and design scores
amana inspect-design <file.amana>

# Watch mode: live rebuild on change
amana dev    <file.amana> [output-dir]

# Start Amana Language Server (LSP / IDE integration)
amana lsp
```

---

## 📂 Documentation

| Document | Description |
|---|---|
| 📖 [Language Reference](doc/language.md) | Complete syntax: `app`, `theme`, `model`, `view`, `route`, `component`, `slot`, `variant` |
| 🏗️ [CSS Layout & Theme System](doc/css-theme-rtl.md) | Design tokens, layout engines, RTL, gradients |
| 🛡️ [Security Architecture](doc/node-runtime-security.md) | 4-layer CSS sanitizer, SQL injection protection |
| 💻 [CLI & DX Guide](doc/cli-dx.md) | Formatter, LSP, inspector, diagnostics |
| 🎨 [Examples Gallery](doc/examples-gallery.md) | All 5 premium examples explained |
| 🗺️ [Roadmap](doc/roadmap.md) | What's coming next |

---

## 🗺️ Roadmap

| Status | Feature |
|---|---|
| ✅ | Full-stack compiler (Lexer → Parser → Semantic → Codegen) |
| ✅ | SQLite model → migration → seed pipeline |
| ✅ | 8 premium layout engines (bento, masonry, split, asymmetric, magazine…) |
| ✅ | 4-layer CSS security sanitizer |
| ✅ | Custom components, named slots, variant registry |
| ✅ | JSON diagnostic output + `check` command |
| ✅ | `fmt` formatter + `inspect-design` |
| ✅ | Language Server Protocol (LSP) skeleton |
| 🔄 | VS Code extension with syntax highlighting |
| 🔄 | `amana cloud` deploy (one-command Fly.io / Railway deploy) |
| 🔄 | PostgreSQL backend (in addition to SQLite) |
| 🔄 | Auth scaffolding (`capabilities: auth` → full login/signup) |
| 🔮 | WASM compiler target (run Amana in the browser) |
| 🔮 | Multi-page import graph (`import "./models/user.amana"`) |
| 🔮 | `amana ui` — visual drag-and-drop `.amana` editor |

---

## 🤝 Contributing

We love contributions! Please read [CONTRIBUTING.md](CONTRIBUTING.md) to get started.

**Quick guide:**
1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes and add tests
4. Run `cargo test` — all tests must pass
5. Open a Pull Request

Check out [Good First Issues](https://github.com/akleeko2/amana-dsl/labels/good%20first%20issue) if you're new!

---

## 📜 License

**GNU Affero General Public License v3.0** — see [LICENSE](LICENSE).

> If you run Amana as a network service (e.g., a SaaS compiler), you **must** make any modifications publicly available under the same license. This ensures Amana remains open for everyone.

---

<div align="center">

Made with ❤️ and a lot of Rust.<br/>
<sub>If Amana saved you hours, consider starring ⭐ the repo — it helps more than you think.</sub>

</div>
