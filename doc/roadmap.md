# Amana DSL — Roadmap (v2.1)

This document tracks the public roadmap for the Amana DSL platform. It is updated regularly as features are shipped, planned, or in active development.

---

## ✅ Shipped (v2.1 — Current)

| Feature | Status | Description |
|---|---|---|
| **Full-Stack compiler pipeline** | ✅ Shipped | Complete Lexer → Parser → Semantic → Codegen compilation chain. |
| **SQLite model & seed engine** | ✅ Shipped | Automatic database migration, schema alignment, and development data seeding. |
| **RTL & Arabic-First design** | ✅ Shipped | Native RTL layout mirroring and automated conversion of properties to CSS Logical Properties. |
| **Multi-File imports graph** | ✅ Shipped | Relative imports (`import "./file.amana"`) supporting recursive resolution and deduplication. |
| **Auth Scaffolding** | ✅ Shipped | `capabilities: auth` generates secure login, registration, and session middleware. |
| **18+ Layout engines** | ✅ Shipped | Support for `bento`, `masonry`, `split`, `asymmetric`, `magazine`, `sidebar`, `command-center`, `showcase-rail`, `split-diagonal`, `editorial`, `dashboard-shell`, `timeline`, and basic flex/grid layouts. |
| **4-Layer CSS sanitizer** | ✅ Shipped | Scopes custom style definitions, checks selector safety, property allowlists, and strips exploit values. |
| **Custom slots & components** | ✅ Shipped | Reusable components supporting required named slots, optional slots, and default fallbacks. |
| **Variant styling registry** | ✅ Shipped | Component design customization using `variant Component.name`. |
| **JSON diagnostics & Levenshtein** | ✅ Shipped | Machine-readable JSON output formats for IDE integration combined with Levenshtein spellchecking suggestion tips. |
| **`amana fmt`** | ✅ Shipped | Code formatter with recursive `--all` graph parsing. |
| **`amana dev`** | ✅ Shipped | Hot-reloading watcher rebuild loop with WebSocket browser refresh integration. |
| **`amana inspect-design`** | ✅ Shipped | Audits design layouts, checking diversity, responsiveness, and token consistency. |
| **Language Server Protocol (LSP)** | ✅ Shipped | Active stdio LSP server yielding completions, formatting hooks, and realtime diagnostics. |
| **9 Premium showcase applications** | ✅ Shipped | Full-stack examples library demonstrating the layout engine (located in `examples/`). |
| **AGPLv3 License** | ✅ Shipped | Open-source licensing. |

---

## 🔄 In Progress & Planned (v2.2)

| Feature | Status | Notes |
|---|---|---|
| **VS Code marketplace extension** | 🔄 In Progress | Full syntax highlighting, error spans, and integrated live-reload dev commands. |
| **PostgreSQL engine backend** | 🗓️ Planned | Multi-tenant production DB mapping options alongside standard SQLite. |
| **`amana cloud` deployment tool** | 🗓️ Planned | Zero-config Docker-based deployments to Fly.io, Railway, or AWS. |
| **WASM compiler target** | 🗓️ Planned | Compiles the Rust compiler to WebAssembly to execute checks and formatters entirely in browser sandboxes. |

---

## 🔮 Future (v3.0)

| Feature | Notes |
|---|---|
| **`amana ui` visual designer** | Drag-and-drop schema editor compiling layouts back to clean `.amana` files. |
| **React / Vue / Svelte target** | Compile layouts to modern frontend component framework outputs instead of EJS. |
| **AI design-to-dsl** | Natural language prompts compiling directly to valid Amana code. |
| **Third-Party plugin hooks** | Modular interface for developers to register custom layouts, paint filters, and DB adapters. |
