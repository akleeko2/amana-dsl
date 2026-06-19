# Changelog

All notable changes to the Amana compiler, language surface, and generated runtime are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow the internal `vMAJOR.MINOR.PATCH` tags used in commit history (e.g. `v2.2.0`). The IR schema carries its own independent version (`amana_ir.json` → `ir_version`, currently `1.0.0`).

> **Versioning note:** The Rust crate version in `Cargo.toml` (`0.1.0`) has drifted from the feature tags in commit history (`v2.x.y`). The feature tags are the authoritative human-facing version until the crate version is realigned. This is tracked as a documentation-level discrepancy, not a code bug.

## Categories

- **Added** — new language syntax, components, CLI flags, or runtime features.
- **Changed** — changes to existing behavior, generated output, or docs.
- **Fixed** — bug fixes in compiler, codegen, or runtime.
- **Security** — security-relevant changes (auth, CSRF, sessions, sanitization).
- **Docs** — documentation-only changes.
- **Removed** — features or files removed.

---

## [Unreleased]

### Added
- **`doc/getting-started.md`** — first end-user onboarding guide (install → first app → auth + data). Bilingual.
- **`doc/design-anti-patterns.md`** — unified bilingual design anti-patterns reference; each rule tagged `SOLVED` / `ACTIVE RISK` / `ENFORCED`.
- **`CHANGELOG.md`** — this file.
- **Known Issues table** in `doc/language-runtime-trust-plan.md` — canonical, maintained list of active component/layout bugs (`Modal`, `Grid` stretch, tablet breakpoints, laptop overflow, `FormField` textarea in modals), with severity and workarounds.
- Explicit clarification of `n/a` cells in the trust-plan coverage matrix.

### Changed
- **Governance rule strengthened:** a feature with an active Known Issue can no longer be described as `implemented` / `production_safe: yes` in the coverage matrix. The `roadmap.md` "Implemented" table is now scoped to core language/runtime surfaces; visual component polish gaps live in Known Issues.
- **`doc/examples-gallery.md`** rewritten to reflect the actual `examples/` directory (`landing.amana`, `royal_deck.amana`, `test_ternary.amana`). The deleted numbered examples (`01_saas_aura` … `09_multi_file_portal`) are explicitly marked non-authoritative.
- **`doc/cli-dx.md`** now contains the full `amana dev` / live-rebuild workflow; the standalone `live-preview.md` is merged in.
- **`.gitignore`** expanded to cover `scratch/`, `المستقبل/`, `task.md`, `*_dist/`, `examples_dist_test/`, `examples_dist/`, `royal_deck_dist/`, `.amana_live_dist/`, and other build/test artifact directories.
- **`AMANA_VISUAL_LANGUAGE_AUDIT.md`** "Remaining Verified Bugs" section now points to the trust-plan Known Issues as the source of truth.

### Fixed
- **Missing compiler sources in git** — 20 source files under `src/parser/`, `src/semantic/`, and `src/codegen/express/` (~7,979 lines, including the entire `static_files/` runtime generator, `express/theme.rs`, `express/views.rs`, `parser/{css,design,expressions,styles,top_level,views}.rs`, and `semantic/{schema,scope,suggestions,types,views}.rs`) were untracked and are now committed.
- **Missing apps/docs/scripts/examples in git** — the `apps/` multi-file applications, the generated `doc/language-inventory.generated.md`, `doc/language-runtime-trust-plan.md`, `doc/AMANA_VISUAL_LANGUAGE_AUDIT.md`, `scripts/language_inventory.py`, `scripts/search-language.ps1`, `examples/{landing,royal_deck,test_ternary}.amana`, and `AMANA_DEVELOPMENT_STANDARDS.md` were untracked and are now committed.

### Removed
- **`doc/design-bugs-prevention.md`** and **`doc/design-mistakes-avoidance.md`** — superseded by the unified `doc/design-anti-patterns.md`.
- **`doc/live-preview.md`** — merged into `doc/cli-dx.md`.

### Docs
- Reconciled the contradiction between `roadmap.md` (claimed everything "Implemented") and `AMANA_VISUAL_LANGUAGE_AUDIT.md` (listed 5 verified current bugs). The audit bugs are now first-class entries in the trust-plan Known Issues and the roadmap's Remaining Work.

---

## [v2.2.0] — Facebook feed responsiveness refactor
### Changed
- Refactored Facebook feed responsiveness using CSS `clamp()` and inline styles.

## [v2.1.9] — Facebook post redesign
### Fixed
- Redesigned Facebook posts and fixed composer action vertical-stretch bug.

## [v2.1.8] — Facebook feed polish
### Changed
- Polished Facebook feed visual presentation and aligned terminology.

## [v2.1.7] — Docs alignment with compiler rules
### Changed
- Aligned documentation with compiler rules for imports placement, design values, and URL CSS sanitizer.

## [v2.1.6] — Roadmap sync
### Added
- `permit` end-to-end enforcement, `auth_model` generalization, `tokens` IR/CSS emission, `persist` browser persistence, `Chart` parser syntax, ternary parser support, Resource lifecycle runtime behavior, `variants` IR/CSS/runtime application.
### Docs
- Synchronized `roadmap.md` with shipped features.

## [v2.1.4] – [v2.1.0] — Example apps 6–9
### Added
- Example apps: Atelier Aurelia, Nexus Portal, multi-file portal, Pro Dashboard.
### Fixed
- CSS Grid cyclic auto-fit validation bug; flex-shrink header squeezing on mobile.
- Mobile responsiveness issues in examples 6, 7, 8.
- Hyphenated keywords parsing.

## [v2.0.7] — CSP fix
### Security
- Added `'unsafe-eval'` to CSP `scriptSrc` to allow AlpineJS execution; allowlisted `logoipsum.com` in `imgSrc`.

## [v2.0.6] — init attribute
### Added
- `init` attribute mapped to Alpine `x-init`.

## [v2.0.x] and earlier
- Initial Express/EJS/SQLite backend, lexer, parser, semantic analyzer, IR generation, formatter, JSON diagnostics, LSP server, design inspection.

---

## How To Maintain This File

1. When you change the **language surface** (new syntax, new component, changed validation), add an entry under `[Unreleased]` with the right category.
2. When you tag a release, rename `[Unreleased]` to `[vX.Y.Z] — <short description>` with the tag date, and open a fresh `[Unreleased]`.
3. **Security changes must always be recorded**, even small ones (CSP tweaks, sanitization, session config).
4. Keep IR-version bumps visible: if `ir_version` in `amana_ir.json` changes, note it explicitly so downstream tooling knows snapshots may be incompatible.
5. Cross-link Known Issues: when a change fixes a Known Issue, mark the issue row as removed in `doc/language-runtime-trust-plan.md` in the same commit.
