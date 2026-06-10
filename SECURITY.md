# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| `main` branch | ✅ Yes |
| Tagged releases | ✅ Yes (latest) |

## Reporting a Vulnerability

**Please do NOT open a public GitHub issue for security vulnerabilities.**

Instead, report security issues privately by emailing the maintainer directly. Include:

1. **Description** of the vulnerability and its potential impact.
2. **Steps to reproduce** — a minimal `.amana` file or command sequence that triggers the issue.
3. **Affected components** — parser, semantic analyzer, CSS sanitizer, generated runtime, etc.
4. **Suggested fix** (optional but appreciated).

You will receive an acknowledgement within **48 hours** and a full response within **7 days**.

---

## Security Design of Amana

Amana has a multi-layer security model built into the compiler itself:

### 1. CSS 4-Layer Sanitizer (Compile-Time)

The compiler enforces strict CSS safety before code generation:

- **Selector Safety**: Globally blocked selectors (`body`, `html`, `*`, `script`, `[onclick]`, etc.) are rejected with a `Security` compiler error.
- **Property Allowlist**: Only modern, safe layout and paint CSS properties are allowed through.
- **Value Sanitizer**: Dangerous patterns (`javascript:`, `expression(`, `url(data:...)`, `vbscript:`) are stripped.
- **Layer Isolation**: All output CSS is scoped into `@layer components, variants, overrides` to prevent cascade attacks.

### 2. HTML Tag Blocklist (Compile-Time)

Raw HTML tags that present XSS or injection risks (`<script>`, `<iframe>`, `<style>`, `<link>`, `<meta>`, `<base>`, `<object>`, `<embed>`, `<applet>`, `<noscript>`) are **rejected by the compiler** before any output is generated.

### 3. SQL Injection Protection (Runtime)

The generated Express server uses **parameterized queries exclusively** (via `better-sqlite3`). No raw string interpolation is used in any generated database handler.

### 4. Form Validation (Runtime)

All generated form submission routes include server-side validation derived from the model's field constraints (`required`, `min`, `max`, `email`, `unique`). Client-side validation is supplementary only.

---

## Scope

| In Scope | Out of Scope |
|---|---|
| Compiler crashes / panics on malicious input | Vulnerabilities in user's own `.amana` app logic |
| CSS sanitizer bypasses | End-user network configuration issues |
| Generated code with SQL injection risk | Vulnerabilities in `node_modules` dependencies |
| XSS via EJS template output | Issues in unrelated Rust crates |
| Parser denial-of-service (infinite loops) | |

Thank you for helping keep Amana safe! 🛡️
