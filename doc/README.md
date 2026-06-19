# Amana Documentation

This directory is the working reference for the Amana compiler and generated runtime. It is intentionally based on implementation files under `src/`, not on old examples or tests.

> **New here?** Start with [getting-started.md](getting-started.md) — the end-user onboarding guide. The rest of this directory is the implementation reference.
>
> **جديد هنا؟** ابدأ بـ [getting-started.md](getting-started.md) — دليل البداية للمستخدم.

## Source Of Truth

- Core grammar: `src/lexer`, `src/parser`, and `src/ast`.
- Validation rules: `src/semantic`.
- Generated output: `src/codegen`.
- CLI, imports, LSP, formatter, and design inspection: `src/main.rs` and `src/formatter.rs`.
- Generated language inventory: [language-inventory.generated.md](language-inventory.generated.md).
- Active bug list: [Known Issues](language-runtime-trust-plan.md) in the trust plan.
- Release history: [../CHANGELOG.md](../CHANGELOG.md).

When language behavior changes, update docs by running:

```powershell
python scripts/language_inventory.py --write
scripts/search-language.ps1 -Area all
```

## Reading Order

**For app authors / لكاتبي التطبيقات:**

1. [getting-started.md](getting-started.md) — install, first app, auth + data. التنصيب والتطبيق الأول.

**For compiler contributors / لمساهمي المترجم:**

1. [language.md](language.md) — complete language reference and current implementation status.
2. [language-inventory.generated.md](language-inventory.generated.md) — generated inventory of tokens, parser blocks, semantic keys, CSS tokens, components, and runtime outputs.
3. [language-runtime-trust-plan.md](language-runtime-trust-plan.md) — language/runtime contract matrix, implemented trust fixes, **Known Issues**, and remaining governance gates.
4. [cli-dx.md](cli-dx.md) — CLI commands, JSON diagnostics, formatter, IR snapshots, LSP, documentation scripts, and the `amana dev` live-rebuild workflow.
5. [html-components-forms.md](html-components-forms.md) — render tree, built-in components, Alpine bindings, forms, resource blocks.
6. [css-theme-rtl.md](css-theme-rtl.md) — theme keys, design grammar, scoped CSS DSL, RTL behavior.
7. [node-runtime-security.md](node-runtime-security.md) — Express runtime, sessions, CSRF, REST API, seed policy, hooks.
8. [multi-file-lsp.md](multi-file-lsp.md) — imports and editor/LSP behavior.
9. [design-anti-patterns.md](design-anti-patterns.md) — bilingual design anti-patterns; each rule tagged `SOLVED` / `ACTIVE RISK` / `ENFORCED`. (Replaces the former `design-bugs-prevention.md` and `design-mistakes-avoidance.md`.)
10. [AMANA_VISUAL_LANGUAGE_AUDIT.md](AMANA_VISUAL_LANGUAGE_AUDIT.md) — historical root-cause audit of visual/layout bugs; its active findings are mirrored into the trust-plan Known Issues.
11. [examples-gallery.md](examples-gallery.md) — verified example strategy. Examples are supporting artifacts, not the documentation source of truth.
12. [roadmap.md](roadmap.md) — current status and next implementation work.

## Verification

```powershell
cargo test
python scripts/language_inventory.py --check
cargo run -- check <entry-file.amana> --json
```

Use tests for regression verification only. Do not use test snippets as documentation truth unless the corresponding behavior is confirmed in compiler implementation files.

## Documentation Conventions

- **Status words are binding.** A language feature is one of: `implemented`, `experimental`, `internal`, or `rejected`. There is no public `partial` status (see [language-runtime-trust-plan.md](language-runtime-trust-plan.md)).
- **Known Issues are authoritative.** If a component has an active entry in the trust-plan Known Issues, it cannot simultaneously be described as fully `implemented` elsewhere — point to the Known Issue instead.
- **Two languages, one truth.** Reference prose may be English or Arabic, but the same facts must appear in both where both are given. Do not let one language drift.
- **Generated files stay generated.** Never hand-edit `language-inventory.generated.md`; regenerate it.
