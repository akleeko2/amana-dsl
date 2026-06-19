# Design Anti-Patterns / أنماط التصميم الخاطئة

> **Bilingual reference.** This file merges the former `design-bugs-prevention.md` (English) and `design-mistakes-avoidance.md` (Arabic) into a single source of truth. Each rule below is tagged with its current status so you know whether it is a still-active risk or a solved historical bug.
>
> **المرجع الموحّد (ثنائي اللغة).** هذا الملف يدمج محتوى `design-bugs-prevention.md` (إنجليزي) و`design-mistakes-avoidance.md` (عربي) في مصدر واحد. كل قاعدة موسومة بحالتها الحالية لتعرف إن كانت خطرًا نشطًا أم خطأ تاريخيًا تم حله.

## Status legend / مفتاح الحالات

- **SOLVED / تم حله** — The compiler was changed to handle this automatically; the rule is now historical context only. The compiler يتعامل معها تلقائيًا؛ القاعدة مجرد سياق تاريخي.
- **ACTIVE RISK / خطر نشط** — The compiler does not block this; you must avoid it by discipline. الكمبايلر لا يحظرها؛ يجب تجنبها بالانضباط اليدوي.
- **ENFORCED / مُلزَم** — Semantic validation rejects this at compile time. التحقق الدلالي يرفضها وقت التجميع.

For the canonical list of component-level bugs (e.g. `Modal`, `Grid` stretch), see [Known Issues](language-runtime-trust-plan.md) in the trust plan — those are tracked there, not here.

---

## 1. Card Soup — Active Risk / خطر نشط

**EN** — Putting every element, table, and stat inside independent white rounded shadowed cards scatters the interface and hides the most important element.

**AR** — وضع كل العناصر والجداول والإحصائيات داخل بطاقات بيضاء مستقلة ذات زوايا دائرية وظلال يجعل الواجهة مشتتة.

**Fix / الحل:** Use three depth levels:
1. Base canvas (Level 1) — warm muted color (e.g. `#f7f3ea`).
2. Flat panels (Level 2) — flat white, no thick borders or heavy shadows.
3. Floating surfaces (Level 3) — only for featured cards, modals, popovers; soft shadow `0 20px 40px rgba(17,24,39,0.06)`.

Do not combine `border` and heavy `box-shadow` on the same element unless strictly necessary. لا تجمع بين الحدود والظلال على نفس العنصر إلا للضرورة.

---

## 2. Missing Mobile Responsive Rules — Active Risk / خطر نشط

**EN** — Fixed-column grids or sticky sidebars without declared stacking behavior produce horizontally squeezed, thin vertical strips on small screens.

**AR** — استخدام شبكات ذات أعمدة ثابتة أو أشرطة جانبية sticky دون تحديد سلوك التكديس يجعل الواجهة مضغوطة أفقياً.

**Fix / الحل:** Always declare mobile behavior:
```amana
Grid(columns: "3"):
    responsive:
        mobile.columns: 1
```
Or `responsive: mobile: stacked`. For custom mobile grids use auto-fit:
```css
grid-template-columns: repeat(auto-fit, minmax(min(100%, 18rem), 1fr))
```

> **Note:** The Mobile DashboardShell Contract (`.side-rail`, `.settings-nav` → horizontal swipe rail; grids → `1fr`) is **SOLVED** for the built-in shell, but custom grids you write still need explicit responsive rules.

> ملاحظة: عقد Mobile DashboardShell (تحويل `.side-rail` و `.settings-nav` لشريط أفقي؛ تكديس الشبكات) **تم حله** للقوالب المدمجة، لكن الشبكات المخصصة تحتاج قواعد responsive صريحة.

---

## 3. Alpine JS Variables Inside EJS Tags — Enforced / مُلزَم

**EN** — Embedding client state (Alpine) variables inside server-side EJS tags (`<%= %>`) to control visibility crashes the app with `ReferenceError`.

```html
<!-- FATAL - crashes the app -->
<div class="amana-modal" x-show="<%= task_modal %>">
```

