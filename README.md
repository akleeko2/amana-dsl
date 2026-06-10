# 🌌 Amana

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](#)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rustc-1.75%2B-orange.svg)](#)
[![DX Rating](https://img.shields.io/badge/DX-Premium-purple.svg)](#)

Amana is a declarative, zero-dependency, full-stack **Design-to-Code DSL** and compiler. It compiles a single `.amana` file into a production-hardened Node.js/Express web application with automated SQLite database migrations, secure server-side forms, and premium CSS layouts.

With its advanced **v2 Design Engine**, Amana bridges the gap between design tokens and real-world database actions, compiling high-end layouts (such as Bento grids, asymmetrical showcases, editorial columns, and technical consoles) with zero configuration.

---

## 🚀 Key Architectural Pillars

*   **⚡ Compile to Node/Express & EJS:** The compiler produces a highly structured Express server using optimized database handlers, security middleware, and lightweight EJS layouts.
*   **🎨 Declarative Design Grammar v2:** Design blocks like `compose:`, `visual:`, and `responsive:` compile into responsive flex/grid layouts with automatic HSL themes, mesh gradients, spotlights, and custom border/surface tokens.
*   **🛡️ 4-Layer Scoped CSS Sanitizer:** A static compiler-level security shield that sanitizes selector hierarchies, whitelists property types, blocks javascript protocols (`javascript:`), and prevents styling-based data leakage.
*   **📊 Database & Forms Integration:** Declaring a `model` generates SQLite migrations, seeds, and server-side validated `form` submission pipelines automatically.
*   **🔍 DX & AI-Ready Toolchain:** Built-in code formatter (`amana fmt`), language server (LSP), layout diversity inspections, and JSON diagnostic outputs for automated AI repair loops.

---

## 🏗️ Visual Layout Gallery

Amana includes 5 pre-built, premium-designed examples in the `examples/` directory demonstrating different design directions and layouts:

| Example | Visual Aesthetic | Layout Primitives | Database Models |
| :--- | :--- | :--- | :--- |
| [saas_aura](examples/01_saas_aura.amana) | **Dark Neon Space Glass**<br>Mesh gradients, glow highlights | `split` (Hero), `bento` (Live metrics grid), `asymmetric` | `Metric`, `Feedback` |
| [maison_luxe](examples/02_maison_luxe.amana) | **Editorial Luxe Day Mode**<br>Gold & Emerald accent, Serif fonts | `split-diagonal` (Diagonal cuts), `magazine` (12-col spans), `editorial` | `CollectionItem`, `Booking` |
| [vortex_console](examples/03_vortex_console.amana) | **Obsidian DevOps Tech**<br>Blueprint grid, lime glow | `command-center` (3-column terminal), `masonry` (Diagnostics logs) | `DeployLog`, `AccessRequest` |
| [nova_creative](examples/04_nova_creative.amana) | **Deep Orchid Motion**<br>Purple blob visual shapes, smooth animations | `asymmetric` (Stagger-up hero), `showcase-rail` (Horizontal portfolio) | `ProjectBrief` |
| [cura_wellness](examples/05_cura_wellness.amana) | **Modern Day Healing**<br>Mint & Teal rounded surfaces | `split` (Hero stats), `sidebar` (Sticky Specialties), `bento` | `Appointment` |

---

## ⚡ Quick Start

### 1. Build and Test the compiler
Clone the repository and run the cargo test suite:
```powershell
cargo test
```

### 2. Verify an Example
Run semantic checks and build one of the premium examples:
```powershell
# Run compiler analysis check
cargo run -- check examples/01_saas_aura.amana --json

# Build the complete full-stack web application
cargo run -- build examples/01_saas_aura.amana .amana_saas_dist
```

### 3. Launch the Web Application
Inside the generated directory, install dependencies and launch the server:
```powershell
cd .amana_saas_dist
npm install
npm run dev
```
Open [http://localhost:3000](http://localhost:3000) to view your live, interactive Amana application!

---

## 🛠️ The Amana CLI Toolchain

The Amana binary supports several commands to maximize developer experience:

```bash
# Check syntax and semantics (prints JSON diagnostics)
amana check <file.amana> [--json]

# Format Amana source files recursively
amana fmt <file.amana> [--all] [--check]

# Analyze layout diversity and design scores
amana inspect-design <file.amana>

# Start developer watch mode with live hot reloading
amana dev <file.amana> [output-dir]

# Run the Amana Language Server Protocol (LSP)
amana lsp
```

---

## 📂 Documentation

Deep dive into Amana's specifications:
*   📖 **[Language Reference](doc/language.md):** Syntax guidelines, custom components, and database mapping.
*   🛡️ **[Security Architecture](doc/node-runtime-security.md):** 4-layer scoped CSS sanitizer and SQL protection.
*   📐 **[CSS Layout & Theme System](doc/css-theme-rtl.md):** RTL support, visual tokens, and grids.
*   💻 **[CLI Developer Experience](doc/cli-dx.md):** Code formatting, inspections, and LSP.
*   🎨 **[Examples Gallery Info](doc/examples-gallery.md):** Complete specifications of all pre-built layouts.

---

## 📜 License

This project is licensed under the **GNU Affero General Public License v3.0** - see the [LICENSE](LICENSE) file for details. If you use Amana over the network (e.g. as a cloud SaaS compiler), you must make your modified source code available.
