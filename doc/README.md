# Amana Documentation

This folder documents the current Amana compiler/runtime stack after the latest production hardening work.

## What Amana Is

Amana is a single-source DSL for building full-stack apps and landing pages. A `.amana` file can declare:

- app metadata
- theme tokens
- models and seeds
- routes and views
- server fetches and forms
- reusable components
- design grammar blocks

The compiler emits:

- Node.js / Express runtime
- EJS views
- SQLite schema and seed data
- generated CSS
- JSON diagnostics

## Current Capabilities

- `amana check --json` for AI repair loops
- `amana fmt` and `amana fmt --all`
- `amana build` with generated runtime files
- multi-file imports
- route params using bracket syntax like `/projects/[id]`
- server fetch pagination with `limit`, `offset`, and `page`
- theme customization with custom font families
- forms v2 and ownership constraints
- seed guards and production runtime hardening
- live preview and design feedback

## Main Docs

- [language.md](language.md)
- [cli-dx.md](cli-dx.md)
- [css-theme-rtl.md](css-theme-rtl.md)
- [html-components-forms.md](html-components-forms.md)
- [node-runtime-security.md](node-runtime-security.md)
- [multi-file-lsp.md](multi-file-lsp.md)
- [live-preview.md](live-preview.md)
- [examples-gallery.md](examples-gallery.md)
- [roadmap.md](roadmap.md)

## Examples

The `examples/` folder currently contains 5 premium verified examples showcasing the layout engine.

## Verification

```powershell
cargo test
cargo run -- check examples\01_saas_aura.amana --json
cargo run -- build examples\01_saas_aura.amana examples\01_saas_aura_dist --json
```

