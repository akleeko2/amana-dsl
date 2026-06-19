# Multi-File Programs And LSP

This document covers implemented import graph behavior in `src/main.rs` and the current LSP server.

## Imports

```amana
import "./models.amana"
import "./components/cards.amana"
```

Rules:

- Import directives are stripped before parsing.
- Imports may appear anywhere in the file.
- Syntax must be exactly an `import` word followed by a quoted path.
- Text after the closing quote is allowed only when it starts with `#`.
- Relative imports resolve against the importing file's directory.
- Absolute paths are accepted.
- Missing files produce an `imports` stage diagnostic with line/column.
- Circular graphs produce an `imports` stage diagnostic.
- Duplicate imports are deduplicated by canonical path.
- Duplicate top-level public symbols across the resolved graph are rejected before IR generation. `model`, `view`, and `component` share one symbol namespace; `route` paths, `variant` targets, and the single `app` declaration are checked with their own collision keys.

## Graph Compilation

For `check`, `build`, `dev`, and LSP diagnostics, the compiler resolves the source graph before semantic analysis.

Resolution order:

1. Normalize entry path.
2. Read source.
3. Strip import lines.
4. Recursively resolve imported files.
5. Parse each cleaned file.
6. Merge all parsed nodes into one program node list.

The merged program is then semantically validated and converted into IR.

## Formatter Graph Mode

```powershell
amana fmt app.amana --all
```

`--all` uses the import graph and formats every reachable `.amana` file.

Without `--all`, only the provided source file is formatted.

## Dev Watch Mode

```powershell
amana dev app.amana dist
```

The dev watcher tracks modification times for every reachable source file in the import graph. When any file changes, it rebuilds the generated project and checks `runtime/engine.js` with Node.

## LSP

Start:

```powershell
amana lsp
```

Alias:

```powershell
amana language-server
```

Transport:

- stdio
- JSON-RPC 2.0
- `Content-Length` framed messages

Implemented methods:

- `initialize`
- `textDocument/didOpen`
- `textDocument/didChange`
- `textDocument/didSave`
- `textDocument/completion`
- `textDocument/hover`
- `textDocument/definition`
- `textDocument/formatting`

Diagnostics:

- The LSP resolves and compiles the source graph.
- Diagnostics use compiler stages such as `lexer`, `parser`, and `semantic`.
- Diagnostics include line/column when available.
- Suggestions are appended to the diagnostic message when available.

Completion:

Completion combines the static language/design keywords with a semantic project index built from the resolved import graph.

Semantic completion currently includes:

- top-level definitions from reachable files: apps, models, routes, views, components, and variants
- the configured `<auth_model>.current` principal expression
- `current` after typing `<auth_model>.`
- fields after typing `<auth_model>.current.`, including implicit `id`

Hover:

- Hover over a known top-level symbol returns its kind, source file, line/column, and model details when applicable.
- Model hover includes fields and permit rule count.
- Hover over `<auth_model>.current` shows the configured principal and available fields.

Go-to-definition:

- Definition lookup resolves top-level symbols across the import graph.
- Member chains such as `Project.all` fall back to the `Project` model definition.

Formatting:

`textDocument/formatting` returns a full-file replacement using the same formatter as `amana fmt`.

## Current Limits

- Rename and references are not implemented yet.
- Field-level definition currently points through the owning symbol only; arbitrary model field definitions are not exposed as separate LSP targets yet.
- LSP graph resolution can read files from disk, while open-buffer overlays are only used for the active document path during diagnostics.
