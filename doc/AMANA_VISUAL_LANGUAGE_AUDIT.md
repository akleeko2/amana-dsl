# Amana Visual Language Root-Cause Audit Report (Revised)

## 1. Executive Summary

This audit report delivers a comprehensive technical analysis of the visual styling constraints, compiler behavior, and runtime limitations of the Amana DSL system. 

Historically, both AI engines and human developers working on Amana have encountered recurrent layout failures, design bugs (such as lost scrollbars, layout collapses, and horizontal viewport stretching), and structural inconsistencies. A deep inspection of the compiler pipeline, generated EJS templates, and static layout configurations reveals that **design failures in Amana are not simply developer styling errors; they are structural issues caused by compiler codegen bugs, loose semantic validators, and a lack of layout constraints in the DSL.**

---

## 2. Root Cause Diagnosis & Completed Fixes

To fix Amana's design bugs at the source, we must understand the core architectural failures in the compiler and code generator:

### A) The Compiler Codegen / EJS Mismatch (The State Wrapper Bug) - [COMPLETED]
Previously, the compiler's view code generator in [src/codegen/express/views.rs](src/codegen/express/views.rs) wrapped the EJS output in an unclassed `div` when client states existed:
```rust
ejs_body = format!("<div x-data=\"{}\">\n{}\n</div>", escaped_x_data, ejs_body);
```
Because this `div` wrapper had no class and was ignored in CSS layout styles, it defaulted to `height: auto` in block layouts. The child `.app-shell`'s `height: 100%` directive evaluated against this auto-height container, expanding the entire app shell to the full height of all tables and grids combined, disabling inner scrollbars.

**Resolution:** The compiler now wraps the EJS output in a classed `<div class="amana-state-scope" x-data="...">` container. The compiler's base CSS in [src/codegen/express/views.rs](src/codegen/express/views.rs) and [src/codegen/express/static_files/engine.rs](src/codegen/express/static_files/engine.rs) styles `.amana-state-scope` as a neutral flex layout container (`height: 100%; min-height: 0; display: flex; flex-direction: column;`) and isolates it from direct layout grids.

### B) Mobile Layout Constraints (The Squeezed Viewport Bug) - [COMPLETED]
On mobile viewports (max-width: 720px), the fixed-width desktop sidebar (`.side-rail` at `240px`) used to remain visible, squeezing the main workspace and causing layout breakage. Additionally, custom view-specific grids (like `.dashboard-grid` at `62fr 38fr` or `.settings-layout` at `220px 1fr`) did not stack, rendering content unreadable.

**Resolution:** Implemented the **Mobile DashboardShell Layout Contract** under the `@media (max-width: 720px)` media query in both compiled CSS and the preview engine. The `.side-rail` sidebar and `.settings-nav` tabs are automatically reformatted into horizontal swipeable navigation rails (`flex-wrap: nowrap; overflow-x: auto;`), custom layout columns are stacked vertically (`grid-template-columns: 1fr !important`), and padding is compacted to `1rem` to maximize usable mobile space.

### C) Interactive DSL Layout Primitives (Tabs / Accordion / collapsible sections) - [COMPLETED]
Amana now provides native semantic primitives for interactive layouts: `Tabs`, `Accordion`, and collapsible section syntax (`[collapsible: true]`). Instead of writing custom CSS classes or complex Alpine.js bindings by hand, developers use these standard compiler-supported primitives which emit clean EJS/Alpine layouts with robust defaults.

---

## 3. Visual Surface & Execution Status

This section categorizes layout behaviors and components by their current execution status:

### A) Completed Fixes
* **State Scope Wrapper:** EJS state wrapper uses `<div class="amana-state-scope" x-data="...">` with neutral flex-height styles.
* **Mobile DashboardShell Layout Contract:** Sidebar is formatted as a compact horizontal swipe nav, custom layout grids stack on mobile viewports, and page paddings are compacted.
* **Mobile Density Contract:** Completed. Compounded vertical gaps, grid padding, titles, KPI cards, and custom components are down-scaled on mobile viewports. Secondary long tables/lists (such as performance table, knowledge base list, CSAT feedback list, agent status, and urgent tickets) are constrained using `max-height` and vertical overflow scrolling. This reduced the height of `/reports` by 58% and `/dashboard` by 7.7%.
* **Horizontal settings-nav tabs:** `.settings-nav` layout switches to a horizontal swipeable tabs bar on mobile screens.
* **Timeline RTL Alignment:** Timeline items use logical positioning (`inset-inline-start`) instead of `left/right` to support RTL directions.
* **Chart Scaling:** Explicit dimensions are applied to the generated `.chart-container` to scale cleanly in dashboard panels.

### B) Remaining Verified Bugs

> **Source of truth moved:** The canonical, maintained list of active bugs now lives in [language-runtime-trust-plan.md → Known Issues](language-runtime-trust-plan.md). That section is updated whenever a bug is fixed or discovered. The summary below is kept for historical audit context; treat the trust-plan as authoritative if they ever disagree.

