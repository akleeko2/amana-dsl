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

## [v3.0.0] — 2026-06-23: Production-Ready Layout Components, Design Tokens & Arabic Canvas RTL Fixes

### Added
- **Design Tokens & Theme Presets**: Support for luxury, stripe, and linear themes with dynamic switching of CSS variables (colors, border-radius, spacing, density, etc.).
- **Localized Styling Overrides for Luxury Modals**: Form inputs inside Luxury theme modals elevated with gold/amber borders, custom hover/focus glows, and structured vertical spacing and label alignments.
- **Accessibility Enhancements (Layer 1-A)**: Completed ARIA annotations and keyboard navigation (left/right arrow controls for Tabs, accordion ARIA integration, focus traps).
- **Layout Components (Layer 1-B)**: Added `Center`, `Cover`, `Reel`, and `Masonry` components to the compiler and styling libraries.
- **State System Components (Layer 1-C)**: Added `Skeleton` components (shimmer loading text/avatar), `LoadingState`, `ErrorState`, and `OfflineState` network connection listeners.
- **Feedback System (Layer 1-D)**: Added `Toast` (auto-dismiss alerts) and `Banner` (with info/success/warning/danger types and close triggers).
- **Core Page Shells (Layer 2-A)**: Added `DashboardShell`, `AuthPage`, and `PricingSection` components.
- **Navigation Components (Layer 2-B)**: Added `Breadcrumb`, `Dropdown`, and `CommandPalette` (with escape key listener and focus traps).
- **Data Components (Layer 3-A)**: Added `SearchBar`, `FilterBar`, `Paginator`, `DataTable` client-side search/select-all.
- **Advanced Interaction (Layer 3-B)**: Added `FileUpload`, `RichEditor`, and `ColorPicker`.
- **Content Sections (Layer 3-C)**: Added `HeroSection`, `SettingsPage`, `StatsSection`, `FAQSection`, `BlogSection`, `TestimonialsSection`, and `ContactSection`.
- **DX Tools (Layer 4)**: Integrated `amana generate` boilerplate creator, `amana dev/watch` live-reloader, and `amana check` compiler diagnostics checks.

### Fixed
- **Arabic Canvas Ligatures & RTL Flow**: Re-ordered and shaped Arabic contextual characters on HTML5 canvas contexts using jsDelivr `arabic-reshaper` and a custom Javascript string segmenter.
- **Blank Chart Element ID Collision**: Implemented thread-local `CHART_COUNTER` (compile-time) and random suffixing (runtime) to prevent duplicate canvas element IDs on a single page view.
- **Compiler Warnings Suppression**: Resolved all dead-code and unused-import warnings in `tokens.rs`, `hooks.rs`, and `static_files/` via local attributes (`#![allow(dead_code)]`) and removed the unused `HashMap` import.

## [v2.6.0] — 2026-06-20: Production-ready Modal & Grid Alignment

### Added
- **Visual Smoke Tests** — Created visual smoke testing environment using Playwright with npm scripts (`npm run test:visual`), including a self-contained visual testing application (`examples/test_modal_grid.amana`) and CI pipeline in `.github/workflows/visual.yml`.
- **Monotonic Modal IDs** — Modals generate unique monotonically increasing title IDs per view (`amana-modal-title-0`, etc.) to prevent ARIA identifier collisions.
- **Pure JavaScript Focus Trap** — Pure JS tab event handler on Modal to ensure keyboard focus is locked inside without loading external Alpine Focus plugins.

### Changed
- **Modal Component Hardening** — Improved the standard Modal component: securely escape titles utilizing EJS `<%= %>` syntax, lock page scrolling via client-side `x-effect` on `document.body.style.overflow`, and support closable backdrop/overlay click and ESC key events.
- **Grid Layout Alignment** — Content-led grids default to top-alignment (`align-items: start`) to avoid card/content stretching. Explicit stretching can be opted into using `Grid(stretch: true)`, which appends the `amana-grid-stretch` CSS utility.
- **Cleaned Up Known Issues** — Reconciled resolved layout issues in `doc/language-runtime-trust-plan.md` and `doc/AMANA_VISUAL_LANGUAGE_AUDIT.md`.

### Docs
- Updated `doc/language.md` and `doc/html-components-forms.md` to document the updated Modal API parameters.
- Updated `doc/design-anti-patterns.md` to document card grid top-alignment and dashboard stretching guidelines.
- Updated `CONTRIBUTING.md` with instructions on running visual smoke tests.

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
