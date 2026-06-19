# Roadmap And Implementation Status

This roadmap reflects current implementation state from `src/`. It avoids claims based on old docs or old examples.

For the product and compiler governance plan that turns non-production/internal items into a two-phase execution program, see [language-runtime-trust-plan.md](language-runtime-trust-plan.md). **Active, documented bugs live in the [Known Issues](language-runtime-trust-plan.md) section of that file — they are NOT listed here as fully implemented.** Items listed as implemented below are core language/runtime surfaces; visual component polish gaps are tracked separately in the Known Issues table.

## Implemented

| Area | Status |
| --- | --- |
| Indentation-sensitive lexer | Implemented. Spaces only; tabs error. |
| Top-level parser | Implements `app`, `theme`, `model`, `seed`, `route`, `view`, `component`, `variant`, `tokens`. |
| Import graph | Implemented in `src/main.rs`; supports relative imports, deduplication, and cycle detection. |
| Semantic analyzer | Implements scope/type checks, theme/design validation, seed validation, form validation, standard library capability checks. |
| IR generation | Implemented for app, models, theme, routes, views, fetches, guards, form actions, seeds. |
| AST optimizer | Constant folding and model dead-code elimination. |
| Express backend | Current default backend. |
| SQLite schema generation | Implemented with quoted identifiers and constraints. |
| Safe query generation | Implemented for `all`, `find`, `filter`, `count`. |
| EJS view generation | Implemented. |
| Standard components | Implemented in EJS codegen for the inventory-listed components. |
| Forms runtime | Implemented for create/update/delete/login/register/logout. |
| CSRF/session/security middleware | Implemented in generated runtime. |
| REST API generation | Implemented behind `api.rest`. |
| `tokens:` IR/CSS emission | Implemented. Top-level token blocks are preserved in IR and emitted as CSS variables. |
| `permit` runtime enforcement | Implemented in generated Express runtime for REST, form mutations, and server fetch filtering; covered by multi-session REST/form integration tests for positive and negative authorization paths. |
| `Chart(...)` parser syntax | Implemented for `Chart(data, type, x, y)` identifier arguments. |
| Ternary parser support | Implemented for `cond ? then : else`. |
| Client state persistence | Implemented for `memory`, `local`, `session`, and `cookie`. |
| Resource lifecycle controls | Implemented for server-fetched rows with loading/error/empty, filters, and sort in generated EJS. |
| Component variants | Implemented. Global and component-local variants are preserved in IR and emitted as target-specific CSS for base, hover, slots, and responsive rules. |
| Formatter | Implemented. |
| JSON diagnostics | Implemented for CLI success/error paths. |
| IR snapshots | Implemented for write/verify. |
| LSP stdio server | Implemented with diagnostics, formatting, static plus semantic completion, hover, and go-to-definition over the resolved import graph. |
| Design inspection | Implemented with structured warnings and score. |
| Documentation inventory script | Implemented in `scripts/language_inventory.py`. |
| State Scope Wrapper | Implemented. Emits classed flex-height `<div class="amana-state-scope" x-data="...">` wrappers to prevent container collapse. |
| Mobile DashboardShell Contract | Implemented. Formats mobile sidebars as swipeable nav elements and stacks layout grids. |
| Mobile Density Contract | Implemented. Compacts spacing, gaps, and pads secondary long tables via max-height auto-scroll in mobile layouts. |
| Interactive DSL Layout Primitives | Implemented. Native compiler-supported `Tabs`, `Accordion`, and collapsible elements (`[collapsible: true]`). |

## Remaining Work

| Area | Current state | Needed work |
| --- | --- | --- |
| Configurable auth model current-user mapping | Generalized in semantic/codegen/runtime for the main generated Express path and covered by non-`User` `auth_model` runtime integration tests. | Extend coverage if new auth actions or alternate principal fields are added. |
| LSP | Diagnostics, formatting, semantic completion, hover, and go-to-definition are implemented. | Add references and rename. |
| `Modal` component | Compiles to raw `div` without overlay/backdrop/scroll-lock/focus-trap. | Rewrite EJS template to inject a real modal overlay (tracked as **high severity** Known Issue). |
| `Grid` vertical alignment | Cards stretch to tallest column on desktop. | Make `align-items: start` the default in grid containers. |
| Mid-range breakpoints | Three-column layouts squeeze on tablets (1024px) before stacking at 720px. | Add tablet breakpoint to the responsive contract. |
| Laptop content overflow | Fixed sidebars cause horizontal scrollbar at 1280px. | Add `min-width: 0` to content wrappers. |
| `FormField` textarea in modals | No height/overflow constraints. | Constrain generated textarea size. |

## Near-Term Priorities

1. Fix the **high-severity** `Modal` overlay/backdrop bug (the only Known Issue blocking a component from production-safe use).
2. Add references and rename support to the LSP.
3. Extend semantic completion beyond current-principal fields to arbitrary model fields where the syntax context identifies the owning model.
4. Add certified examples that run in CI and do not become the source of truth.
5. Keep `doc/language-inventory.generated.md` current in CI.
6. Add compiler warnings for dangerous CSS patterns (`100vh` in `dashboard-shell`, pixel `grid-template-columns`) — currently the compiler silently accepts them.

## Documentation Maintenance Rule

Every language change should include:

```powershell
python scripts/language_inventory.py --write
python scripts/language_inventory.py --check
scripts/search-language.ps1 -Area all
```

Then update the relevant hand-written document.