* **Grid Column Stretching:** Grid cards stretch to fill the tallest grid column on desktop because layout alignment defaults to `stretch`. *(severity: medium)*
* **Laptop Width Content Overflow:** Fixed desktop sidebars cause horizontal scrollbars on laptop screens (1280px) due to missing `min-width: 0` on content wrappers. *(severity: medium)*
* **Tablet Viewport Squeezing:** Lack of mid-range breakpoints (like 1024px or 900px) causes three-column layouts to squeeze excessively on tablet viewports before stacking at 720px. *(severity: medium)*
* **FormField Textarea Boundaries:** Textareas in form inputs lack height and overflow constraints, causing visual breaking in modal dialogs. *(severity: low)*
* **Modal Backdrop Blur:** Modals compile to raw `div`s and lack overlays, backdrop filters, or scroll locking. *(severity: high — blocks production-safe use of `Modal`)*

### C) Pages Remaining Long & Interactive DSL Layout Primitives - [COMPLETED]
* **Pages Remaining Long:** With tabs and collapsible sections implemented, the vertical scroll heights on mobile viewports have been significantly reduced: `/dashboard` dropped to ~1931px, `/settings` to ~1117px, and `/reports` to ~1678px.
* **Interactive DSL Layout Primitives:** Native compiler-supported interactive layout elements are fully operational:
  - **Tabs Primitive:** Renders a responsive header bar and content panels with Alpine.js switching state.
    *Syntax:*
    ```amana
    Tabs:
        tab "Recent Tickets":
            div.panel: "First tab content"
        tab "Performance Trend":
            div.panel: "Second tab content"
    ```
  - **Accordion Primitive:** Renders an expandable/collapsible list of panel items.
    *Syntax:*
    ```amana
    Accordion:
        panel "Ticket Details":
            div.info-row: "Detail 1"
        panel "Assigned Agent":
            div.info-agent: "Detail 2"
    ```
  - **Collapsible Elements:** Allows any custom or standard layout element to become collapsible by adding the bracketed attributes `[collapsible: true]`.
    *Syntax:*
    ```amana
    section.report-panel [collapsible: true, default: "open"]:
        div.report-panel-header:
            h2.report-panel-title: "Ticket Volume"
        div.volume-chart: "This body collapses"
    ```
    *Note:* The first child of the collapsible element is automatically compiled as the clickable header; all subsequent children are wrapped inside the collapsible body controlled by Alpine.js `x-show="open"`.

* **Production Boundaries & Testing:**
  - Tested in unit tests (`src/tests.rs::test_interactive_layout_primitives`) validating parser outputs, EJS codegen, and Alpine.js output markup.
  - Tested browser compatibility (zero console errors, correct viewport scale, and click triggers on both mobile/desktop).
  - Production used in:
    - `/reports` (`reports.amana`) using `Tabs` at the top and nested `[collapsible: true]` sections.
    - `/dashboard` (`dashboard.amana`) using `Tabs` to switch between "Recent Support", "Performance Trend", and "Team Status".
    - `/ticket-detail` (`ticket_detail.amana`) using `Accordion` to display ticket information details.

### D) Future Product Enhancements
* **Official `DashboardShell` Component:** Introduce a central, compiler-backed primitive layout component in the DSL render block.
* **Contextual CSS Validation:** Block `100vh` and `min-height: 100vh` custom style declarations inside `dashboard-shell` layouts during compilation, while allowing them on standalone landing pages.
* **Mobile Sidebar Hamburger Menu Transition:** Add support for a collapsible slide-out hamburger menu drawer as an alternative sidebar option.

---

## 4. Component/Pattern Matrix