**AR** — إدراج متغيرات العميل داخل وسم الخادم EJS يوقف التطبيق بخطأ `ReferenceError`.

**Fix / الحل:** Let Alpine control the property directly without EJS:
```html
<!-- Correct -->
<div class="amana-modal" x-show="task_modal">
```

---

## 4. Missing Resource Lifecycle States — Active Risk / خطر نشط

**EN** — Rendering server-fetched data directly without `loading`, `error`, and `empty` blocks yields a poor UX on slow or failing connections. `inspect-design` warns about this.

**AR** — عرض البيانات دون كتل `loading`/`error`/`empty` يؤدي لتجربة سيئة عند بطء الاتصال.

**Fix / الحل:**
```amana
ResourceGrid(projects):
    item ProjectCard(project)
    loading:
        p: "Loading..."
    error:
        Alert(tone: "warning", message: "Could not load projects")
    empty:
        EmptyState(title: "No projects")
```

---

## 5. Standard Library Calls Inside Render — Enforced / مُلزَم

**EN/AR** — Calling server-only functions (e.g. `time.now()`) directly inside a `render:` block fails type checking.

```amana
# Wrong / خطأ
render:
    p: time.now()
```

**Fix / الحل:**
```amana
server:
    fetch current_time = time.now()
render:
    p: current_time
```

Standard libraries require app capabilities and are only reachable through server fetches.

---

## 6. Unprotected Current-User Writes — Enforced / مُلزَم

**EN/AR** — Using `default owner_id = User.current.id` (or `<auth_model>.current`) in an unprotected view fails semantic validation.

**Fix / الحل:** Make the view protected first:
```amana
view Dashboard:
    protected:
        allow: User.current != null
        deny: -> /login
        unauthenticated: -> /login
    render:
        form [title]:
            connect Task.create
            default owner_id = User.current.id
```

---

## 7. FormField Textarea Misuse — Solvable / قابل للحل

**EN/AR** — Using plain tags/text for large inputs breaks the page structure.

**Fix / الحل:** Use the dedicated `textarea` type:
```amana
field message:
    label: "Details"
    type: textarea
```
This renders `<textarea class="amana-form-control" rows="4">`.

> **Active Known Issue:** Textareas lack height/overflow constraints inside modals. See [Known Issues](language-runtime-trust-plan.md) for the `FormField textarea` bug and its workaround.

---

## 8. Card Stretching And CSS Spacing — Partially Solved / تم حله جزئيًا

**AR** —
* تمدد البطاقات لملء الحاوية رأسيًا بسبب `min-height: 100%` افتراضي.
* كتابة `calc(100vh-2.5rem)` بدون مسافات يتجاهلها المتصفح.

**What is SOLVED / ما تم حله:**
- Card default height is now `auto` (prevents unwanted stretching).
- The compiler auto-formats binary operators (`+`, `-`, `*`, `/`) inside `calc`, `min`, `max`, `clamp` with correct spacing.

**What is still an ACTIVE RISK / ما زال خطرًا نشطًا:**
- Grid columns still stretch to the tallest column on desktop (Known Issue). Workaround:
```css
.amana-grid:has(> .dashboard-main-col):
    align-items: start
```

---

## 9. EJS JSON Stringification Inside Client Scripts — Solved / تم حله

**EN/AR** — Passing raw EJS output directly into client script variables caused editor syntax errors.

```html
<!-- INCORRECT - editor syntax error -->
<script>
  const data = <%- JSON.stringify(metrics) %>;
</script>
```

**Fix (compiler now does this) / الحل (المترجم يقوم بذلك الآن):** Always wrap in quotes and parse on the client:
```javascript
const data = JSON.parse('<%- JSON.stringify(metrics) %>');
```

---

## 10. EJS Tags Inside Static Style Attributes — Solved / تم حله

**EN/AR** — The compiler previously wrapped static style values like `style: "width: 77%"` with dynamic EJS tags, confusing the CSS parser.

