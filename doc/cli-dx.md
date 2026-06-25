# CLI And Developer Workflow

This document describes the CLI implemented in `src/main.rs`, the formatter in `src/formatter.rs`, and the documentation helper scripts in `scripts/`.

During development, run commands through Cargo:

```powershell
cargo run -- check app.amana --json
```

After building/installing the binary, use:

```powershell
amana check app.amana --json
```

## Commands

### `check`

```powershell
amana check <source-file.amana> [--json] [--snapshot-ir [path]] [--verify-ir-snapshot [path]]
```

Validates the full source graph:

- import resolution
- lexing
- parsing
- semantic validation
- optimization
- IR generation

It does not write the generated web application unless `--snapshot-ir` is used.

### `build`

```powershell
amana build <source-file.amana> [output-dir] [--json] [--snapshot-ir [path]] [--verify-ir-snapshot [path]]
```

Runs the full compiler pipeline and writes the generated Express project.

Default output directory:

```text
./dist
```

### Legacy Build Mode

```powershell
amana <source-file.amana> [output-dir]
```

This is accepted for backward compatibility and maps to `build`.

### `dev`

```powershell
amana dev <source-file.amana> [output-dir] [--no-install] [--no-watch]
```

Behavior:

- Builds the project.
- Runs `npm install` unless `--no-install` is supplied.
- Watches the import graph and rebuilds on source changes unless `--no-watch` is supplied.
- Starts `npm run dev` in the generated output directory.
- After rebuilds, checks `runtime/engine.js` with `node --check`.

### `fmt`

```powershell
amana fmt <source-file.amana> [--check] [--json] [--all]
```

Formatter behavior:

- Normalizes line endings to LF.
- Converts tabs to four spaces before indentation normalization.
- Detects indentation unit and normalizes to four-space levels.
- Collapses excessive blank lines.
- Converts empty self-closing component calls from `Component():` to `Component()`.

Flags:

- `--check`: fail if formatting would change the file.
- `--json`: emit JSON success/error output.
- `--all`: format the entry file and all imports in the source graph.

### `inspect-design`

```powershell
amana inspect-design <source-file.amana> [--json]
amana design-report <source-file.amana> [--json]
```

Runs semantic compilation and produces a design audit report.

The report includes:

- score
- per-view component count
- design blocks used
- standard components used
- AI/design control strings
- structured warnings

### `generate`

```powershell
amana generate <page|component> <name> [--pattern <pattern>] [--model <model>]
amana g <page|component> <name> [--pattern <pattern>] [--model <model>]
```

Generates boilerplate `.amana` files for views, components, and pages.

- `element_type`: Either `page` or `component`.
- `name`: Symbol and filename for the generated boilerplate.
- `--pattern`: Pre-configured layout pattern for `page` generation. Supported values: `PricingSection`, `DashboardShell`, `AuthPage`, `SettingsPage`.
- `--model`: (For custom components) Optional model name to bind.

Examples:
```powershell
# Generate a Settings page using the SettingsPage layout pattern:
amana generate page user_settings --pattern SettingsPage

# Generate a custom TaskCard component:
amana generate component task_card --model Task
```

### `lsp`

```powershell
amana lsp
amana language-server
```

Starts the Language Server Protocol implementation over stdio.

Current LSP behavior:

- `initialize`
- `textDocument/didOpen`
- `textDocument/didChange`
- `textDocument/didSave`
- `textDocument/completion`
- `textDocument/hover`
- `textDocument/definition`
- `textDocument/formatting`
- diagnostics from the same compiler pipeline

Completion is semantic as well as static: the server indexes the resolved source graph and offers reachable models, views, components, routes, variants, `<auth_model>.current`, and fields on `<auth_model>.current`. Hover and definition requests resolve top-level symbols across imports; member chains such as `Project.all` resolve back to the `Project` model definition.

## JSON Diagnostics

Errors with `--json` use:

```json
{
  "ok": false,
  "stage": "semantic",
  "line": 12,
  "column": 5,
  "message": "In view 'Home': ...",
  "suggestion": null,
  "file_path": "C:/project/app.amana"
}
```

Fields:

- `ok`: false for errors.
- `stage`: `file`, `imports`, `lexer`, `parser`, `semantic`, `ir`, `codegen`, or `formatter`.
- `line`, `column`: present when extracted from the error message.
- `message`: human-readable diagnostic.
- `suggestion`: optional repair hint.
- `file_path`: source path when available.

Success output:

```json
{
  "ok": true,
  "stage": "check",
  "message": "Check completed successfully.",
  "output_dir": null,
  "ir_snapshot": null
}
```

## IR Snapshots

IR snapshots are supported by `check` and `build`.

Write default snapshot:

```powershell
amana check app.amana --snapshot-ir
```

Default path:

```text
<source-dir>/.amana_snapshots/<source-stem>.ir.json
```

Write explicit path:

```powershell
amana build app.amana dist --snapshot-ir snapshots/app.ir.json
```

Verify snapshot:

```powershell
amana check app.amana --verify-ir-snapshot snapshots/app.ir.json
```

Verification fails if the canonical pretty-printed IR differs.

## Documentation Scripts

Generate the implementation-derived language inventory:

```powershell
python scripts/language_inventory.py --write
```

Verify the generated inventory is current:

```powershell
python scripts/language_inventory.py --check
```

Print raw inventory JSON:

```powershell
python scripts/language_inventory.py --json
```

Search language implementation by area:

```powershell
scripts/search-language.ps1 -Area lexer
scripts/search-language.ps1 -Area parser
scripts/search-language.ps1 -Area semantic
scripts/search-language.ps1 -Area codegen
scripts/search-language.ps1 -Area runtime
scripts/search-language.ps1 -Area all
```

Generate inventory before searching:

```powershell
scripts/search-language.ps1 -Inventory -Area all
```

Available search areas:

```text
all, lexer, parser, semantic, codegen, runtime, docs,
theme, design, forms, queries, components
```

## Recommended Change Workflow

1. Edit compiler/runtime code.
2. Run `python scripts/language_inventory.py --write`.
3. Update the relevant hand-written doc.
4. Run `python scripts/language_inventory.py --check`.
5. Run `cargo test`.
6. Run `cargo run -- check <real .amana file> --json`.
7. If output is affected, run `cargo run -- build <file> <dist> --json` and `node --check <dist>/runtime/engine.js`.

## Dev Server And Live Rebuild

> The former `live-preview.md` has been merged into this section. `amana dev` is the live workflow entry point.

```powershell
amana dev <source-file.amana> [output-dir] [--no-install] [--no-watch]
```

Default output directory:

```text
./dist
```

### Startup Sequence

1. Resolve the full Amana source graph.
2. Compile to IR.
3. Generate the Express project into the output directory.
4. Run `npm install` in the output directory unless `--no-install` is supplied.
5. Start the source graph watcher unless `--no-watch` is supplied.
6. Run `npm run dev` in the output directory.

Generated `package.json` maps:

```json
{
  "dev": "nodemon app.js",
  "start": "node app.js"
}
```

### Watch Behavior

The watcher:

- Collects all files reachable from the entry file through `import`.
- Checks modification timestamps every 300ms.
- Debounces rebuilds for roughly 900ms.
- Rebuilds the generated output directory when any source file changes.
- Runs `node --check <output-dir>/runtime/engine.js` after successful rebuilds.

The watcher does not run browser automation by itself. Visual verification should be performed separately after the generated app starts.

### Runtime URL

The generated runtime listens on:

```text
http://localhost:3000
```

unless `PORT` is set in the environment.

### Useful Development Commands

```powershell
cargo run -- dev app.amana dist
cargo run -- dev app.amana dist --no-install
cargo run -- dev app.amana dist --no-watch
```

Manual runtime check:

```powershell
node --check dist\runtime\engine.js
```

Manual generated app start:

```powershell
cd dist
npm install
npm run dev
```

### Dev Server Current Limits

- The dev command does not proxy to another server; it starts the generated Express app directly through `npm run dev`.
- Browser refresh is handled by the generated Node/Nodemon workflow, not by the Rust compiler process.
- Runtime syntax checking currently targets `runtime/engine.js`.