| Component/Pattern | Exists in DSL | Has Safe Default CSS | Responsive-Safe | Runtime-Safe | Status | Required Fix |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`Button`** | Yes | Yes | Yes | Yes | **Already Fixed** | Transition animations hardened in `BASE_CSS_CLASSES`. |
| **`Card` / `FeatureCard`** | Yes | Yes | Yes | Yes | **Already Fixed** | Enforce default top-alignment in grid containers. |
| **`Grid`** | Yes | Yes | No | Yes | **Verified Current Bug** | Make mobile stack layout the default when no breakpoint is declared. |
| **`Section`** | Yes | Yes | Yes | Yes | **Verified Current Bug** | Clamp generated section padding in `BASE_CSS_CLASSES`. |
| **`Navbar`** | Yes | Yes | Yes | Yes | **Verified Current Bug** | Wrap generated links in flex-wrap container in `html.rs`. |
| **`Hero`** | Yes | Yes | Yes | Yes | **Verified Current Bug** | Ensure minimum width/height on `.amana-hero-media` in EJS template. |
| **`FormField`** | Yes | Yes | Yes | Yes | **Verified Current Bug** | Constrain textarea size/rows in `html.rs` output. |
| **`ResourceGrid` / `ResourceTable`** | Yes | No | Yes | Yes | **Verified Current Bug** | Enforce `max-height` and pagination constraints in layout styles. |
| **`Modal`** | Yes | No | No | Yes | **Verified Current Bug** | Rewrite EJS template to inject layout overlay. |
| **`Dashboard Shell - state wrapper`** | No (Codegen) | Yes | Yes | Yes | **Already Fixed** | EJS state wrapper uses `.amana-state-scope` class. |
| **`Dashboard Shell - mobile contract`** | No (Codegen) | Yes | Yes | Yes | **Already Fixed** | Stacks layout grids and compacts sidebar to swipe nav. |
| **`Dashboard Shell - official primitive`** | No (Canvas only) | No | Yes | Yes | **Future Product Enhancement** | Add a formal `DashboardShell` primitive layout component. |
| **`Sidebar`** | Yes | Yes | Yes | Yes | **Already Fixed** | Compiled CSS styles `.side-rail` as a horizontal swipe bar on mobile. |
| **`Panel`** | No (ad-hoc class) | No | Yes | Yes | **Future Product Enhancement** | Declare `Panel` as an official DSL component. |
| **`Timeline` / `TimelineItem`**| Yes | Yes | Yes | Yes | **Already Fixed** | Use logical positioning (`inset-inline-start`) instead of `left/right`. |
| **`Chart`** | Yes | Yes | Yes | Yes | **Already Fixed** | Set explicit dimensions on the generated `.chart-container`. |

---

## 5. CSS Freedom Risk Matrix

| Dangerous CSS Pattern | Risk Level | Visual Bug Caused | Status | Compiler Action Required |
| :--- | :--- | :--- | :--- | :--- |
| `min-height: 100vh` inside `dashboard-shell` | High | Bypasses container constraints, breaking layout height. | **Verified Current Bug** | **Block** via compiler warning. Suggest `height: 100%`. |
| `height: 100vh` inside `dashboard-shell` | High | Disables browser scrollbars, causing layout overflow. | **Verified Current Bug** | **Block** via compiler warning. Suggest `height: 100%`. |
| `min-height: 100vh` on standalone page | None | None (valid for full-screen landing or auth pages). | **Already Fixed** | **Allow** dynamically based on page canvas layout settings. |
| `overflow: hidden` on page shells | Medium | Hides content that overflows containers, preventing scroll. | **Verified Current Bug** | **Warn** when applied to outer layout shells. |
| `position: fixed` | High | Bypasses document flow, overlaying elements. | **Verified Current Bug** | **Block** on custom components. Replace with layout canvas settings. |
| `grid-template-columns` | Medium | Hardcoded pixel columns break responsiveness on mobile viewports. | **Verified Current Bug** | **Warn** when using pixel widths. Force `minmax()`. |

---

## 6. AI Authoring Failure Analysis

When models (like Gemini or Claude) write Amana views, they regularly generate broken interfaces. This failure is driven by the following compiler design gaps:

1. **Lack of Layout Constraints:** Because the DSL doesn't enforce layout blocks (like `DashboardLayout`), models must invent layouts from scratch using nested `div` elements and custom CSS classes.
2. **No CSS Semantic Validation:** The compiler parses `style:` declarations but does not validate them. If a model generates `min-height: 100vh` or nested grid configurations that break layout constraints, the compiler builds the app without warnings.
3. **No Visual Feedback Loop:** The compiler does not verify layout rendering. If a model generates overlapping panels or disables scrollbars, the build command succeeds (`"ok": true`), signaling success to the AI agent.
4. **Repetitive Examples in Codebase:** The models learn from existing examples (`apps/01-ops-horizon` and `apps/02-customer-care-hub`) which already contain ad-hoc layout fixes, perpetuating the pattern.

---

## 7. Open Decisions Resolved

1. **How should we handle mobile sidebar navigation?**
   - *Decision:* Completed. The compiler automatically styles `.side-rail` as a compact horizontal swipe navigation bar on mobile viewports. The hamburger menu is reserved purely as a future alternative design option.
2. **Should we block `height: 100vh` entirely or just show warning logs?**
   - *Decision:* Block it during compilation for views using `dashboard-shell` to prevent breaking container layouts. Show a warning for standalone landing pages.

---

## 8. Acceptance Criteria for Mobile Layouts

1. **State Wrapper Height Rule:** EJS templates must wrap client-state pages in `<div class="amana-state-scope" x-data="...">`. The compiled CSS must style `.amana-state-scope` to fill `100%` height.
2. **Horizontal swipe navigation:** Navigation links on mobile must stay in a single row with horizontal overflow enabled (`overflow-x: auto; flex-wrap: nowrap`).
3. **Mobile Layout Density:** Vertical height must be compacted on long scrollable pages (e.g. `/reports` compacted by 58%) through gap reductions and scrolling constraints on secondary lists.