**Status:** The generator now emits static properties as plain HTML attributes:
```html
<!-- CORRECT generated HTML -->
<div style="width: 77%"></div>
```

---

## 11. Dynamic Style Linter Conflict — Solved / تم حله

**EN/AR** — Dynamic EJS inside `style="..."` triggered CSS linter errors.

**Status:** The compiler now binds dynamic styles via Alpine `:style` (which the CSS linter ignores):
```html
<!-- CORRECT generated output -->
<div :style="`<%= `width: \${member.capacity}%` %>`"></div>
```

---

## 12. `!important` Keyword Conflict — Solved / تم حله

**EN/AR** — The compiler previously translated `!important` into `not important`, breaking priority.

**Status:** The compiler now handles `TokenKind::Not` inside style property values and emits it back as `!`, so `!important` renders correctly.

---

## 13. Always Add Page-Level Canvas — Active Risk / خطر نشط

**EN** — Views should declare page-level intent; `inspect-design` warns when missing.

**Fix:**
```amana
view Home:
    canvas:
        layout: column
        density: comfortable
        responsive:
            mobile: stacked
```

---

## 14. Use Design Blocks For Intent — Active Risk / خطر نشط

Prefer explicit design blocks inside render elements; `inspect-design` warns when `creative`, `responsive`, `brand`, or `art` are missing.

```amana
section.hero:
    compose:
        layout: bento
        gap: lg
    visual:
        surface: glass
        gradient: hero
    creative:
        uniqueness: signature
```

---

## 15. Avoid Layout Repetition — Active Risk / خطر نشط

Repeated `compose.layout: column` everywhere reduces design quality and triggers audit warnings. Use variety: `bento`, `split`, `masonry`, `magazine`, `dashboard-shell`.

---

## 16. Use Token Values For Spacing — Active Risk / خطر نشط

**Prefer:**
```amana
style:
    .card:
        padding: lg
        gap: md
```
**Avoid** arbitrary values like `padding: 13px` unless needed. The audit flags non-standard static spacing.

---

## 17. Keep CSS Inside The Safe DSL — Enforced / مُلزَم

```amana
style:
    .panel:
        surface: elevated
        transition: fade 250ms ease
```
Rejected:
```amana
style:
    body:
        background: red
```
Unsafe selectors (`body`, `html`, `*`), event attributes (`[onclick]`), and unapproved URL patterns (`url(https://...)`) are rejected. See [css-theme-rtl.md](css-theme-rtl.md) for the full allowlist.

---

## 18. Avoid Raw Dangerous HTML — Enforced / مُلزَم

Blocked tags:
```text
script iframe object embed applet link meta base style noscript
```
Use client state, event attributes, and `style:` blocks instead.

---

## 19. Add Accessible Labels — Active Risk / خطر نشط

```amana
Button(label: "Save")
FormField(name: "email", label: "Email", type: "email")
```
`inspect-design` warns when buttons or form fields lack accessible text.

---

## 20. Protect Current User Writes — Enforced / مُلزَم

See rule #6.

---

## 21. Keep Standard Libraries Out Of Render — Enforced / مُلزَم

See rule #5.

---

## 22. Use Layout Primitives To Avoid Giant Mobile Pages — Active Risk / خطر نشط

To avoid extremely long mobile scroll heights, use the interactive primitives (implemented and production-safe):
1. **`Tabs:`** — group unrelated dashboards/deep columns; mobile users see one tab at a time.
2. **`Accordion:`** — wrap sidebar properties and info panels; users expand only what they need.
3. **`[collapsible: true]`** — mark large content blocks (charts, status grids); the first child becomes the toggle header.

See [html-components-forms.md → Interactive DSL Layout Primitives](html-components-forms.md#interactive-dsl-layout-primitives) for the exact compiled output.

---

## Maintenance Note

When the compiler fixes a rule marked **Active Risk**, move it to **Solved** in the same commit and add a regression test. When a new anti-pattern is discovered, add it here tagged **Active Risk** and add an `inspect-design` warning if feasible.
