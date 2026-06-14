# CLI Tooling & Developer Experience (DX)

Amana is built with a developer-first focus, prioritizing fast checks, machine-readable validation diagnostics, auto-formatting, and design audits.

---

## 🛠️ CLI Command Reference

The Amana CLI can be run using `cargo run -- <command>` during development or via the compiled binary:

### 1. `check`
```bash
amana check <entry-file.amana> [--json]
```
- Performs lexical, syntactic, and semantic validation across the entire file graph.
- Returns `ok: true` or highlights compilation errors with line/column pointers.

### 2. `build`
```bash
amana build <entry-file.amana> [output-directory] [--json]
```
- Generates the full Express/EJS/SQLite web app distribution.
- Emits assets, route controllers, migrations, and database seed scripts.

### 3. `fmt`
```bash
amana fmt <file.amana> [--check] [--json] [--all]
```
- Standardizes indentation, corrects colons on component calls, and cleans self-closing nodes.
- `--check`: Returns exit code 1 if files do not match format standards (ideal for CI/CD gates).
- `--all`: Traverses imports and formats all referenced files in the source graph.

### 4. `dev`
```bash
amana dev <entry-file.amana> [output-directory] [--no-install] [--no-watch]
```
- Watches all Amana files in the graph and triggers live recompilation upon saves.
- Proxies requests to the generated runtime and enables instant browser previews.

### 5. `inspect-design`
```bash
amana inspect-design <entry-file.amana> [--json]
```
- Performs a layout design diversity audit, outputting a score out of 100.

### 6. `lsp`
```bash
amana lsp
```
- Launches the Amana Language Server Protocol over standard input/output (stdio) to feed autocomplete and diagnostic highlights to IDE extensions.

---

## 📊 Standardized JSON Diagnostics Schema

All CLI operations support the `--json` flag. When active, errors are printed in a structured JSON object to stdout.

```json
{
  "ok": false,
  "stage": "semantic",
  "line": 104,
  "column": 21,
  "message": "In view 'Home': Unknown gradient value \"spotlite\". Did you mean \"spotlight\"?",
  "suggestion": "spotlight",
  "file_path": "C:/projects/app/examples/01_saas_aura.amana"
}
```

### JSON Diagnostic Fields:
- `ok` (boolean): `true` if compilation succeeded without errors, otherwise `false`.
- `stage` (string): The compilation pipeline stage where the error occurred (`lexer`, `parser`, `semantic`, `codegen`, `fmt`).
- `line` (number | null): 1-indexed line number where the issue resides.
- `column` (number | null): 1-indexed column number within the line.
- `message` (string): Detailed human-readable explanation of the issue.
- `suggestion` (string | null): Recommended fix string (e.g. spelling correction).
- `file_path` (string): Absolute filepath of the file containing the error.

---

## 🔤 Levenshtein Spelling Suggestion Engine

To prevent typos in design configuration, the compiler's semantic stage spellchecks all values declared for **closed token properties** (`layout`, `surface`, `hover`, `entrance`/`reveal`, `gradient`, `density`, and `shadow`) inside design blocks.

If a value for these specific properties is not recognized, the compiler computes the **Levenshtein distance** (minimum number of single-character edits required to change one word into another) between the input string and all valid options. If a match is found with a distance of **`<= 2`**, the compiler automatically suggests the corrected property in the diagnostic report.

> [!NOTE]
> All other visual metadata properties (such as `uniqueness`, `freedom`, `voice`, `colorway`, `direction` in `art:`, `motif`, `lighting`, `texture`, `contrast`, `screenreader`, `feedback`, `cursor`) are **open-ended metadata fields**. They are not validated against closed lists, allowing developers to supply free-form design descriptions (e.g., `uniqueness: facebook-clone` or `motif: social-feed`).

### Allowed Keyword Sets:

- **Layouts (`layout:`)**: `row`, `column`, `stack`, `grid`, `center`, `inline`, `cluster`, `split`, `bento`, `split-diagonal`, `asymmetric`, `editorial`, `dashboard-shell`, `magazine`, `command-center`, `showcase-rail`, `masonry`, `sidebar`.
- **Surfaces (`surface:`)**: `base`, `muted`, `elevated`, `glass`, `custom`, `outline`, `flat`, `layered`, `glass-layered`.
- **Gradients (`gradient:`)**: `primary`, `accent`, `hero`, `mesh`, `aurora`, `spotlight`, `custom`, `brand`, `sunset`, `ocean`, `mesh-cyan-indigo`, `mesh-aurora`.
- **Shadows (`shadow:`)**: `sm`, `md`, `lg`, `xl`, `soft`, `floating`, `strong`, `smooth`, `none`.
- **Hovers (`hover:`)**: `lift`, `glow`, `scale`, `lift-glow`, `none`.
- **Animations (`entrance:`)**: `fade`, `slide-up`, `slide-down`, `zoom`, `blur`, `clip`, `stagger-up`, `none`.
- **Density (`density:`)**: `compact`, `comfortable`, `spacious`.

---

## 📐 Inspect Design & Audit Scorer

The `inspect-design` command rates application layouts on a scale of `0` to `100` based on three core indicators:

### 1. Layout Diversity
- Encourages expressive, non-repetitive grids.
- A layout consisting only of repeating standard `row` or `column` divisions receives a lower score.
- Using advanced structures like `bento`, `split-diagonal`, or asymmetrical containers boosts score metrics.

### 2. Responsive Coverage
- Checks for matching mobile column modifiers on wide-width containers.
- Grids containing wide elements without mobile responsive limits (e.g. `desktop_columns` present but `mobile_columns` missing) raise warnings and deduct points.

### 3. Token Consistency
- Checks if values match design tokens.
- Hardcoded color codes (e.g. `color: #ff0000`) or explicit pixel margins (e.g. `margin-left: 23px`) inside `style:` blocks bypass global themes.
- Points are deducted unless values refer to design tokens (`var(--space-md)`) or theme colors (`primary`, `border`).
