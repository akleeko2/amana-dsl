# Multi-File Compilation Architecture & LSP (DSL v2.1)

Amana supports modular code distribution, allowing projects to scale from single-file scripts to nested multi-file source graphs.

---

## 🏛️ Multi-File Imports Syntax

Use the `import` statement to link other modules:

```amana
import "./models/schema.amana"
import "./views/dashboard.amana"
```

### Resolution Rules:
- **Relative Resolution**: Paths must be relative to the importing file's directory.
- **Flexible Placement**: The compiler preprocessor scans and extracts `import` lines from *anywhere* in the source file before parsing. While declaring imports at the top is standard convention, they can safely appear anywhere in the file (e.g., after the `app` or `theme` blocks).
- **Deduplication**: If multiple files import the same `.amana` file, the compiler parses and links it only once, resolving circular dependency trees cleanly.
- **Global Scope Resolution**: Models, custom components, and views declared inside imported files are registered in the application's global scope. If two files declare identical model or view names, the compiler raises a duplicate symbol validation error.

---

## ⚙️ Compilation Graph Actions

When executing CLI subcommands, the compiler constructs a dependency graph starting from the entrypoint:

### 1. `amana check entry.amana`
Parses and semantically validates all files in the import graph.
- Diagnostic logs automatically report the absolute `file_path` for each error.

### 2. `amana build entry.amana [dist]`
Compiles and generates the runtime target.
- Merges all imported SQLite schemas into a single migration script.
- Populates seed records across all modules.
- Groups all view templates into the output Express application.

### 3. `amana fmt entry.amana --all`
Recursively traverses all imports starting from the entrypoint and formats all files in place.

### 4. `amana dev entry.amana`
Watches the entire dependency graph. Saving any modified file triggers an automatic rebuild of the target Express application.

---

## 🔌 Language Server Protocol (LSP)

Amana includes a built-in Language Server Protocol (LSP) server configured to integrate directly with IDE extensions (like VS Code or Neovim).

### Active Capabilities:

- **JSON Diagnostics**:
  - The LSP spawns the semantic checker in memory.
  - Validation checks report problems in real time as the developer types.
  - Errors map line/column positions and link the precise `file_path` to focus the editor viewport.
- **Completions**:
  - Autocompletes built-in standard library components.
  - Suggests allowed keywords inside design blocks (e.g. suggests valid layouts, shadows, gradients, and density strings).
  - Autocompletes database fields based on active model declarations within the import graph.
- **Document Formatting**:
  - Automatically formats on saves (`editor.formatOnSave` integration).
  - Leverages the same AST printer from the `fmt` engine to guarantee deterministic layout indentation and syntax corrections across files.
