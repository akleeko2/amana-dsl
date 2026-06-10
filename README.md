# Amana

Amana is an AI-friendly full-stack DSL that compiles `.amana` files into a Node.js/Express app with EJS views, SQLite, generated CSS, Alpine.js behavior, server-side forms, and JSON diagnostics.

## Quick Start

```powershell
cargo test
cargo run -- check examples\01_saas_aura.amana --json
cargo run -- build examples\01_saas_aura.amana .amana_saas_dist --json
```

Run the generated app:

```powershell
cd .amana_saas_dist
npm install
npm run dev
```

## Current Focus

- `amana check --json` for AI repair loops
- `amana fmt` and `amana fmt --all`
- `amana build` with generated EJS views
- multi-file imports
- CSS DSL v2 and theme system
- forms v2, seeds, and runtime security
- live preview and design feedback

## Examples

The `examples/` folder currently contains 5 premium examples with different structures and visual directions.

See:

- [examples/README.md](examples/README.md)
- [doc/README.md](doc/README.md)

## Documentation

Start with [doc/README.md](doc/README.md).

