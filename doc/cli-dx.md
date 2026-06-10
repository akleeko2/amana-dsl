# CLI and Developer Experience (DX)

Amana is optimized for an autonomous AI-assisted development loop:

```text
generate -> check --json -> fix -> fmt -> build -> preview -> review -> repeat
```

The CLI provides fast, machine-readable validation messages that allow IDE extensions and AI agents to instantly resolve structural, logical, or design-token bugs.

---

## Command Reference

Run the compiler via Cargo during development:

```powershell
# Check project syntax and semantic logic, outputting detailed JSON report
cargo run -- check app.amana --json

# Format a single Amana file (corrects indents, colons, self-closing components)
cargo run -- fmt app.amana

# Verify formatting state without rewriting, returning JSON status
cargo run -- fmt app.amana --check --json

# Format all files in the current workspace/import graph
cargo run -- fmt app.amana --all

# Build optimized production Node.js engine and static assets
cargo run -- build app.amana dist --json

# Watch workspace files and trigger automatic livereload server
cargo run -- dev app.amana dist

# Inspect design token usage, layout ratios, and responsiveness
cargo run -- inspect-design app.amana --json

# Start standard language server protocol (LSP) over stdio
cargo run -- lsp
```

---

## Machine-Readable JSON Diagnostics

All compiler operations support the `--json` flag. Error signals return standardized JSON structures with line, column, and suggestion information:

```json
{
  "ok": false,
  "stage": "semantic",
  "line": 48,
  "column": 20,
  "message": "In view 'Home': Unknown gradient value \"mesh slate blue\". Valid values: primary, accent, hero, mesh, aurora, spotlight, custom, brand, sunset, ocean, mesh-cyan-indigo, mesh-aurora.",
  "suggestion": "Did you mean \"mesh\"?",
  "file_path": "examples/01_saas_aura.amana"
}
```

### Response Schema:
- `ok` (boolean): `true` if operation completed successfully, otherwise `false`.
- `stage` (string): Compile stage where error was raised (`parser`, `semantic`, `codegen`, `fmt`).
- `line` (number/null): 1-indexed line number in source file.
- `column` (number/null): 1-indexed column number in source file.
- `message` (string): Verbose description of error.
- `suggestion` (string/null): Actionable fix suggestion (e.g. spelling correction, correct component syntax).
- `file_path` (string): Absolute or relative file path of source file.

---

## Design Grammar Spellchecking (Levenshtein Suggestions)

To prevent typos in design configuration, the semantic stage validates all properties inside design blocks (e.g., `visual`, `compose`, `responsive`, `style`) against their expected keyword lists. 

If a value is unrecognized, the compiler computes the **Levenshtein distance** between the input value and all permitted options. If a match is found with a distance of **`<= 2`**, the compiler automatically includes a recommended suggestion in the diagnostic report.

### Checked Keyword Categories:
- **Layouts (`layout:`)**: `row`, `column`, `stack`, `grid`, `center`, `inline`, `cluster`, `split`, `bento`, `split-diagonal`, `asymmetric`, `editorial`, `dashboard-shell`, `magazine`, `command-center`, `showcase-rail`, `masonry`.
- **Surfaces (`surface:`)**: `base`, `muted`, `elevated`, `glass`, `custom`, `outline`, `flat`, `layered`, `glass-layered`.
- **Gradients (`gradient:`)**: `primary`, `accent`, `hero`, `mesh`, `aurora`, `spotlight`, `custom`, `brand`, `sunset`, `ocean`, `mesh-cyan-indigo`, `mesh-aurora`.
- **Shadows (`shadow:`)**: `sm`, `md`, `lg`, `xl`, `soft`, `floating`, `strong`, `smooth`, `none`.
- **Hovers (`hover:`)**: `lift`, `glow`, `scale`, `lift-glow`, `none`.
- **Entrance Animations (`entrance:`)**: `fade`, `slide-up`, `slide-down`, `zoom`, `blur`, `clip`, `stagger-up`, `none`.
- **Density (`density:`)**: `compact`, `comfortable`, `spacious`.

---

## Inspect Design command

The `inspect-design` command performs design diagnostics and scores your layout. It reports:
- **Layout Diversity**: Detects repetitive row/column patterns and encourages expressive bento grid or diagonal sections.
- **Responsive Coverage**: Warns if mobile/tablet columns are missing on complex wide section grids.
- **Token Consistency**: Inspects if raw color hexes or hardcoded margins are used instead of custom design tokens (like `--space-md` or `--radius-lg`).
