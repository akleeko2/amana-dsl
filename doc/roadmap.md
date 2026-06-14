# Amana DSL — Roadmap

This document tracks the public roadmap for Amana. It is updated as features are planned, in progress, or completed.

---

## ✅ Shipped (v0.1 — Current)

| Feature | Status |
|---|---|
| Full-stack compiler pipeline (Lexer → Parser → Semantic → Codegen) | ✅ Done |
| SQLite model → migration → seed pipeline | ✅ Done |
| 8+ premium layout engines (`bento`, `masonry`, `split`, `asymmetric`, `magazine`, `sidebar`, `command-center`, `showcase-rail`) | ✅ Done |
| 4-layer CSS security sanitizer (selector, property, value, layer scope) | ✅ Done |
| Custom components with named slots and optional slots | ✅ Done |
| Variant registry (`variant Component.style`) | ✅ Done |
| JSON diagnostic output (`--json` flag) | ✅ Done |
| `amana fmt` — source code formatter | ✅ Done |
| `amana inspect-design` — layout diversity analysis | ✅ Done |
| Language Server Protocol (LSP) skeleton | ✅ Done |
| 9 premium example applications | ✅ Done |
| AGPLv3 open-source license | ✅ Done |

---

## 🔄 In Progress (v0.2)

| Feature | Status |
|---|---|
| VS Code extension with syntax highlighting and inline errors | 🔄 Active |
| `amana check` — improved error messages with line-number spans | 🔄 Active |
| `amana dev` — watch mode with live rebuild on save | 🔄 Active |

---

## 🗓️ Planned (v0.3)

| Feature | Notes |
|---|---|
| **PostgreSQL backend** | Alternative to SQLite for production deployments |
| **Auth scaffolding** | `capabilities: auth` generates full login, signup, session middleware |
| **`amana cloud` deploy** | One-command deploy to Fly.io / Railway |
| **RTL / Arabic-first design** | Full RTL layout support for Arabic and Persian apps |

---

## 🔮 Future (v1.0)

| Feature | Notes |
|---|---|
| **WASM compiler target** | Run the Amana compiler entirely in the browser |
| **Multi-file import graph** | `import "./models/user.amana"` across files |
| **`amana ui`** | Visual drag-and-drop `.amana` editor |
| **React / Vue output target** | Compile `.amana` to React components instead of EJS |
| **AI design-to-amana** | Describe a UI in natural language → generate `.amana` |
| **Plugin system** | Community-built layout engines and component libraries |

---

## 💬 Request a Feature

Open a [Feature Request](https://github.com/akleeko2/amana-dsl/issues/new?template=feature_request.yml) on GitHub.

We prioritize features based on community demand and sponsor support.
