# Contributing to Amana DSL

Thank you for your interest in contributing to **Amana** — the declarative full-stack DSL compiler. Every contribution, large or small, makes a real difference.

---

## 🧭 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Ways to Contribute](#ways-to-contribute)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Making Changes](#making-changes)
- [Commit Style](#commit-style)
- [Pull Request Process](#pull-request-process)
- [Writing Tests](#writing-tests)
- [Adding New Layout Engines](#adding-new-layout-engines)
- [Adding New Standard Components](#adding-new-standard-components)
- [Style Guide](#style-guide)

---

## Code of Conduct

This project follows the [Contributor Covenant](https://www.contributor-covenant.org/). Be kind, respectful, and constructive. Harassment of any kind will not be tolerated.

---

## Ways to Contribute

| Type | Where to start |
|---|---|
| 🐛 Bug fix | [Open Issues](https://github.com/akleeko2/amana-dsl/issues?q=is%3Aissue+is%3Aopen+label%3Abug) |
| ✨ New feature | [Feature Requests](https://github.com/akleeko2/amana-dsl/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement) |
| 🏷️ Good first issue | [Good First Issues](https://github.com/akleeko2/amana-dsl/labels/good%20first%20issue) |
| 📖 Documentation | Edit any file in `doc/` |
| 🧪 Tests | Add cases to `src/` test modules |
| 🎨 New example | Add a `.amana` file to `examples/` |
| 🔤 New layout engine | See [Adding New Layout Engines](#adding-new-layout-engines) |

---

## Development Setup

### Requirements

- **Rust 1.75+** — install from [rustup.rs](https://rustup.rs/)
- **Node.js 18+** — for testing generated runtime output

### 1. Fork & Clone

```bash
git clone https://github.com/<your-username>/amana-dsl.git
cd amana-dsl
```

### 2. Build

```bash
cargo build
```

### 3. Run Tests

```bash
cargo test
```

All tests must pass before you open a PR. ✅

### 4. Run a Specific Test

```bash
cargo test <test_name>
# e.g.
cargo test test_semantic_layout
```

### 5. Build an Example

```bash
cargo run -- build examples/01_saas_aura.amana .amana_test_dist
cd .amana_test_dist && npm install && npm run dev
```

---

## Project Structure

```
amana-dsl/
├── src/
│   ├── main.rs          # CLI entry point (build, check, fmt, dev, lsp)
│   ├── lexer/           # Tokenization
│   ├── parser/          # AST construction
│   ├── semantic/        # Type checking, layout validation, scoping
│   │   └── mod.rs       # LAYOUT_VALUES, THEME_VALUES allowlists
│   └── codegen/
│       └── express.rs   # Node/Express/EJS/CSS code generation
│
├── examples/            # 5 premium .amana examples
├── doc/                 # Markdown documentation
├── scripts/             # Build utilities
└── Cargo.toml
```

### Key Constants to Know

**`src/semantic/mod.rs`**
- `LAYOUT_VALUES` — all valid layout engine names
- `THEME_SURFACE_VALUES` — valid `surface:` tokens
- `ALLOWED_CSS_PROPERTIES` — property safety allowlist

**`src/codegen/express.rs`**
- CSS generation per layout type
- EJS template rendering
- SQLite migration emission

---

## Making Changes

1. **Create a branch** from `main`:
   ```bash
   git checkout -b feat/my-feature
   # or
   git checkout -b fix/parser-crash-on-empty-slot
   ```

2. **Make your changes** with focused, atomic commits.

3. **Add tests** for any new behavior (see [Writing Tests](#writing-tests)).

4. **Run the full test suite**:
   ```bash
   cargo test
   ```

5. **Run clippy** (no warnings allowed):
   ```bash
   cargo clippy -- -D warnings
   ```

6. **Format your code**:
   ```bash
   cargo fmt
   ```

---

## Commit Style

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short description>

[optional body]
[optional footer]
```

**Types:**

| Type | When |
|---|---|
| `feat` | New language feature, layout, or CLI command |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `test` | Adding or modifying tests |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `perf` | Performance improvement |
| `chore` | Tooling, CI, dependencies |

**Examples:**
```
feat(semantic): add command-center layout validation
fix(codegen): correct EJS slot rendering for optional slots
docs(language): document variant registry syntax
test(parser): add edge cases for empty component calls
```

---

## Pull Request Process

1. Ensure your PR **targets the `main` branch**.
2. Fill in the **PR template** completely.
3. Link any related issues with `Closes #123`.
4. Ensure all **CI checks pass**.
5. Request a review from a maintainer.
6. Be responsive to feedback — PRs inactive for 14 days may be closed.

---

## Writing Tests

Tests live inline in the source files using Rust's `#[cfg(test)]` module pattern.

### Adding a Semantic Test

```rust
// In src/semantic/mod.rs (or a sub-module)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_layout_validation() {
        // Arrange
        let input = r#"
            compose:
                layout: my-new-layout
        "#;
        // Act + Assert
        let result = validate_compose_block(input);
        assert!(result.is_ok());
    }
}
```

### Adding a Codegen Test

```rust
#[test]
fn test_ejs_output_for_kpi_component() {
    let ast = /* build minimal AST */;
    let output = generate_express(ast);
    assert!(output.contains("<div class=\"kpi-card\">"));
}
```

### Adding a Full Example Test

Place a new `.amana` file in `examples/` and add a build integration test:

```rust
#[test]
fn test_example_compiles() {
    let result = compile_file("examples/my_example.amana", "/tmp/test_out");
    assert!(result.is_ok(), "Compile failed: {:?}", result.err());
}
```

### Running Visual Smoke Tests

We use Playwright to run visual smoke and regression tests asserting correct layout, CSS alignment, and component interactivity (such as focus traps and scroll locking on Modals).

1. **Install dependencies**:
   ```bash
   npm install
   npx playwright install chromium
   ```

2. **Run tests**:
   ```bash
   npm run test:visual
   ```

This command compiles the test Amana application (`examples/test_modal_grid.amana`), spawns the Express server, and runs Playwright assertions.

---

## Adding New Layout Engines

1. **Register the name** in `src/semantic/mod.rs`:
   ```rust
   const LAYOUT_VALUES: &[&str] = &[
       // ... existing layouts ...
       "my-new-layout",
   ];
   ```

2. **Add allowed compose keys** in the semantic validator for the new layout.

3. **Add CSS generation** in `src/codegen/express.rs`:
   ```rust
   "my-new-layout" => {
       css.push_str(".compose-my-new-layout { /* your CSS */ }");
   }
   ```

4. **Write a test** in `src/semantic/mod.rs`.

5. **Add a documentation entry** in `doc/language.md` under `layout-specific grammars`.

6. **Add an example** that uses the new layout in `examples/`.

---

## Adding New Standard Components

1. **Register the name** in the parser's standard component list.

2. **Add EJS template** in `src/codegen/express.rs` — the component rendering function.

3. **Document** the component's props in `doc/language.md` under `standard components`.

4. **Add a test** verifying the component renders correctly in EJS output.

---

## Style Guide

- **Rust**: Follow `rustfmt` defaults. Run `cargo fmt` before every commit.
- **Amana source** (`.amana` files): 4-space indentation, one block per line.
- **Documentation**: Plain English, active voice, short sentences.
- **No commented-out code** in PRs — delete it or open a tracking issue.

---

## Questions?

Open a [Discussion](https://github.com/akleeko2/amana-dsl/discussions) — we're happy to help!
