Phase A — Generated output correctness
- EJS compile test
- CSS injection داخل style/file
- node --check app.js + runtime/engine.js
- HTTP 200 smoke test لكل route

Phase B — CSS/runtime correctness
- missing CSS variables
- radius scale
- hover pseudo rules
- FormField required
- Iconify fix

Phase C — security hardening
- allowed tags
- allowed attributes
- safe href/src protocols
- unknown settings diagnostics

Phase D — design system extraction
- amana-tokens.css
- amana-system.css
- page-specific CSS
- login/todos بدون Bootstrap

Phase E — language expansion
- Navbar links
- redirect error
- List/Json
- Money cents
- custom components