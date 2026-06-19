// src/codegen/express/views.rs
use crate::ast::{DesignBlock, PersistMode, StyleRule, VariantDecl};
use crate::semantic::ir::{AmanaIR, ViewIR};

pub(crate) fn compile_view_ejs(
    view: &ViewIR,
    html_lang: &str,
    html_dir: &str,
    _bootstrap_css: &str,
    theme_css_block: &str,
    _ir: &AmanaIR,
) -> String {
    let mut ejs_body = if let Some(body) = &view.render_body {
        crate::codegen::html::generate_ejs_with_auth_model(
            body,
            &view.client_states,
            &_ir.app.auth_model,
        )
    } else {
        String::new()
    };
    if !view.client_states.is_empty() {
        let x_data_str = alpine_state_data(view, &_ir.app.auth_model);
        let escaped_x_data = x_data_str
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        ejs_body = format!(
            "<div class=\"amana-state-scope\" x-data=\"{}\">\n{}\n</div>",
            escaped_x_data, ejs_body
        );
    }

    let body_attrs = canvas_attributes(view.canvas.as_ref());
    let page_styles = view.styles.as_deref().unwrap_or("");
    let variant_css_block = variant_css(&_ir.variants);

    format!(
        r#"<!DOCTYPE html>
<html lang="{}" dir="{}">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title><%= typeof title !== 'undefined' ? title : 'Amana Application' %></title>
  <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
  <script defer src="https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js"></script>
  <script defer src="https://code.iconify.design/iconify-icon/2.1.0/iconify-icon.min.js"></script>
  <script>
    window.amanaResource = function() {{
      return {{
        rows: [],
        filters: {{}},
        sortField: '',
        sortDirection: 'asc',
        loading: true,
        error: null,
        init() {{
          try {{
            const payload = this.$root.querySelector('[data-amana-resource-rows]');
            this.rows = payload ? JSON.parse(payload.textContent || '[]') : [];
          }} catch (err) {{
            this.error = 'Resource data could not be loaded.';
            this.rows = [];
          }} finally {{
            this.loading = false;
          }}
        }},
        rowMatches(row) {{
          return Object.entries(this.filters).every(([field, value]) => {{
            if (!value) return true;
            return String(row?.[field] ?? '').toLowerCase().includes(String(value).toLowerCase());
          }});
        }},
        rowAt(index) {{
          const parsed = Number(index);
          return Number.isInteger(parsed) && parsed >= 0 ? this.rows[parsed] : undefined;
        }},
        rowMatchesAt(index) {{
          const row = this.rowAt(index);
          return row ? this.rowMatches(row) : false;
        }},
        sortedRows() {{
          const rows = this.rows.filter(row => this.rowMatches(row));
          if (!this.sortField) return rows;
          const dir = this.sortDirection === 'desc' ? -1 : 1;
          return rows.slice().sort((a, b) => String(a?.[this.sortField] ?? '').localeCompare(String(b?.[this.sortField] ?? ''), undefined, {{ numeric: true }}) * dir);
        }},
        rowOrder(row) {{
          const key = row && row.id !== undefined ? `id:${{row.id}}` : JSON.stringify(row);
          const index = this.sortedRows().findIndex(item => {{
            const itemKey = item && item.id !== undefined ? `id:${{item.id}}` : JSON.stringify(item);
            return itemKey === key;
          }});
          return index < 0 ? 0 : index;
        }},
        rowOrderAt(index) {{
          const row = this.rowAt(index);
          return row ? this.rowOrder(row) : 0;
        }},
        visibleCount() {{
          return this.rows.filter(row => this.rowMatches(row)).length;
        }}
      }};
    }};
  </script>
  <style>
    {}
    {}
    {}
    {}
    :where(.amana-runtime-shell, .amana-page, .page) {{ width: 100%; max-width: 100%; overflow-x: hidden; }}
    :where(.amana-runtime-shell, .amana-page, .page) :where(section, header, main, footer, div, article, aside, form) {{ min-width: 0; }}
    :where(.amana-runtime-shell, .amana-page, .page) :where(h1, h2, h3, p, a, button, span, strong, label, input, textarea, pre) {{ max-width: 100%; overflow-wrap: break-word; }}
    :where(.dg-layout-split-diagonal, .dg-layout-asymmetric, .dg-layout-editorial, .dg-layout-dashboard-shell, .dg-layout-magazine, .dg-layout-bento, .dg-layout-command-center, .dg-layout-showcase-rail) > .amana-container {{ grid-column: 1 / -1; }}
    @media (max-width: 1200px) {{
      :where(.hero-title, .section-title, .auth-card h1, .cta-box h2, .amana-hero h1, h1) {{ font-size: clamp(2.15rem, 8vw, 4.4rem) !important; line-height: 1.08 !important; max-width: 100% !important; }}
      :where(.hero-lead, .section-lead, .amana-hero-copy, p) {{ font-size: clamp(1rem, 2.4vw, 1.2rem); }}
      :where(.hero-shell, .section-space, .workflow, .pricing-section, .testimonials, .cta-section, .auth-shell) {{ padding-inline: clamp(1rem, 4vw, 2rem) !important; }}
      :where(.hero-grid, .split, .workflow-grid, .pricing-grid, .testimonial-grid, .cta-box, .amana-split, .dg-layout-split-diagonal, .dg-layout-asymmetric, .dg-layout-editorial, .dg-layout-dashboard-shell, :where(.dg-layout-dashboard-shell) .amana-runtime-shell > :not(script):not(style):not(.amana-state-scope), :where(.dg-layout-dashboard-shell) .amana-runtime-shell > .amana-state-scope > :not(script):not(style), .dg-layout-command-center, .dg-layout-showcase-rail):not([style*="--bp-laptop-columns"]):not([style*="--bp-tablet-columns"]):not([style*="--bp-mobile-columns"]) {{ grid-template-columns: minmax(0, 1fr) !important; }}
    }}
    @media (max-width: 720px) {{
      :where(.hero-title, .section-title, .auth-card h1, .cta-box h2, .amana-hero h1, h1) {{ font-size: clamp(2rem, 11vw, 3.4rem) !important; }}
      :where(.hero-panel, .workflow-box, .visual-card, .price-card, .testimonial-card, .cta-box, .auth-card, .contact-card, .amana-card) {{ padding: clamp(1rem, 5vw, 1.5rem) !important; border-radius: min(var(--radius-2xl), 22px) !important; }}
      :where(.hero-actions, .trust-strip, .badge-row, .token-list, .logo-row, .amana-hero-actions, .amana-cluster) {{ align-items: stretch; }}
      :where(.amana-btn, .plan-button, button) {{ white-space: normal; text-align: center; }}
      :where(.dg-layout-dashboard-shell) {{
        height: auto !important;
        min-height: 100vh !important;
        overflow: auto !important;
      }}
      :where(.dg-layout-dashboard-shell) .amana-runtime-shell {{
        height: auto !important;
        min-height: 100vh !important;
        overflow: auto !important;
      }}
      :where(.dg-layout-dashboard-shell) .amana-state-scope {{
        height: auto !important;
        min-height: 100vh !important;
        overflow: visible !important;
      }}
      .app-shell {{
        display: flex !important;
        flex-direction: column !important;
        height: auto !important;
        min-height: 100vh !important;
        overflow: visible !important;
        padding: 0.5rem !important;
        gap: 1rem !important;
        width: 100% !important;
        max-width: 100% !important;
      }}
      .side-rail {{
        position: static !important;
        width: 100% !important;
        max-width: 100% !important;
        height: auto !important;
        min-height: 0 !important;
        flex-direction: row !important;
        flex-wrap: nowrap !important;
        align-items: center !important;
        justify-content: space-between !important;
        padding: 0.75rem 1rem !important;
        gap: 0.75rem !important;
        border-right: none !important;
        border-bottom: 1px solid rgba(255, 255, 255, 0.08) !important;
      }}
      .side-rail > [class*="brand"],
      .side-brand {{
        width: auto !important;
        border-bottom: none !important;
        padding: 0 !important;
        flex-shrink: 0 !important;
      }}
      .side-rail > [class*="nav"],
      .side-nav {{
        display: flex !important;
        flex-direction: row !important;
        flex-wrap: nowrap !important;
        overflow-x: auto !important;
        -webkit-overflow-scrolling: touch;
        scrollbar-width: none !important;
        gap: 0.5rem !important;
        width: auto !important;
        padding: 0 !important;
        flex: 1 !important;
        justify-content: flex-end !important;
      }}
      .side-rail > [class*="nav"]::-webkit-scrollbar,
      .side-nav::-webkit-scrollbar {{
        display: none !important;
      }}
      .side-rail > [class*="nav"] a,
      .side-rail a[class*="link"],
      .side-nav a {{
        flex: 0 0 auto !important;
        font-size: 0.8rem !important;
        padding: 0.35rem 0.65rem !important;
        border-radius: 8px !important;
        white-space: nowrap !important;
      }}
      .side-rail > [class*="footer"],
      .side-footer {{
        display: none !important;
      }}
      .dashboard-main {{
        flex: 1 1 auto !important;
        height: auto !important;
        min-height: 0 !important;
        overflow: visible !important;
        width: 100% !important;
        max-width: 100% !important;
      }}
      .workspace {{
        padding: 0 !important;
      }}
      :where(.dg-layout-dashboard-shell) .dashboard-main > :not(script):not(style) {{
        padding-left: 1rem !important;
        padding-right: 1rem !important;
      }}
      .dash-header {{
        padding-left: 1rem !important;
        padding-right: 1rem !important;
        flex-wrap: wrap !important;
        gap: 1rem !important;
      }}
      .dashboard-grid,
      .settings-layout,
      .ticket-detail-grid,
      .reports-grid,
      .reports-secondary {{
        grid-template-columns: 1fr !important;
        padding-left: 1rem !important;
        padding-right: 1rem !important;
        gap: 1.25rem !important;
      }}
      .reports-kpis {{
        grid-template-columns: repeat(2, 1fr) !important;
        gap: 0.75rem !important;
      }}
      .settings-nav {{
        position: static !important;
        flex-direction: row !important;
        overflow-x: auto !important;
        flex-wrap: nowrap !important;
        scrollbar-width: none !important;
        width: 100% !important;
        padding: 0.25rem !important;
      }}
      .settings-nav::-webkit-scrollbar {{
        display: none !important;
      }}
      .settings-nav-item {{
        white-space: nowrap !important;
      }}
      .inbox-split-pane {{
        grid-template-columns: 1fr !important;
        min-height: auto !important;
      }}
      .inbox-list-pane {{
        border-right: none !important;
        padding-right: 0 !important;
        border-bottom: 1px solid rgba(17, 24, 39, 0.06) !important;
        padding-bottom: 1.25rem !important;
      }}
      .inbox-detail-pane {{
        padding-left: 0 !important;
      }}
      .table-row {{
        grid-template-columns: 1fr !important;
        gap: 0.5rem !important;
        align-items: flex-start !important;
      }}
      .dg-rsp-mobile-stacked, .dg-responsive-mobile-stacked {{
        grid-template-columns: minmax(0, 1fr) !important;
      }}
      .dg-rsp-mobile-stacked > *, .dg-responsive-mobile-stacked > * {{
        grid-column: span 1 / auto !important;
      }}
      /* Mobile Content Density & Section Compaction */
      :where(.dg-layout-dashboard-shell) .reports-container {{
        padding: 1.05rem !important;
        gap: 1rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .dashboard-grid,
      :where(.dg-layout-dashboard-shell) .settings-layout,
      :where(.dg-layout-dashboard-shell) .ticket-detail-grid,
      :where(.dg-layout-dashboard-shell) .reports-grid,
      :where(.dg-layout-dashboard-shell) .reports-secondary {{
        gap: 0.85rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .dashboard-main-col,
      :where(.dg-layout-dashboard-shell) .dashboard-side-col {{
        gap: 0.85rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .panel {{
        padding: 1rem !important;
        border-radius: 12px !important;
      }}
      :where(.dg-layout-dashboard-shell) .panel-header {{
        margin-bottom: 0.85rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .kpi-row {{
        padding: 1rem !important;
        gap: 0.75rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .dash-header {{
        padding: 1rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .amana-kpi {{
        padding: 0.85rem !important;
        border-radius: 10px !important;
        gap: 0.25rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .amana-kpi-value {{
        font-size: 1.8rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .kpi-wide {{
        padding: 0.75rem 0.85rem !important;
        border-radius: 10px !important;
      }}
      :where(.dg-layout-dashboard-shell) .kpi-wide-value {{
        font-size: 1.35rem !important;
        margin-bottom: 0.15rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .kpi-wide-label {{
        font-size: 0.65rem !important;
        margin-bottom: 0.2rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .performance-table {{
        max-height: 280px !important;
        overflow-y: auto !important;
      }}
      :where(.dg-layout-dashboard-shell) .kb-list {{
        max-height: 280px !important;
        overflow-y: auto !important;
      }}
      :where(.dg-layout-dashboard-shell) .csat-list {{
        max-height: 280px !important;
        overflow-y: auto !important;
      }}
      :where(.dg-layout-dashboard-shell) .agent-status-list {{
        max-height: 240px !important;
        overflow-y: auto !important;
      }}
      :where(.dg-layout-dashboard-shell) .urgent-list {{
        max-height: 240px !important;
        overflow-y: auto !important;
      }}
      :where(.dg-layout-dashboard-shell) .agent-status-row {{
        padding: 0.4rem 0.5rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .kb-row {{
        padding: 0.5rem 0.25rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .perf-row {{
        padding: 0.4rem 0.25rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .csat-row {{
        padding: 0.6rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .amana-resource-item {{
        padding: 0.65rem !important;
        gap: 0.4rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .amana-resource-list {{
        gap: 0.5rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .report-panel-header {{
        margin-bottom: 0.85rem !important;
        padding-bottom: 0.6rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .report-panel-title {{
        font-size: 0.88rem !important;
      }}
      :where(.dg-layout-dashboard-shell) .volume-chart {{
        height: 130px !important;
      }}
      :where(.dg-layout-dashboard-shell) .chart-bars-large {{
        height: 110px !important;
      }}
      :where(.dg-layout-dashboard-shell) .bar-wrap-lg {{
        height: 80px !important;
      }}
      :where(.dg-layout-dashboard-shell) .chart-wrap {{
        min-height: 140px !important;
      }}
      :where(.dg-layout-dashboard-shell) .chart-bars {{
        height: 100px !important;
      }}
      :where(.dg-layout-dashboard-shell) .bar-wrap {{
        height: 70px !important;
      }}
    }}
  </style>
  <%- typeof styles !== 'undefined' && styles ? '<style>' + styles + '</style>' : '' %>
</head>
<body{}>
  <main class="amana-runtime-shell">
  {}
  </main>
</body>
</html>"#,
        html_lang,
        html_dir,
        theme_css_block,
        BASE_CSS_CLASSES,
        variant_css_block,
        page_styles,
        body_attrs,
        ejs_body
    )
}

fn target_base_selector(target: &str) -> String {
    let token = design_token(target);
    match target {
        "Button" => ".amana-btn".to_string(),
        "Card" => ".amana-card".to_string(),
        "FeatureCard" => ".amana-card.amana-feature-card".to_string(),
        "PricingCard" => ".amana-card.amana-pricing-card".to_string(),
        "Container" => ".amana-container".to_string(),
        "Section" => ".amana-section".to_string(),
        "Grid" => ".amana-grid".to_string(),
        "Stack" => ".amana-stack".to_string(),
        "Navbar" => ".amana-navbar".to_string(),
        "Hero" => ".amana-hero".to_string(),
        "FormField" => ".amana-field".to_string(),
        "Alert" => ".amana-alert".to_string(),
        "Footer" => ".amana-footer".to_string(),
        "Modal" => ".amana-modal".to_string(),
        "Tabs" => ".amana-tabs".to_string(),
        "Badge" => ".amana-badge".to_string(),
        "Kpi" | "Stat" => ".amana-kpi".to_string(),
        "LogoCloud" => ".amana-logo-cloud".to_string(),
        "TestimonialCard" => ".amana-testimonial".to_string(),
        "Timeline" => ".amana-timeline".to_string(),
        "TimelineItem" => ".amana-timeline-item".to_string(),
        "EmptyState" => ".amana-empty-state".to_string(),
        "Split" => ".amana-split".to_string(),
        "Cluster" => ".amana-cluster".to_string(),
        "Sidebar" => ".amana-sidebar".to_string(),
        "Slides" => ".amana-slides".to_string(),
        _ => format!(".amana-component-{}", token),
    }
}

fn variant_selector(variant: &VariantDecl) -> String {
    let name = design_token(&variant.name);
    let base = target_base_selector(&variant.target);
    format!(
        ":where({}.amana-variant-{}, {}.dg-component-variant-{})",
        base, name, base, name
    )
}

fn scoped_variant_selector(base: &str, rule: &StyleRule) -> String {
    let selector = rule.selector.trim();
    if selector.is_empty() || selector == "&" {
        return base.to_string();
    }
    if selector.contains('&') {
        selector.replace('&', base)
    } else {
        format!("{} {}", base, selector)
    }
}

fn compile_variant_rule(selector: &str, rule: &StyleRule) -> String {
    let scoped = scoped_variant_selector(selector, rule);
    let mut declarations = Vec::new();
    for decl in &rule.declarations {
        if let Ok(compiled) = crate::parser::css::compile_css_decl(&decl.property, &decl.value) {
            declarations.push(compiled);
        }
    }
    if declarations.is_empty() {
        return String::new();
    }
    format!("{} {{ {} }}\n", scoped, declarations.join(" "))
}

fn responsive_media(breakpoint: &str) -> Option<&'static str> {
    match breakpoint {
        "desktop" => Some("(min-width: 1201px)"),
        "tablet" => Some("(max-width: 900px) and (min-width: 641px)"),
        "mobile" => Some("(max-width: 640px)"),
        _ => None,
    }
}

pub(crate) fn variant_css(variants: &[VariantDecl]) -> String {
    let mut css = String::new();
    for variant in variants {
        let selector = variant_selector(variant);
        for rule in &variant.base_rules {
            css.push_str(&compile_variant_rule(&selector, rule));
        }
        for rule in &variant.hover_rules {
            css.push_str(&compile_variant_rule(&format!("{}:hover", selector), rule));
        }
        for (slot, rules) in &variant.slot_rules {
            let slot_selector = format!(
                "{} :where(.slot-{}, [data-slot=\"{}\"])",
                selector,
                design_token(slot),
                slot
            );
            for rule in rules {
                css.push_str(&compile_variant_rule(&slot_selector, rule));
            }
        }
        for responsive in &variant.responsive_rules {
            if let Some(media) = responsive_media(&responsive.breakpoint) {
                let mut body = String::new();
                for rule in &responsive.rules {
                    body.push_str(&compile_variant_rule(&selector, rule));
                }
                if !body.is_empty() {
                    css.push_str(&format!("@media {} {{\n{}}}\n", media, body));
                }
            }
        }
    }
    css
}

fn persist_mode_js(mode: &PersistMode) -> Option<&'static str> {
    match mode {
        PersistMode::Memory => None,
        PersistMode::Local => Some("local"),
        PersistMode::Session => Some("session"),
        PersistMode::Cookie => Some("cookie"),
    }
}

fn js_single_quoted(value: &str) -> String {
    format!(
        "'{}'",
        value
            .replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    )
}

fn alpine_state_data(view: &ViewIR, auth_model: &str) -> String {
    let mut state_fields = Vec::new();
    let mut hydrate_lines = Vec::new();
    let mut watch_lines = Vec::new();

    for state in &view.client_states {
        let initial_js = crate::codegen::html::compile_expression_to_js_with_auth_model(
            &state.initial_value,
            auth_model,
        );
        state_fields.push(format!("{}: {}", state.name, initial_js));
        if let Some(mode) = persist_mode_js(&state.persist) {
            let key = format!("amana:{}:{}", view.name, state.name);
            hydrate_lines.push(format!(
                "this.{name} = load({mode}, {key}, this.{name});",
                name = state.name,
                mode = js_single_quoted(mode),
                key = js_single_quoted(&key)
            ));
            watch_lines.push(format!(
                "this.$watch({name_key}, value => save({mode}, {key}, value));",
                name_key = js_single_quoted(&state.name),
                mode = js_single_quoted(mode),
                key = js_single_quoted(&key)
            ));
        }
    }

    if hydrate_lines.is_empty() && watch_lines.is_empty() {
        return format!("{{ {} }}", state_fields.join(", "));
    }

    state_fields.push(format!(
        "init() {{ const readCookie = key => document.cookie.split('; ').find(row => row.startsWith(key + '='))?.split('=').slice(1).join('='); const writeCookie = (key, value) => {{ document.cookie = `${{key}}=${{value}}; path=/; max-age=31536000; samesite=lax`; }}; const load = (mode, key, fallback) => {{ try {{ const raw = mode === 'local' ? localStorage.getItem(key) : mode === 'session' ? sessionStorage.getItem(key) : mode === 'cookie' ? readCookie(key) : null; return raw == null ? fallback : JSON.parse(decodeURIComponent(raw)); }} catch (_) {{ return fallback; }} }}; const save = (mode, key, value) => {{ try {{ const raw = encodeURIComponent(JSON.stringify(value)); if (mode === 'local') localStorage.setItem(key, raw); else if (mode === 'session') sessionStorage.setItem(key, raw); else if (mode === 'cookie') writeCookie(key, raw); }} catch (_) {{}} }}; {} {} }}",
        hydrate_lines.join(" "),
        watch_lines.join(" ")
    ));

    format!("{{ {} }}", state_fields.join(", "))
}

pub(crate) fn compile_default_login_ejs(
    html_lang: &str,
    html_dir: &str,
    bootstrap_css: &str,
) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="{}" dir="{}">
<head>
  <meta charset="UTF-8">
  <title>تسجيل الدخول</title>
  <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/{}" rel="stylesheet">
  <style>
    body {{ background-color: #f8f9fa; height: 100vh; display: flex; align-items: center; justify-content: center; }}
    .login-card {{ width: 400px; padding: 2rem; border-radius: 8px; background: white; box-shadow: 0 10px 15px -3px rgba(0,0,0,0.1); }}
  </style>
</head>
<body>
  <div class="login-card">
    <h2 class="text-center mb-4">تسجيل الدخول</h2>
    <% if (error) {{ %>
      <div class="alert alert-danger"><%= error %></div>
    <% }} %>
    <form action="/login" method="POST">
      <input type="hidden" name="_csrf" value="<%= csrfToken %>">
      <div class="mb-3">
        <label class="form-label" for="email">البريد الإلكتروني</label>
        <input class="form-control" type="email" id="email" name="email" required>
      </div>
      <div class="mb-3">
        <label class="form-label" for="password">كلمة المرور</label>
        <input class="form-control" type="password" id="password" name="password" required>
      </div>
      <button class="btn btn-primary w-100" type="submit">دخول</button>
    </form>
    <div class="mt-3 text-center text-muted">
      <small>Use AMANA_SEED_ADMIN=true with AMANA_ADMIN_EMAIL and AMANA_ADMIN_PASSWORD to create an initial admin account.</small>
    </div>
  </div>
</body>
</html>"#,
        html_lang, html_dir, bootstrap_css
    )
}

fn design_token(val: &str) -> String {
    let mut res = String::new();
    let val_lower = val.trim().to_lowercase();
    let mut prev_was_hyphen = false;
    for c in val_lower.chars() {
        if c.is_ascii_alphanumeric() {
            res.push(c);
            prev_was_hyphen = false;
        } else {
            if !res.is_empty() && !prev_was_hyphen {
                res.push('-');
                prev_was_hyphen = true;
            }
        }
    }
    if res.ends_with('-') {
        res.pop();
    }
    res
}

pub(crate) fn design_class_list(canvas: &DesignBlock) -> Vec<String> {
    let mut classes = Vec::new();
    let kind = design_token(&canvas.kind);
    if kind.is_empty() {
        return classes;
    }
    for (raw_key, raw_value) in &canvas.settings {
        let key = design_token(&raw_key.replace('.', "-"));
        let value = design_token(raw_value);
        if key.is_empty() || value.is_empty() {
            continue;
        }
        classes.push(format!("dg-{}-{}-{}", kind, key, value));
        if kind == "canvas" && key.starts_with("responsive-") {
            classes.push(format!(
                "dg-rsp-{}-{}",
                key.replace("responsive-", ""),
                value
            ));
        }
        if kind == "canvas" && key == "layout" {
            classes.push(format!("dg-layout-{}", value));
        }
        if kind == "canvas" && key == "surface" {
            classes.push(format!("dg-surface-{}", value));
        }
        if kind == "canvas" && key == "density" {
            classes.push(format!("dg-density-{}", value));
        }
        if kind == "canvas" && key == "rhythm" {
            classes.push(format!("dg-rhythm-{}", value));
        }
        if kind == "canvas" && key == "mode" {
            classes.push(format!("dg-mode-{}", value));
        }
        if kind == "canvas" && key == "palette" {
            classes.push(format!("dg-palette-{}", value));
        }
        if kind == "compose" && key == "layout" {
            classes.push(format!("dg-layout-{}", value));
        }
        if kind == "compose" && key == "rhythm" {
            classes.push(format!("dg-rhythm-{}", value));
        }
        if kind == "compose" && key == "density" {
            classes.push(format!("dg-density-{}", value));
        }
        if kind == "compose" && key == "flow" {
            classes.push(format!("dg-flow-{}", value));
        }
        if kind == "compose" && key == "focus-path" {
            classes.push(format!("dg-focus-path-{}", value));
        }
        if kind == "compose" && key == "alignment" {
            classes.push(format!("dg-align-{}", value));
        }
        if kind == "visual" && key == "gradient" {
            classes.push(format!("dg-gradient-{}", value));
        }
        if kind == "visual" && key == "surface" {
            classes.push(format!("dg-surface-{}", value));
        }
        if kind == "visual" && key == "shape" {
            classes.push(format!("dg-shape-{}", value));
        }
        if kind == "visual" && key == "mode" {
            classes.push(format!("dg-mode-{}", value));
        }
        if kind == "visual" && key == "texture" {
            classes.push(format!("dg-texture-{}", value));
        }
        if kind == "visual" && key == "palette" {
            classes.push(format!("dg-palette-{}", value));
        }
        if kind == "visual" && key == "frame" {
            classes.push(format!("dg-frame-{}", value));
        }
        if kind == "component" && key == "variant" {
            classes.push(format!("dg-component-variant-{}", value));
        }
        if kind == "component" && key == "shape" {
            classes.push(format!("dg-component-shape-{}", value));
        }
        if kind == "component" && key == "density" {
            classes.push(format!("dg-component-density-{}", value));
        }
        if kind == "component" && key == "chrome" {
            classes.push(format!("dg-component-chrome-{}", value));
        }
        if kind == "type" && key == "scale" {
            classes.push(format!("dg-type-scale-{}", value));
        }
        if kind == "type" && key == "align" {
            classes.push(format!("dg-type-align-{}", value));
        }
        if kind == "type" && key == "measure" {
            classes.push(format!("dg-type-measure-{}", value));
        }
        if kind == "type" && key == "hierarchy" {
            classes.push(format!("dg-type-hierarchy-{}", value));
        }
        if kind == "type" && key == "tone" {
            classes.push(format!("dg-type-tone-{}", value));
        }
        if kind == "motion" && key == "entrance" {
            classes.push(format!("dg-motion-{}", value));
        }
        if kind == "motion" && key == "hover" {
            classes.push(format!("dg-hover-{}", value));
        }
        if kind == "motion" && key == "reveal" {
            classes.push(format!("dg-reveal-{}", value));
        }
        if kind == "brand" && key == "voice" {
            classes.push(format!("dg-brand-voice-{}", value));
        }
        if kind == "brand" && key == "personality" {
            classes.push(format!("dg-brand-personality-{}", value));
        }
        if kind == "brand" && key == "colorway" {
            classes.push(format!("dg-colorway-{}", value));
        }
        if kind == "brand" && key == "trust" {
            classes.push(format!("dg-brand-trust-{}", value));
        }
        if kind == "art" && key == "direction" {
            classes.push(format!("dg-art-{}", value));
        }
        if kind == "art" && key == "motif" {
            classes.push(format!("dg-motif-{}", value));
        }
        if kind == "art" && key == "lighting" {
            classes.push(format!("dg-lighting-{}", value));
        }
        if kind == "art" && key == "texture" {
            classes.push(format!("dg-texture-{}", value));
        }
        if kind == "responsive" {
            classes.push(format!("dg-rsp-{}-{}", key, value));
        }
        if kind == "interaction" && key == "feedback" {
            classes.push(format!("dg-feedback-{}", value));
        }
        if kind == "interaction" && key == "affordance" {
            classes.push(format!("dg-affordance-{}", value));
        }
        if kind == "interaction" && key == "cursor" {
            classes.push(format!("dg-cursor-{}", value));
        }
        if kind == "a11y" && key == "contrast" {
            classes.push(format!("dg-a11y-contrast-{}", value));
        }
        if kind == "a11y" && key == "focus" {
            classes.push(format!("dg-focus-visible-{}", value));
        }
        if kind == "a11y" && key == "reduce-motion" {
            classes.push(format!("dg-reduce-motion-{}", value));
        }
    }
    classes
}

fn normalize_design_style_value(key: &str, value: &str) -> String {
    let raw = value.trim();
    let normalized_key = key.to_lowercase().replace('_', "-");
    let token = raw.to_lowercase();
    if raw.is_empty() {
        return String::new();
    }

    if [
        "padding",
        "padding-x",
        "padding-y",
        "space-padding",
        "space-padding-x",
        "space-padding-y",
        "gap",
        "space-gap",
        "margin",
        "margin-x",
        "margin-y",
    ]
    .contains(&normalized_key.as_str())
        || normalized_key.ends_with(".padding")
        || normalized_key.ends_with(".gap")
    {
        return match token.as_str() {
            "none" | "0" => "0".to_string(),
            "xs" => "var(--space-xs)".to_string(),
            "sm" => "var(--space-sm)".to_string(),
            "small" => "var(--padding-small)".to_string(),
            "md" => "var(--space-md)".to_string(),
            "medium" => "var(--padding-medium)".to_string(),
            "lg" => "var(--space-lg)".to_string(),
            "large" => "var(--padding-large)".to_string(),
            "xl" => "var(--space-xl)".to_string(),
            "2xl" | "xxl" => "var(--space-2xl)".to_string(),
            "3xl" => "var(--space-3xl)".to_string(),
            "4xl" => "var(--space-4xl)".to_string(),
            _ => raw.to_string(),
        };
    }

    if [
        "width",
        "height",
        "min-width",
        "min-height",
        "max-width",
        "max-height",
        "title-width",
        "copy-width",
        "text-width",
    ]
    .contains(&normalized_key.as_str())
        || ["size", "font-size", "font_size", "copy-size", "title-size"]
            .contains(&normalized_key.as_str())
    {
        return match token.as_str() {
            "full" => "100%".to_string(),
            "screen" => "100vh".to_string(),
            "fit" => "fit-content".to_string(),
            "min" => "min-content".to_string(),
            "max" => "max-content".to_string(),
            "content" => "var(--content-width)".to_string(),
            "readable" => "var(--readable-width)".to_string(),
            "wide" => "var(--wide-width)".to_string(),
            "fluid-xs" => "clamp(0.75rem, 1.4vw, 0.9rem)".to_string(),
            "fluid-sm" => "clamp(0.875rem, 1.6vw, 1rem)".to_string(),
            "fluid-md" => "clamp(1rem, 1.8vw, 1.15rem)".to_string(),
            "fluid-lg" => "clamp(1.125rem, 2.2vw, 1.35rem)".to_string(),
            "fluid-xl" => "clamp(1.5rem, 4vw, 2.4rem)".to_string(),
            "fluid-2xl" => "clamp(2rem, 6vw, 4rem)".to_string(),
            "fluid-3xl" => "clamp(2.6rem, 8vw, 6rem)".to_string(),
            _ => raw.to_string(),
        };
    }

    if [
        "primary",
        "accent",
        "background",
        "bg",
        "surface-bg",
        "color-background",
        "surface-color",
        "fill",
        "text",
        "ink",
        "color-text",
        "muted",
        "subtle",
        "color-muted",
        "border",
        "border-color",
        "stroke",
        "outline",
    ]
    .contains(&normalized_key.as_str())
    {
        return match token.as_str() {
            "primary" => "var(--color-primary)".to_string(),
            "primary-soft" => "var(--color-primary-soft)".to_string(),
            "accent" => "var(--color-accent)".to_string(),
            "success" => "var(--color-success)".to_string(),
            "warning" => "var(--color-warning)".to_string(),
            "danger" => "var(--color-danger)".to_string(),
            "canvas" => "var(--bg-secondary)".to_string(),
            "surface" => "var(--surface-base)".to_string(),
            "surface-muted" => "var(--surface-muted)".to_string(),
            "surface-elevated" => "var(--surface-elevated)".to_string(),
            "text" | "ink" => "var(--text-primary)".to_string(),
            "muted" | "subtle" => "var(--text-secondary)".to_string(),
            "border" => "var(--border-subtle)".to_string(),
            "custom-primary" => "var(--custom-primary, var(--color-primary))".to_string(),
            "custom-accent" => "var(--custom-accent, var(--color-accent))".to_string(),
            "custom-bg" => "var(--custom-bg, var(--bg-secondary))".to_string(),
            "custom-text" => "var(--custom-text, var(--text-primary))".to_string(),
            _ => raw.to_string(),
        };
    }

    if ["radius", "shape-radius"].contains(&normalized_key.as_str()) {
        return match token.as_str() {
            "full" | "pill" => "9999px".to_string(),
            "xl" => "var(--radius-xl)".to_string(),
            "2xl" => "var(--radius-2xl)".to_string(),
            "soft" => "var(--radius-soft)".to_string(),
            "lg" | "large" => "var(--radius-large)".to_string(),
            "md" | "medium" => "var(--radius-medium)".to_string(),
            "sm" | "small" => "var(--radius-small)".to_string(),
            _ => raw.to_string(),
        };
    }

    if ["shadow", "shadow-value"].contains(&normalized_key.as_str()) {
        return match token.as_str() {
            "none" => "none".to_string(),
            "soft" => "var(--shadow-soft)".to_string(),
            "smooth" => "var(--shadow-smooth)".to_string(),
            "floating" => "var(--shadow-floating)".to_string(),
            "strong" => "var(--shadow-strong)".to_string(),
            "lg" | "large" => "var(--shadow-large)".to_string(),
            "primary" => "var(--glow-primary)".to_string(),
            "accent" => "var(--glow-accent)".to_string(),
            _ => raw.to_string(),
        };
    }

    if ["gradient", "gradient-value", "custom-gradient"].contains(&normalized_key.as_str()) {
        return match token.as_str() {
            "primary" => "var(--gradient-primary)".to_string(),
            "accent" => "var(--gradient-accent)".to_string(),
            "hero" => "var(--gradient-hero)".to_string(),
            "mesh" => "var(--gradient-mesh)".to_string(),
            "aurora" => "var(--gradient-aurora)".to_string(),
            "spotlight" => "var(--gradient-spotlight)".to_string(),
            "custom" => "var(--custom-gradient, var(--gradient-primary))".to_string(),
            "brand" => "linear-gradient(var(--gradient-angle, 135deg), var(--custom-primary, var(--color-primary)), var(--custom-accent, var(--color-accent)))".to_string(),
            _ => raw.to_string(),
        };
    }

    raw.to_string()
}

fn safe_design_var_value(val: &str) -> String {
    let lower = val.to_lowercase();
    if lower.contains("javascript:")
        || lower.contains("expression")
        || lower.contains("behavior")
        || lower.contains("@import")
        || lower.contains("url")
        || lower.contains('<')
        || lower.contains('>')
        || lower.contains(';')
        || lower.contains('{')
        || lower.contains('}')
    {
        return String::new();
    }
    if !val
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || " .,%#()+-/*".contains(c))
    {
        return String::new();
    }
    escape_html_attr(val)
}

pub(crate) fn design_style_vars(canvas: &DesignBlock) -> String {
    let mut styles = Vec::new();
    let kind = canvas.kind.as_str();
    for (raw_key, raw_value) in &canvas.settings {
        let key = raw_key.replace('_', "-");
        let normalized_value = normalize_design_style_value(&key, raw_value);
        let clean_value = safe_design_var_value(&normalized_value);
        if clean_value.is_empty() {
            continue;
        }

        let can_size_component = matches!(kind, "visual" | "tokens" | "component" | "canvas");
        match (kind, key.as_str()) {
            ("visual", "depth") => styles.push(format!("--dg-depth:{}", clean_value)),
            ("visual", "visual-weight") => {
                styles.push(format!("--dg-visual-weight:{}", clean_value))
            }
            ("visual", "texture-opacity") => {
                styles.push(format!("--dg-texture-opacity:{}", clean_value))
            }
            ("visual", "glow-strength") => {
                styles.push(format!("--dg-glow-strength:{}", clean_value))
            }
            (
                "visual" | "tokens" | "component" | "canvas",
                "primary" | "color.primary" | "brand.primary",
            ) => {
                styles.push(format!("--custom-primary:{}", clean_value));
            }
            (
                "visual" | "tokens" | "component" | "canvas",
                "accent" | "color.accent" | "brand.accent",
            ) => {
                styles.push(format!("--custom-accent:{}", clean_value));
            }
            (
                "visual" | "tokens" | "component" | "canvas",
                "background" | "bg" | "surface.bg" | "color.background",
            ) => {
                styles.push(format!(
                    "--custom-bg:{};background:{}",
                    clean_value, clean_value
                ));
            }
            ("visual" | "tokens" | "component" | "canvas", "text" | "ink" | "color.text") => {
                styles.push(format!(
                    "--custom-text:{};color:{}",
                    clean_value, clean_value
                ));
            }
            ("visual" | "tokens" | "component" | "canvas", "muted" | "subtle" | "color.muted") => {
                styles.push(format!("--custom-muted:{}", clean_value));
            }
            (
                "visual" | "tokens" | "component" | "canvas",
                "border" | "border.color" | "stroke" | "outline",
            ) => {
                styles.push(format!(
                    "--custom-border:{};border-color:{}",
                    clean_value, clean_value
                ));
            }
            (
                "visual" | "tokens" | "component" | "canvas",
                "gradient.value" | "gradient-value" | "custom-gradient",
            ) => {
                styles.push(format!(
                    "--custom-gradient:{};background:{}",
                    clean_value, clean_value
                ));
            }
            ("visual" | "tokens" | "component" | "canvas", "radius" | "shape.radius") => {
                styles.push(format!(
                    "--custom-radius:{};border-radius:{}",
                    clean_value, clean_value
                ));
            }
            ("visual" | "tokens" | "component" | "canvas", "shadow" | "shadow.value") => {
                styles.push(format!(
                    "--custom-shadow:{};box-shadow:{}",
                    clean_value, clean_value
                ));
            }
            ("visual" | "tokens" | "component" | "canvas", "padding" | "space.padding") => {
                styles.push(format!(
                    "--custom-padding:{};padding:{}",
                    clean_value, clean_value
                ));
            }
            ("visual" | "tokens" | "component" | "canvas", "gap" | "space.gap") => {
                styles.push(format!("--custom-gap:{};gap:{}", clean_value, clean_value));
            }
            ("compose", "columns") => {
                let formatted = if let Ok(num) = clean_value.trim().parse::<u32>() {
                    if num == 1 {
                        "minmax(0, 1fr)".to_string()
                    } else {
                        format!("repeat({}, minmax(0, 1fr))", num)
                    }
                } else {
                    clean_value.clone()
                };
                styles.push(format!("--dg-columns:{}", formatted))
            }
            ("compose", "gap") => styles.push(format!("--dg-gap:{}", clean_value)),
            ("compose", "grid-min") => styles.push(format!("--grid-min:{}", clean_value)),
            ("compose", "max-width") => styles.push(format!("--dg-max-width:{}", clean_value)),
            ("compose", "template" | "grid-template") => {
                styles.push(format!("--dg-template:{}", clean_value))
            }
            ("motion", "speed") => styles.push(format!("--dg-motion-speed:{}", clean_value)),
            ("type", "measure") => styles.push(format!("--dg-type-measure:{}", clean_value)),
            ("type", "weight") => styles.push(format!("--dg-type-weight:{}", clean_value)),
            ("type", "size" | "font-size") => styles.push(format!("font-size:{}", clean_value)),
            ("type", "leading" | "line-height") => {
                styles.push(format!("line-height:{}", clean_value))
            }
            ("type", "tracking" | "letter-spacing") => {
                styles.push(format!("letter-spacing:{}", clean_value))
            }
            ("canvas", "content-width") => styles.push(format!("--content-width:{}", clean_value)),
            ("canvas", "wide-width") => styles.push(format!("--wide-width:{}", clean_value)),
            ("canvas", "readable-width") => {
                styles.push(format!("--readable-width:{}", clean_value))
            }
            ("responsive", "columns") => {
                let formatted = if let Ok(num) = clean_value.trim().parse::<u32>() {
                    if num == 1 {
                        "minmax(0, 1fr)".to_string()
                    } else {
                        format!("repeat({}, minmax(0, 1fr))", num)
                    }
                } else {
                    clean_value.clone()
                };
                styles.push(format!("--dg-responsive-columns:{}", formatted))
            }
            ("responsive", "desktop.columns") => {
                let formatted = if let Ok(num) = clean_value.trim().parse::<u32>() {
                    if num == 1 {
                        "minmax(0, 1fr)".to_string()
                    } else {
                        format!("repeat({}, minmax(0, 1fr))", num)
                    }
                } else {
                    clean_value.clone()
                };
                styles.push(format!("--bp-desktop-columns:{}", formatted))
            }
            ("responsive", "laptop.columns") => {
                let formatted = if let Ok(num) = clean_value.trim().parse::<u32>() {
                    if num == 1 {
                        "minmax(0, 1fr)".to_string()
                    } else {
                        format!("repeat({}, minmax(0, 1fr))", num)
                    }
                } else {
                    clean_value.clone()
                };
                styles.push(format!("--bp-laptop-columns:{}", formatted))
            }
            ("responsive", "tablet.columns") => {
                let formatted = if let Ok(num) = clean_value.trim().parse::<u32>() {
                    if num == 1 {
                        "minmax(0, 1fr)".to_string()
                    } else {
                        format!("repeat({}, minmax(0, 1fr))", num)
                    }
                } else {
                    clean_value.clone()
                };
                styles.push(format!("--bp-tablet-columns:{}", formatted))
            }
            ("responsive", "mobile.columns") => {
                let formatted = if let Ok(num) = clean_value.trim().parse::<u32>() {
                    if num == 1 {
                        "minmax(0, 1fr)".to_string()
                    } else {
                        format!("repeat({}, minmax(0, 1fr))", num)
                    }
                } else {
                    clean_value.clone()
                };
                styles.push(format!("--bp-mobile-columns:{}", formatted))
            }
            ("responsive", "desktop.padding") => {
                styles.push(format!("--bp-desktop-padding:{}", clean_value))
            }
            ("responsive", "laptop.padding") => {
                styles.push(format!("--bp-laptop-padding:{}", clean_value))
            }
            ("responsive", "tablet.padding") => {
                styles.push(format!("--bp-tablet-padding:{}", clean_value))
            }
            ("responsive", "mobile.padding") => {
                styles.push(format!("--bp-mobile-padding:{}", clean_value))
            }
            ("responsive", "desktop.gap") => {
                styles.push(format!("--bp-desktop-gap:{}", clean_value))
            }
            ("responsive", "laptop.gap") => styles.push(format!("--bp-laptop-gap:{}", clean_value)),
            ("responsive", "tablet.gap") => styles.push(format!("--bp-tablet-gap:{}", clean_value)),
            ("responsive", "mobile.gap") => styles.push(format!("--bp-mobile-gap:{}", clean_value)),
            ("art", "texture-opacity") => {
                styles.push(format!("--dg-texture-opacity:{}", clean_value))
            }
            ("interaction", "focus-strength") => {
                styles.push(format!("--dg-focus-strength:{}", clean_value))
            }
            _ => {}
        }

        if can_size_component {
            match key.as_str() {
                "min-height" => styles.push(format!(
                    "--component-min-height:{};min-height:{}",
                    clean_value, clean_value
                )),
                "height" => styles.push(format!(
                    "--component-height:{};height:{}",
                    clean_value, clean_value
                )),
                "width" => styles.push(format!(
                    "--component-width:{};width:{}",
                    clean_value, clean_value
                )),
                "max-width" => styles.push(format!(
                    "--component-max-width:{};max-width:{}",
                    clean_value, clean_value
                )),
                "padding-x" => styles.push(format!("--component-padding-x:{}", clean_value)),
                "padding-y" => styles.push(format!("--component-padding-y:{}", clean_value)),
                "min-width" => styles.push(format!(
                    "--component-min-width:{};min-width:{}",
                    clean_value, clean_value
                )),
                "max-height" => styles.push(format!(
                    "--component-max-height:{};max-height:{}",
                    clean_value, clean_value
                )),
                "columns" | "layout.columns" => {
                    let formatted = if let Ok(num) = clean_value.trim().parse::<u32>() {
                        if num == 1 {
                            "minmax(0, 1fr)".to_string()
                        } else {
                            format!("repeat({}, minmax(0, 1fr))", num)
                        }
                    } else {
                        clean_value.clone()
                    };
                    styles.push(format!(
                        "--component-columns:{};--dg-columns:{}",
                        formatted, formatted
                    ))
                }
                "title-width" => styles.push(format!("--component-title-width:{}", clean_value)),
                "copy-width" | "text-width" => {
                    styles.push(format!("--component-copy-width:{}", clean_value))
                }
                "title-size" => styles.push(format!("--component-title-size:{}", clean_value)),
                "copy-size" | "text-size" => {
                    styles.push(format!("--component-copy-size:{}", clean_value))
                }
                "transition" | "motion.transition" => styles.push(format!(
                    "--component-transition:{};transition:{}",
                    clean_value, clean_value
                )),
                "transform" => styles.push(format!(
                    "--component-transform:{};transform:{}",
                    clean_value, clean_value
                )),
                "opacity" => styles.push(format!(
                    "--component-opacity:{};opacity:{}",
                    clean_value, clean_value
                )),
                _ => {}
            }
        }
    }
    styles.join(";")
}

fn design_settings_summary(canvas: &DesignBlock) -> String {
    canvas
        .settings
        .iter()
        .take(12)
        .map(|(k, v)| format!("{}:{}", k, v))
        .collect::<Vec<String>>()
        .join(";")
}

pub(crate) fn design_data_attrs(canvas: &DesignBlock) -> String {
    let mut attrs = Vec::new();
    let kind = design_token(&canvas.kind);
    if !kind.is_empty() {
        attrs.push(format!("data-dg-blocks=\"{}\"", escape_html_attr(&kind)));
        let summary = design_settings_summary(canvas);
        if !summary.is_empty() {
            attrs.push(format!(
                "data-dg-{}=\"{}\"",
                escape_html_attr(&kind),
                escape_html_attr(&summary)
            ));
        }
    }
    attrs.join(" ")
}

fn canvas_attributes(canvas: Option<&DesignBlock>) -> String {
    let canvas = match canvas {
        Some(c) => c,
        None => return " class=\"amana-page\"".to_string(),
    };

    let mut class_list = vec!["amana-page".to_string()];
    class_list.extend(design_class_list(canvas));

    let mut unique_classes = Vec::new();
    for cls in class_list {
        if !unique_classes.contains(&cls) {
            unique_classes.push(cls);
        }
    }
    let class_attr = format!(" class=\"{}\"", escape_html_attr(&unique_classes.join(" ")));

    let style = design_style_vars(canvas);
    let style_attr = if !style.is_empty() {
        format!(" style=\"{}\"", escape_html_attr(&style))
    } else {
        String::new()
    };

    let data_attrs = design_data_attrs(canvas);
    let data_attr = if !data_attrs.is_empty() {
        format!(" {}", data_attrs)
    } else {
        String::new()
    };

    format!("{}{}{}", class_attr, style_attr, data_attr)
}

fn escape_html_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

const BASE_CSS_CLASSES: &str = r#"
    *, *::before, *::after { box-sizing: border-box; }
    html { width: 100%; max-width: 100%; overflow-x: hidden; scroll-behavior: smooth; }
    body { width: 100%; max-width: 100%; min-width: 0; margin: 0; overflow-x: hidden; background-color: var(--bg-secondary); color: var(--text-primary); font: var(--font-body); text-rendering: geometricPrecision; }
    body.amana-page { display: block; padding: 0 !important; gap: normal !important; }
    :where(main, section, article, aside, header, footer, nav, div, form) { min-width: 0; }
    :where(h1, h2, h3, h4, h5, h6, p, span, strong, a, button, label, input, textarea, pre) { max-width: 100%; overflow-wrap: break-word; word-break: normal; letter-spacing: 0; }
    :where(h1, h2, h3) { text-wrap: balance; }
    :where(p, li, blockquote) { text-wrap: pretty; }
    img, svg, video, canvas { max-width: 100%; display: block; }
    a { color: inherit; }
    .card { border: none; background-color: var(--surface-elevated); box-shadow: var(--shadow-soft); }
    .amana-container { width: var(--component-width, min(100% - 2rem, var(--content-width))); max-width: var(--component-max-width, none); margin-inline: auto; }
    .amana-container-full { width: 100%; max-width: none; }
    .amana-container-wide { width: min(100% - 2rem, var(--wide-width)); }
    .amana-container-readable { width: min(100% - 2rem, var(--readable-width)); }
    .amana-section { position: relative; width: var(--component-width, auto); max-width: var(--component-max-width, none); min-height: var(--component-min-height, auto); height: var(--component-height, auto); padding-block: var(--component-padding-y, var(--component-padding, clamp(3rem, 7vw, 6rem))); padding-inline: var(--component-padding-x, 0); }
    .amana-section-head { display: grid; gap: var(--space-sm); margin-bottom: clamp(2rem, 5vw, 3.5rem); max-width: 780px; }
    .amana-section-head:has(.amana-section-copy) { gap: var(--space-md); }
    .amana-section h2, .amana-section-head h2 { margin: 0; font-size: clamp(2rem, 5vw, 4.2rem); line-height: 1.05; letter-spacing: -0.01em; font-weight: 900; }
    .amana-section-copy { color: var(--text-secondary); max-width: 68ch; font-size: clamp(1rem, 2vw, 1.2rem); line-height: 1.8; }
    .amana-grid { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, var(--dg-columns, repeat(auto-fit, minmax(var(--grid-min, 16rem), 1fr))))); gap: var(--component-gap, var(--custom-gap, var(--dg-gap, var(--space-lg)))); }
    .amana-grid > *, .amana-split > *, .dg-layout-split-diagonal > *, .dg-layout-asymmetric > *, .dg-layout-editorial > *, .dg-layout-dashboard-shell > *, .dg-layout-command-center > *, .dg-layout-showcase-rail > * { min-width: 0; }
    .amana-stack { display: flex; flex-direction: column; gap: var(--space-md); }
    .amana-stack-gap-xs { gap: var(--space-xs); }
    .amana-stack-gap-sm { gap: var(--space-sm); }
    .amana-stack-gap-lg { gap: var(--space-lg); }
    .amana-stack-gap-xl { gap: var(--space-xl); }
    .amana-navbar { width: var(--component-width, min(100% - 2rem, var(--wide-width))); max-width: var(--component-max-width, none); margin-inline: auto; display: flex; align-items: center; justify-content: space-between; gap: var(--component-gap, var(--space-lg)); padding: var(--component-padding, 0.85rem 0); min-height: var(--component-min-height, 4.25rem); }
    .amana-brand { display: inline-flex; align-items: center; gap: 0.65rem; color: var(--text-primary); font-weight: 900; text-decoration: none; letter-spacing: -0.01em; }
    .amana-brand::before { content: ""; width: 0.72rem; height: 0.72rem; border-radius: 999px; background: var(--gradient-primary); box-shadow: var(--glow-accent); }
    .amana-navlinks { display: flex; align-items: center; justify-content: flex-end; gap: var(--space-xs); flex-wrap: wrap; }
    .amana-navlinks a { text-decoration: none; padding: 0.5rem 0.85rem; border-radius: var(--radius-soft); font-size: var(--text-sm); font-weight: 600; color: var(--text-secondary); transition: var(--transition-fast); }
    .amana-navlinks a:hover { background: var(--color-primary-soft); color: var(--color-primary); }
    .amana-navlinks a:focus-visible { outline: 2px solid var(--color-primary); }
    .amana-hero { position: relative; isolation: isolate; display: grid; grid-template-columns: var(--component-columns, var(--dg-template, var(--dg-columns, minmax(0, 1fr)))); gap: var(--component-gap, var(--custom-gap, var(--dg-gap, clamp(1.5rem, 4vw, 3.5rem)))); align-items: center; width: var(--component-width, auto); max-width: var(--component-max-width, none); min-width: var(--component-min-width, 0); min-height: var(--component-min-height, auto); height: var(--component-height, auto); padding: var(--component-padding, clamp(1.5rem, 4vw, 4rem)); background: var(--custom-bg, var(--custom-gradient, var(--gradient-hero))); border: 1px solid var(--custom-border, var(--border-subtle)); border-radius: var(--custom-radius, var(--radius-2xl)); overflow: hidden; box-shadow: var(--custom-shadow, var(--shadow-floating)); opacity: var(--component-opacity, 1); transform: var(--component-transform, none); transition: var(--component-transition, transform 180ms ease, box-shadow 180ms ease, border-color 180ms ease); }
    .amana-hero::before { content: ""; position: absolute; inset: -20%; background: radial-gradient(circle at 15% 20%, rgba(34,211,238,0.18), transparent 28%), radial-gradient(circle at 85% 20%, rgba(99,102,241,0.22), transparent 32%); z-index: -1; }
    .amana-hero-content { display: grid; gap: var(--component-gap, var(--space-md)); max-width: var(--component-copy-width, 780px); min-width: 0; }
    .amana-hero h1 { margin: 0; font-size: var(--component-title-size, clamp(2.25rem, 5.4vw, 5.4rem)); line-height: var(--component-title-leading, 1.02); max-width: var(--component-title-width, min(100%, 16ch)); letter-spacing: -0.02em; font-weight: var(--dg-type-weight, 900); }
    .amana-hero-copy { margin: 0; max-width: var(--component-copy-width, 66ch); color: var(--custom-muted, var(--text-secondary)); font-size: var(--component-copy-size, clamp(1rem, 1.8vw, 1.2rem)); line-height: 1.8; }
    .amana-hero-actions { display: flex; gap: var(--component-gap, var(--space-md)); flex-wrap: wrap; margin-top: var(--space-md); align-items: center; }
    .amana-hero-proof { color: var(--text-secondary); font-weight: 800; }
    .amana-hero-media { min-height: clamp(16rem, 34vw, 28rem); border-radius: var(--radius-2xl); background-size: cover; background-position: center; border: 1px solid var(--border-subtle); box-shadow: var(--shadow-floating); }
    .amana-eyebrow { color: var(--color-accent); font-weight: 900; text-transform: uppercase; letter-spacing: 0.1em; font-size: var(--text-sm); }
    .amana-card { position: relative; display: grid; gap: var(--component-gap, var(--custom-gap, var(--space-md))); width: var(--component-width, auto); max-width: var(--component-max-width, none); min-width: var(--component-min-width, 0); min-height: var(--component-min-height, auto); height: var(--component-height, auto); background: var(--custom-bg, var(--custom-gradient, linear-gradient(180deg, color-mix(in srgb, var(--surface-elevated) 92%, transparent), color-mix(in srgb, var(--surface-muted) 82%, transparent)))); border: 1px solid var(--custom-border, var(--border-subtle)); border-radius: var(--custom-radius, var(--radius-2xl)); padding: var(--component-padding, var(--custom-padding, clamp(1.1rem, 2.6vw, 1.8rem))); box-shadow: var(--custom-shadow, var(--shadow-soft)); opacity: var(--component-opacity, 1); transform: var(--component-transform, none); overflow: hidden; transition: var(--component-transition, transform 180ms ease, box-shadow 180ms ease, border-color 180ms ease); }
    .amana-card::before { content: ""; position: absolute; inset: 0; pointer-events: none; background: linear-gradient(135deg, rgba(255,255,255,0.08), transparent 38%); opacity: 0.75; }
    .amana-card:hover { transform: translateY(-4px); box-shadow: var(--shadow-floating); border-color: color-mix(in srgb, var(--color-accent) 32%, var(--border-subtle)); }
    .amana-card > * { position: relative; }
    .amana-card h3 { margin: 0; font-size: clamp(1.25rem, 2vw, 1.75rem); line-height: 1.15; font-weight: 900; }
    .amana-feature-card { min-height: 13rem; }
    .amana-pricing-card { display: flex; flex-direction: column; gap: var(--space-md); border-color: var(--border-subtle); background: var(--surface-elevated); transition: var(--transition-smooth); }
    .amana-pricing-card.amana-variant-featured, .amana-pricing-card.dg-component-variant-featured { border: 2px solid var(--color-primary); transform: scale(1.02); box-shadow: var(--shadow-xl), var(--glow-primary); }
    .amana-pricing-card.amana-variant-featured:hover, .amana-pricing-card.dg-component-variant-featured:hover { transform: scale(1.04) translateY(-2px); }
    .amana-price { font-size: clamp(2rem, 5vw, 3.5rem); line-height: 1; font-weight: 950; }
    .amana-muted { color: var(--text-secondary); line-height: 1.75; }
    .amana-btn { position: relative; display: inline-flex; align-items: center; justify-content: center; gap: var(--component-gap, 0.65rem); width: var(--component-width, auto); max-width: var(--component-max-width, 100%); min-width: var(--component-min-width, 0); min-height: var(--component-min-height, 3rem); height: var(--component-height, auto); padding: var(--component-padding, 0.78rem 1.12rem); border-radius: var(--custom-radius, 999px); font-weight: 900; text-decoration: none; border: 1px solid var(--custom-border, transparent); transition: var(--component-transition, transform 160ms ease, box-shadow 160ms ease, border-color 160ms ease, background 160ms ease); white-space: nowrap; line-height: 1.15; overflow: hidden; opacity: var(--component-opacity, 1); transform: var(--component-transform, none); cursor: pointer; }
    .amana-btn:hover { transform: translateY(-2px); }
    .amana-btn:active { transform: translateY(0) scale(0.97); }
    .amana-btn:focus-visible { outline: 3px solid color-mix(in srgb, var(--color-accent) 60%, transparent); outline-offset: 3px; }
    .amana-btn-primary { background: var(--gradient-primary); color: white; box-shadow: var(--glow-primary); }
    .amana-btn-primary:hover { box-shadow: var(--shadow-floating), var(--glow-primary); }
    .amana-btn-secondary { color: var(--text-primary); background: color-mix(in srgb, var(--surface-elevated) 82%, transparent); border-color: var(--border-subtle); box-shadow: var(--shadow-soft); }
    .amana-btn-secondary:hover { background: var(--surface-muted); border-color: var(--color-primary-soft); }
    .amana-btn-ghost { color: var(--text-primary); background: transparent; border-color: var(--border-subtle); }
    .amana-btn-sm { min-height: 2.35rem; padding: 0.52rem 0.82rem; font-size: var(--text-sm); }
    .amana-btn-lg { min-height: 3.45rem; padding: 0.96rem 1.35rem; font-size: var(--text-lg); }
    .amana-icon, .amana-btn-icon, iconify-icon { display: inline-grid; place-items: center; width: 1.25em; min-width: 1.25em; height: 1.25em; line-height: 1; vertical-align: -0.18em; transition: transform 160ms ease; }
    .amana-btn:hover .amana-btn-icon { transform: translateX(-2px); }
    .amana-btn-intent-danger { background: var(--color-danger); color: white; }
    .amana-btn-intent-success { background: var(--color-success); color: white; }
    .amana-field { display: flex; flex-direction: column; gap: 0.45rem; margin-bottom: var(--space-md); width: 100%; }
    .amana-field span { color: var(--text-primary); font-weight: 800; font-size: var(--text-sm); }
    .amana-field input, .amana-form-control { width: 100%; border: 1px solid var(--border-subtle); border-radius: var(--radius-soft); min-height: 3rem; padding: 0.78rem 0.92rem; background: color-mix(in srgb, var(--surface-base) 84%, transparent); color: var(--text-primary); box-shadow: inset 0 1px 0 rgba(255,255,255,0.04); font-size: var(--text-sm); transition: all 0.12s ease-in-out; }
    .amana-field input:focus, .amana-form-control:focus { outline: none; border-color: var(--color-primary); box-shadow: 0 0 0 3px var(--color-primary-soft); }
    textarea.amana-form-control { min-height: 7rem; resize: vertical; }
    .amana-form-card { background: color-mix(in srgb, var(--surface-elevated) 88%, transparent); border: 1px solid var(--border-subtle); border-radius: var(--radius-2xl); padding: clamp(1.25rem, 3vw, 2rem); box-shadow: var(--shadow-floating); }
    .amana-help { color: var(--text-secondary); font-size: var(--text-sm); }
    .amana-alert { border-radius: var(--radius-soft); border: 1px solid var(--border-subtle); padding: var(--space-md); background: var(--surface-muted); }
    .amana-alert-success { border-color: rgba(22,163,74,0.35); }
    .amana-alert-danger { border-color: rgba(220,38,38,0.35); }
    .amana-footer { width: min(100% - 2rem, var(--wide-width)); margin: var(--space-3xl) auto 0; padding-block: var(--space-xl); color: var(--text-secondary); border-top: 1px solid var(--border-subtle); }
    .amana-modal { position: fixed; inset: 0; display: grid; place-items: center; background: rgba(2,6,23,0.55); padding: var(--space-lg); backdrop-filter: blur(10px); }
    .amana-modal-panel { width: min(100%, 36rem); background: var(--surface-elevated); border-radius: var(--radius-2xl); padding: var(--space-lg); box-shadow: var(--shadow-strong); }
    .amana-tabs {
        display: flex;
        flex-direction: column;
        width: 100%;
        margin-bottom: 1.5rem;
        background: var(--surface-elevated);
        border: 1px solid var(--border-subtle);
        border-radius: var(--radius-xl);
        overflow: hidden;
    }
    .amana-tabs-header {
        display: flex;
        border-bottom: 1px solid var(--border-subtle);
        background: var(--surface-muted);
        overflow-x: auto;
        -webkit-overflow-scrolling: touch;
        scrollbar-width: none;
    }
    .amana-tabs-header::-webkit-scrollbar {
        display: none;
    }
    .amana-tab-button {
        flex: 1;
        text-align: center;
        padding: 0.85rem 1.25rem;
        font-weight: 700;
        font-size: var(--text-sm);
        color: var(--text-secondary);
        background: transparent;
        border: none;
        border-bottom: 2px solid transparent;
        cursor: pointer;
        transition: all 0.2s ease;
        white-space: nowrap;
    }
    .amana-tab-button:hover {
        color: var(--text-primary);
        background: rgba(0, 0, 0, 0.02);
    }
    .amana-tab-button.active {
        color: var(--color-primary);
        border-bottom-color: var(--color-primary);
        background: var(--surface-elevated);
    }
    .amana-tabs-content {
        padding: 1.25rem;
        background: var(--surface-elevated);
        min-height: 0;
    }
    .amana-tab-panel {
        animation: amanaFadeIn 0.2s ease-out;
    }
    .amana-accordion {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        width: 100%;
        margin-bottom: 1.5rem;
    }
    .amana-accordion-item {
        background: var(--surface-elevated);
        border: 1px solid var(--border-subtle);
        border-radius: var(--radius-lg);
        overflow: hidden;
        transition: all 0.2s ease;
    }
    .amana-accordion-item:focus-within {
        border-color: var(--color-primary-soft);
    }
    .amana-accordion-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        width: 100%;
        padding: 1rem 1.25rem;
        background: var(--surface-muted);
        border: none;
        color: var(--text-primary);
        font-weight: 700;
        font-size: var(--text-sm);
        cursor: pointer;
        text-align: right;
        transition: background 0.2s ease;
    }
    .amana-accordion-header:hover {
        background: color-mix(in srgb, var(--surface-muted) 90%, var(--color-primary-soft));
    }
    .amana-accordion-title {
        flex: 1;
    }
    .amana-accordion-chevron {
        width: 1.25rem;
        height: 1.25rem;
        transition: transform 0.2s ease;
        color: var(--text-secondary);
        flex-shrink: 0;
    }
    .amana-accordion-content {
        padding: 1.25rem;
        background: var(--surface-elevated);
        border-top: 1px solid var(--border-subtle);
        animation: amanaFadeIn 0.2s ease-out;
    }
    .rotate-180 {
        transform: rotate(180deg);
    }
    .amana-collapse-section {
        display: flex;
        flex-direction: column;
        width: 100%;
        margin-bottom: 1.5rem;
        background: var(--surface-elevated);
        border: 1px solid var(--border-subtle);
        border-radius: var(--radius-xl);
        overflow: hidden;
    }
    .amana-collapse-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.75rem 1.25rem;
        background: var(--surface-muted);
        border-bottom: 1px solid var(--border-subtle);
        cursor: pointer;
        user-select: none;
    }
    .amana-collapse-header:hover {
        background: color-mix(in srgb, var(--surface-muted) 90%, var(--color-primary-soft));
    }
    .amana-collapse-title-wrapper {
        flex: 1;
        display: flex;
        align-items: center;
        min-width: 0;
    }
    .amana-collapse-title-wrapper > * {
        margin: 0 !important;
    }
    .amana-collapse-chevron {
        width: 1.25rem;
        height: 1.25rem;
        transition: transform 0.2s ease;
        color: var(--text-secondary);
        flex-shrink: 0;
        margin-inline-start: 1rem;
    }
    .amana-collapse-body {
        padding: 1.25rem;
        background: var(--surface-elevated);
        animation: amanaFadeIn 0.2s ease-out;
    }
    @keyframes amanaFadeIn {
        from { opacity: 0; transform: translateY(2px); }
        to { opacity: 1; transform: translateY(0); }
    }
    .amana-card-top { display: flex; align-items: center; justify-content: space-between; gap: var(--space-sm); margin-bottom: var(--space-xs); }
    .amana-card-meta { color: var(--text-secondary); font-size: var(--text-sm); }
    .amana-card-action { color: var(--color-accent); font-weight: 900; text-decoration: none; margin-top: auto; }
    .amana-card-density-compact { padding: var(--space-md); }
    .amana-card-density-spacious { padding: var(--space-xl); }
    .amana-badge { display: inline-flex; align-items: center; width: fit-content; gap: 0.35rem; border: 1px solid var(--border-subtle); border-radius: 999px; padding: 0.38rem 0.78rem; font-size: var(--text-sm); font-weight: 900; background: color-mix(in srgb, var(--surface-muted) 78%, transparent); color: var(--text-primary); box-shadow: var(--shadow-soft); }
    .amana-badge-success { border-color: rgba(22,163,74,0.35); color: var(--color-success); }
    .amana-badge-warning { border-color: rgba(202,138,4,0.35); color: var(--color-warning); }
    .amana-badge-danger { border-color: rgba(220,38,38,0.35); color: var(--color-danger); }
    .amana-kpi { display: flex; flex-direction: column; gap: 0.35rem; padding: clamp(1.25rem, 3vw, 2rem); border: 1px solid var(--border-subtle); border-radius: var(--radius-xl); background: linear-gradient(180deg, var(--surface-elevated), var(--surface-muted)); box-shadow: var(--shadow-soft); transition: var(--transition-fast); }
    .amana-kpi:hover { border-color: color-mix(in srgb, var(--color-primary) 32%, var(--border-subtle)); transform: translateY(-2px); }
    .amana-kpi-label { order: -1; text-transform: uppercase; font-size: var(--text-xs); font-weight: 800; color: var(--text-secondary); letter-spacing: 0.05em; }
    .amana-kpi-value { font-size: clamp(2.25rem, 5vw, 4rem); line-height: 1; font-weight: 950; font-feature-settings: "tnum"; color: var(--text-primary); }
    .amana-kpi-trend { color: var(--color-success); font-weight: 700; }
    .amana-logo-cloud { display: grid; gap: var(--space-md); padding-block: var(--space-lg); }
    .amana-logo-row { display: flex; flex-wrap: wrap; gap: var(--space-md); align-items: center; color: var(--text-secondary); }
    .amana-testimonial { margin: 0; display: grid; gap: var(--space-md); border: 1px solid var(--border-subtle); border-radius: var(--radius-xl); padding: var(--space-lg); background: var(--surface-elevated); box-shadow: var(--shadow-soft); }
    .amana-testimonial blockquote { margin: 0; font-size: var(--text-lg); color: var(--text-primary); }
    .amana-testimonial figcaption { display: grid; gap: 0.1rem; color: var(--text-secondary); }
    .amana-timeline { display: flex; flex-direction: column; gap: var(--space-lg); border-inline-start: 2px solid var(--border-subtle); list-style: none; margin: 0; padding: 0; padding-inline-start: 2rem; position: relative; }
    .amana-timeline-item { position: relative; padding: var(--space-lg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xl); background: var(--surface-elevated); transition: var(--transition-fast); }
    .amana-timeline-item::before { content: ""; position: absolute; top: 1.85rem; width: 0.75rem; height: 0.75rem; border-radius: 50%; background: var(--color-primary); border: 2px solid var(--bg-secondary); box-shadow: 0 0 0 3px var(--color-primary-soft); z-index: 2; }
    [dir="rtl"] .amana-timeline-item::before { right: -2.42rem; }
    [dir="ltr"] .amana-timeline-item::before { left: -2.42rem; }
    .amana-empty-state { display: grid; place-items: center; text-align: center; gap: var(--space-md); min-height: 18rem; border: 1px dashed var(--border-subtle); border-radius: var(--radius-xl); padding: var(--space-xl); background: var(--surface-muted); }
    .amana-split { display: grid; grid-template-columns: minmax(0, 1fr) minmax(16rem, 0.85fr); gap: var(--space-xl); align-items: center; }
    .amana-cluster { display: flex; flex-wrap: wrap; gap: var(--space-md); align-items: center; }
    .amana-sidebar { border: 1px solid var(--border-subtle); border-radius: var(--radius-xl); background: var(--surface-elevated); padding: var(--space-lg); box-shadow: var(--shadow-soft); }
    .amana-navbar-sticky { position: sticky; top: 0; z-index: 20; background: color-mix(in srgb, var(--surface-base) 78%, transparent); backdrop-filter: blur(12px); border-bottom: 1px solid var(--border-subtle); }
    .amana-navbar-glass { background: color-mix(in srgb, var(--surface-base) 60%, transparent) !important; backdrop-filter: blur(14px); border: 1px solid var(--border-subtle) !important; border-radius: 999px; margin-top: 1rem; padding: 0.75rem 2rem !important; box-shadow: var(--shadow-soft); }
    .amana-navbar-elegant { border-bottom: 2px solid var(--color-primary); padding-block: 1.5rem !important; font-family: var(--font-heading); }
    .amana-navbar-floating { position: fixed; top: 1rem; left: 50%; transform: translateX(-50%); width: min(90%, var(--wide-width)) !important; z-index: 100; background: var(--surface-elevated) !important; border: 1px solid var(--border-subtle) !important; border-radius: 999px; padding: 0.75rem 2.5rem !important; box-shadow: var(--shadow-floating); }
    .amana-slides { position: relative; overflow: hidden; width: 100%; border-radius: var(--radius-2xl); background: var(--surface-muted); border: 1px solid var(--border-subtle); display: flex; flex-direction: column; justify-content: center; }
    .amana-slides-inner { position: relative; width: 100%; height: 100%; display: grid; grid-template-columns: 1fr; grid-template-rows: 1fr; }
    .amana-slides-inner > * { grid-area: 1 / 1 / 2 / 2; width: 100%; height: 100%; display: flex; flex-direction: column; justify-content: center; padding: 3rem; }
    .amana-slides-arrow { position: absolute; top: 50%; transform: translateY(-50%); background: rgba(255,255,255,0.15); border: 1px solid rgba(255,255,255,0.1); color: var(--text-primary); width: 2.75rem; height: 2.75rem; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer; backdrop-filter: blur(8px); transition: all 0.2s ease; z-index: 10; font-size: 1.2rem; }
    .amana-slides-arrow:hover { background: var(--color-primary); color: white; transform: translateY(-50%) scale(1.1); }
    .amana-slides-arrow.prev { left: 1rem; }
    .amana-slides-arrow.next { right: 1rem; }
    .amana-slides-dots { position: absolute; bottom: 1.25rem; left: 50%; transform: translateX(-50%); display: flex; gap: 0.5rem; z-index: 10; }
    .amana-slides-dot { width: 0.5rem; height: 0.5rem; border-radius: 50%; background: rgba(255,255,255,0.3); border: 1px solid rgba(0,0,0,0.1); cursor: pointer; transition: all 0.2s ease; }
    .amana-slides-dot.active { background: var(--color-primary); width: 1.25rem; border-radius: 999px; }
    .amana-page { min-height: 100vh; background: var(--bg-secondary); color: var(--text-primary); overflow-x: hidden; }
    .amana-runtime-shell { display: block; width: 100%; max-width: 100%; min-height: 100vh; margin: 0; padding: 0; overflow-x: hidden; }
    .amana-runtime-shell > :not(script):not(style):not(.amana-state-scope), .amana-runtime-shell > .amana-state-scope > :not(script):not(style) { max-width: 100%; }
    .dg-canvas-width-full .amana-container { width: 100%; max-width: none; }
    .dg-canvas-width-wide .amana-container { width: min(100% - 2rem, var(--wide-width)); }
    .dg-canvas-width-readable .amana-container { width: min(100% - 2rem, var(--readable-width)); }
    .dg-layout-split-diagonal,
    .dg-layout-asymmetric { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, minmax(0, 1.1fr) minmax(16rem, 0.85fr))); align-items: center; gap: var(--component-gap, var(--dg-gap, clamp(1.5rem, 5vw, 4rem))); }
    :where(.dg-layout-split-diagonal, .dg-layout-asymmetric, .dg-layout-editorial, .dg-layout-dashboard-shell, .dg-layout-magazine, .dg-layout-bento, .dg-layout-command-center, .dg-layout-showcase-rail) > .amana-container { grid-column: 1 / -1; width: min(100% - 2rem, var(--content-width)); }
    .dg-layout-centered { text-align: center; justify-items: center; }
    .dg-layout-editorial { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, minmax(14rem, 0.55fr) minmax(0, 1fr))); gap: var(--component-gap, var(--dg-gap, clamp(2rem, 6vw, 5rem))); align-items: start; }
    .dg-layout-dashboard-shell,
    :where(.dg-layout-dashboard-shell) .amana-runtime-shell > :not(script):not(style):not(.amana-state-scope),
    :where(.dg-layout-dashboard-shell) .amana-runtime-shell > .amana-state-scope > :not(script):not(style) { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, minmax(14rem, 18rem) minmax(0, 1fr))); gap: var(--component-gap, var(--dg-gap, var(--space-lg))); align-items: start; }
    :where(.dg-layout-dashboard-shell) { height: 100dvh; overflow: hidden; }
    .amana-state-scope { height: 100%; min-height: 0; width: 100%; max-width: 100%; display: flex; flex-direction: column; }
    :where(.dg-layout-dashboard-shell) .amana-state-scope { height: 100%; min-height: 0; overflow: hidden; }
    .amana-state-scope > .app-shell { flex: 1 1 auto; width: 100%; min-height: 0; }
    :where(.dg-layout-dashboard-shell) .amana-runtime-shell { height: 100%; min-height: 0; overflow: hidden; }
    :where(.dg-layout-dashboard-shell) .app-shell { height: 100%; min-height: 0; overflow: hidden; }
    :where(.dg-layout-dashboard-shell) .side-rail { height: 100%; min-height: 0; overflow: auto; }
    :where(.dg-layout-dashboard-shell) .dashboard-main { height: 100%; min-height: 0; overflow: auto; }
    :where(.dg-layout-dashboard-shell) .panel { display: flex; flex-direction: column; min-height: 0; }
    :where(.dg-layout-dashboard-shell) .amana-resource { display: flex; flex-direction: column; min-height: 0; }
    :where(.dg-layout-dashboard-shell) .dashboard-grid,
    :where(.dg-layout-dashboard-shell) .customers-container,
    :where(.dg-layout-dashboard-shell) .tickets-container,
    :where(.dg-layout-dashboard-shell) .agents-container,
    :where(.dg-layout-dashboard-shell) .reports-layout,
    :where(.dg-layout-dashboard-shell) .settings-layout,
    :where(.dg-layout-dashboard-shell) .ticket-detail-grid { min-height: 0; }
    :where(.dg-layout-dashboard-shell) .amana-resource-list { max-height: clamp(14rem, 38vh, 22rem); overflow: auto; }
    :where(.dg-layout-dashboard-shell) .agent-status-list,
    :where(.dg-layout-dashboard-shell) .urgent-list { max-height: clamp(10rem, 28vh, 18rem); overflow: auto; }
    .dg-layout-magazine { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, repeat(12, minmax(0, 1fr)))); gap: var(--component-gap, var(--dg-gap, var(--space-lg))); }
    .dg-layout-magazine > * { grid-column: span 6; }
    .dg-rhythm-compact { gap: var(--space-sm); padding-block: var(--space-md); }
    .dg-rhythm-balanced { gap: var(--space-lg); }
    .dg-rhythm-spacious { gap: var(--space-xl); padding-block: var(--space-2xl); }
    .dg-rhythm-dramatic { gap: clamp(2rem, 7vw, 6rem); padding-block: clamp(4rem, 10vw, 8rem); }
    .dg-surface-layered,
    .dg-surface-glass-layered,
    .dg-visual-surface-layered { position: relative; isolation: isolate; background: var(--surface-elevated); border: 1px solid var(--border-subtle); box-shadow: var(--shadow-floating); }
    .dg-surface-glass,
    .dg-visual-surface-glass { background: var(--glass-bg); border: 1px solid var(--glass-border); backdrop-filter: var(--glass-blur); -webkit-backdrop-filter: var(--glass-blur); }
    .dg-surface-custom,
    .dg-visual-surface-custom,
    .dg-component-variant-custom { background: var(--custom-bg, var(--surface-elevated)); color: var(--custom-text, var(--text-primary)); border-color: var(--custom-border, var(--border-subtle)); border-radius: var(--custom-radius, inherit); box-shadow: var(--custom-shadow, var(--shadow-soft)); }
    .dg-gradient-custom,
    .dg-visual-gradient-custom { background: var(--custom-gradient, var(--gradient-primary)); }
    .dg-mode-light,
    .dg-visual-mode-light,
    .dg-mode-day,
    .dg-visual-mode-day { color-scheme: light; --bg-secondary: #f8fafc; --surface-base: #ffffff; --surface-muted: #f8fafc; --surface-elevated: #ffffff; --text-primary: #0f172a; --text-secondary: #475569; --border-subtle: rgba(15,23,42,0.12); }
    .dg-mode-dark,
    .dg-visual-mode-dark,
    .dg-mode-night,
    .dg-visual-mode-night { color-scheme: dark; --bg-secondary: #050816; --surface-base: #0b1020; --surface-muted: #111827; --surface-elevated: #151d31; --text-primary: #f8fafc; --text-secondary: #cbd5e1; --border-subtle: rgba(148,163,184,0.18); }
    .dg-gradient-mesh-aurora,
    .dg-visual-gradient-mesh-aurora { background: radial-gradient(circle at 12% 18%, rgba(6,182,212,0.28), transparent 32%), radial-gradient(circle at 86% 20%, rgba(79,70,229,0.26), transparent 34%), linear-gradient(135deg, var(--surface-base), var(--surface-muted)); }
    .dg-gradient-mesh-cyan-indigo,
    .dg-visual-gradient-mesh-cyan-indigo { background: radial-gradient(circle at top left, rgba(34,211,238,0.30), transparent 35%), radial-gradient(circle at bottom right, rgba(79,70,229,0.28), transparent 38%), var(--surface-base); }
    .dg-gradient-spotlight,
    .dg-visual-gradient-spotlight { background: radial-gradient(circle at 50% 0%, var(--color-primary-soft), transparent 42%), var(--surface-base); }
    .dg-shape-diagonal-cut,
    .dg-visual-shape-diagonal-cut { clip-path: polygon(0 0, 100% 0, 96% 100%, 0 92%); }
    .dg-shape-soft-blob,
    .dg-visual-shape-soft-blob { border-radius: 32px 18px 42px 22px; }
    .dg-shape-squircle,
    .dg-visual-shape-squircle,
    .dg-component-shape-squircle { border-radius: 28% 22% 30% 20%; }
    .dg-shape-ticket,
    .dg-visual-shape-ticket,
    .dg-component-shape-ticket { clip-path: polygon(0 0, 100% 0, 100% calc(100% - 1rem), calc(100% - 1rem) 100%, 0 100%); }
    .dg-component-shape-pill { border-radius: 999px; }
    .dg-component-density-compact { padding: var(--space-sm); gap: var(--space-sm); }
    .dg-component-density-spacious { padding: var(--space-xl); gap: var(--space-lg); }
    .dg-component-chrome-minimal { border-color: transparent; box-shadow: none; background: transparent; }
    .dg-component-chrome-bold { border-width: 2px; box-shadow: var(--custom-shadow, var(--shadow-floating)); }
    .dg-visual-border-glow-subtle { border-color: color-mix(in srgb, var(--color-accent) 38%, var(--border-subtle)); box-shadow: var(--shadow-soft), var(--glow-accent); }
    .dg-flow-sectional > * + * { margin-top: var(--space-2xl); }
    .dg-flow-immersive { min-height: 100vh; display: grid; align-content: center; }
    .dg-flow-dashboard { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, minmax(14rem, 18rem) minmax(0, 1fr))); gap: var(--component-gap, var(--dg-gap, var(--space-xl))); }
    .dg-density-compact { --space-md: 0.75rem; --space-lg: 1rem; --space-xl: 1.35rem; }
    .dg-density-comfortable { --space-md: 1rem; --space-lg: 1.5rem; --space-xl: 2rem; }
    .dg-density-spacious { --space-md: 1.25rem; --space-lg: 2rem; --space-xl: 3rem; }
    .dg-align-start { justify-items: start; text-align: start; }
    .dg-align-center { justify-items: center; text-align: center; }
    .dg-align-end { justify-items: end; text-align: end; }
    .dg-focus-path-z .amana-card:nth-child(2),
    .dg-focus-path-z > *:nth-child(2) { transform: translateY(1rem); }
    .dg-focus-path-radial { position: relative; }
    .dg-focus-path-radial::before { content: ""; position: absolute; inset: 10%; border-radius: 999px; background: radial-gradient(circle, var(--color-primary-soft), transparent 62%); opacity: 0.42; pointer-events: none; z-index: -1; }
    .dg-layout-bento { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, repeat(6, minmax(0, 1fr)))); gap: var(--component-gap, var(--dg-gap, var(--space-lg))); }
    .dg-layout-bento > * { grid-column: span 2; }
    .dg-layout-bento > *:first-child { grid-column: span 4; grid-row: span 2; }
    .dg-layout-command-center { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, 0.9fr 1.2fr 0.9fr)); gap: var(--component-gap, var(--dg-gap, var(--space-lg))); align-items: stretch; }
    .dg-layout-showcase-rail { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, minmax(0, 0.95fr) minmax(16rem, 0.45fr))); gap: var(--component-gap, var(--dg-gap, var(--space-lg))); }
    .dg-layout-masonry { columns: 3 18rem; column-gap: var(--space-lg); }
    .dg-layout-masonry > * { break-inside: avoid; margin-bottom: var(--space-lg); }
    .dg-palette-mono-luxe { --color-primary: #111827; --color-accent: #64748b; --color-primary-soft: rgba(100,116,139,0.14); }
    .dg-palette-neon-lab { --color-primary: #7c3aed; --color-accent: #06b6d4; --color-primary-soft: rgba(124,58,237,0.18); }
    .dg-palette-earth-tech { --color-primary: #0f766e; --color-accent: #a16207; --color-primary-soft: rgba(15,118,110,0.16); }
    .dg-colorway-calm-saas { --color-primary: #2563eb; --color-accent: #14b8a6; --color-primary-soft: rgba(37,99,235,0.13); }
    .dg-colorway-editorial-ink { --color-primary: #18181b; --color-accent: #be123c; --color-primary-soft: rgba(190,18,60,0.12); }
    .dg-colorway-cyber-cyan { --color-primary: #0891b2; --color-accent: #a78bfa; --color-primary-soft: rgba(8,145,178,0.18); }
    .dg-art-minimal-editorial { letter-spacing: 0; }
    .dg-art-cinematic-product { box-shadow: inset 0 0 0 1px var(--border-subtle), var(--shadow-strong); }
    .dg-art-technical-blueprint { background-image: linear-gradient(var(--border-subtle) 1px, transparent 1px), linear-gradient(90deg, var(--border-subtle) 1px, transparent 1px); background-size: 32px 32px; }
    .dg-motif-orbit { position: relative; overflow: hidden; }
    .dg-motif-orbit::after { content: ""; position: absolute; width: 18rem; aspect-ratio: 1; border: 1px solid var(--border-subtle); border-radius: 999px; inset-inline-end: -5rem; top: -5rem; pointer-events: none; }
    .dg-motif-grid { background-image: linear-gradient(var(--border-subtle) 1px, transparent 1px), linear-gradient(90deg, var(--border-subtle) 1px, transparent 1px); background-size: 28px 28px; }
    .dg-lighting-rim { box-shadow: inset 0 1px 0 rgba(255,255,255,0.18), var(--shadow-floating); }
    .dg-lighting-spot { background: radial-gradient(circle at 50% 0%, var(--color-primary-soft), transparent 45%), var(--surface-base); }
    .dg-texture-noise { position: relative; isolation: isolate; }
    .dg-texture-noise::before { content: ""; position: absolute; inset: 0; opacity: var(--dg-texture-opacity, 0.06); pointer-events: none; background-image: repeating-linear-gradient(45deg, rgba(255,255,255,0.16) 0 1px, transparent 1px 4px); z-index: -1; }
    .dg-texture-paper { background-image: linear-gradient(rgba(255,255,255,0.05), rgba(255,255,255,0.02)); }
    .dg-frame-device { border-radius: 28px; border: 10px solid color-mix(in srgb, var(--text-primary) 82%, transparent); box-shadow: var(--shadow-strong); }
    .dg-frame-browser { border-top: 2rem solid color-mix(in srgb, var(--surface-muted) 88%, var(--text-primary)); border-radius: var(--radius-xl); }
    .dg-brand-voice-premium,
    .dg-brand-personality-premium { --shadow-soft: 0 18px 45px -28px rgba(2,6,23,0.7); }
    .dg-brand-voice-playful,
    .dg-brand-personality-playful { --radius-xl: 28px; --radius-soft: 22px; }
    .dg-brand-trust-high { border-color: color-mix(in srgb, var(--color-success) 28%, var(--border-subtle)); }
    .dg-feedback-tactile :is(a, button, .amana-card) { transition: transform 160ms ease, box-shadow 160ms ease; }
    .dg-feedback-tactile :is(a, button, .amana-card):active { transform: translateY(1px) scale(0.99); }
    .dg-affordance-obvious :is(a, button) { box-shadow: var(--glow-primary); }
    .dg-cursor-precise { cursor: crosshair; }
    [style*="--state-hover-bg"]:hover { background: var(--state-hover-bg) !important; color: var(--state-hover-text, var(--custom-text, var(--text-primary))) !important; box-shadow: var(--state-hover-shadow, var(--custom-shadow, var(--shadow-floating))) !important; }
    [style*="--state-focus-ring"]:focus-visible,
    [style*="--state-focus-ring"] :focus-visible { outline: 3px solid var(--state-focus-ring) !important; outline-offset: 3px; }
    .dg-a11y-contrast-enhanced { --text-secondary: color-mix(in srgb, var(--text-primary) 76%, var(--bg-secondary)); }
    .dg-focus-visible-strong :focus-visible { outline: max(2px, var(--dg-focus-strength, 3px)) solid var(--color-accent); outline-offset: 3px; }
    .dg-type-scale-dramatic h1,
    .dg-type-scale-dramatic h2 { font-size: clamp(3rem, 9vw, 7rem); line-height: 0.95; }
    .dg-type-scale-editorial h1,
    .dg-type-scale-editorial h2 { font-size: clamp(2.4rem, 6vw, 5rem); line-height: 1.02; max-width: 10ch; }
    .dg-type-align-center { text-align: center; }
    .dg-type-align-start { text-align: start; }
    .dg-type-contrast-high h1,
    .dg-type-contrast-high h2 { color: var(--text-primary); }
    .dg-type-measure-tight { --dg-type-measure: 48ch; }
    .dg-type-measure-readable { --dg-type-measure: 68ch; }
    .dg-type-measure-wide { --dg-type-measure: 82ch; }
    .dg-type-measure-tight p,
    .dg-type-measure-readable p,
    .dg-type-measure-wide p { max-width: var(--dg-type-measure); }
    .dg-type-hierarchy-strong h1,
    .dg-type-hierarchy-strong h2,
    .dg-type-hierarchy-strong h3 { font-weight: 850; }
    .dg-type-tone-technical { font-family: var(--font-mono); }
    .dg-type-tone-editorial h1,
    .dg-type-tone-editorial h2 { font-family: Georgia, "Times New Roman", serif; font-weight: 700; }
    .dg-motion-stagger-up > * { animation: dgFadeUp var(--dg-motion-speed, 560ms) ease both; }
    .dg-motion-stagger-up > *:nth-child(2) { animation-delay: 90ms; }
    .dg-motion-stagger-up > *:nth-child(3) { animation-delay: 180ms; }
    .dg-motion-fade { animation: dgFadeUp var(--dg-motion-speed, 520ms) ease both; }
    .dg-hover-lift-glow:hover,
    .dg-hover-lift:hover { transform: translateY(-4px); box-shadow: var(--shadow-floating), var(--glow-primary); }
    .dg-hover-scale:hover { transform: scale(1.015); }
    .dg-hover-lift-glow,
    .dg-hover-lift,
    .dg-hover-scale { transition: var(--transition-smooth); will-change: transform; }
    .dg-reveal-blur { animation: dgBlurIn var(--dg-motion-speed, 640ms) ease both; }
    .dg-reveal-clip { animation: dgClipIn var(--dg-motion-speed, 720ms) ease both; }
    .dg-rsp-mobile-stacked { --dg-mobile-layout: stacked; }
    .dg-rsp-mobile-scroll-snap { scroll-snap-type: x mandatory; overflow-x: auto; }
    .dg-rsp-mobile-scroll-snap > * { scroll-snap-align: start; }
    .dg-rsp-collapse-stack { --dg-collapse: stack; }
    .dg-rsp-columns-adaptive { grid-template-columns: repeat(auto-fit, minmax(var(--grid-min, 16rem), 1fr)); }
    @keyframes dgFadeUp { from { opacity: 0; transform: translateY(18px); } to { opacity: 1; transform: translateY(0); } }
    @keyframes dgBlurIn { from { opacity: 0; filter: blur(14px); transform: translateY(12px); } to { opacity: 1; filter: blur(0); transform: translateY(0); } }
    @keyframes dgClipIn { from { opacity: 0; clip-path: inset(18% 0 0 0); } to { opacity: 1; clip-path: inset(0 0 0 0); } }
    @media (min-width: 1201px) {
      [style*="--bp-desktop-columns"] { --component-columns: var(--bp-desktop-columns) !important; --dg-columns: var(--bp-desktop-columns) !important; grid-template-columns: var(--bp-desktop-columns) !important; }
      [style*="--bp-desktop-padding"] { --component-padding: var(--bp-desktop-padding) !important; }
      [style*="--bp-desktop-gap"] { --component-gap: var(--bp-desktop-gap) !important; --dg-gap: var(--bp-desktop-gap) !important; gap: var(--bp-desktop-gap) !important; }
    }
    @media (max-width: 1200px) and (min-width: 901px) {
      [style*="--bp-laptop-columns"] { --component-columns: var(--bp-laptop-columns) !important; --dg-columns: var(--bp-laptop-columns) !important; grid-template-columns: var(--bp-laptop-columns) !important; }
      [style*="--bp-laptop-padding"] { --component-padding: var(--bp-laptop-padding) !important; }
      [style*="--bp-laptop-gap"] { --component-gap: var(--bp-laptop-gap) !important; --dg-gap: var(--bp-laptop-gap) !important; gap: var(--bp-laptop-gap) !important; }
    }
    @media (max-width: 900px) and (min-width: 641px) {
      [style*="--bp-tablet-columns"] { --component-columns: var(--bp-tablet-columns) !important; --dg-columns: var(--bp-tablet-columns) !important; grid-template-columns: var(--bp-tablet-columns) !important; }
      [style*="--bp-tablet-padding"] { --component-padding: var(--bp-tablet-padding) !important; }
      [style*="--bp-tablet-gap"] { --component-gap: var(--bp-tablet-gap) !important; --dg-gap: var(--bp-tablet-gap) !important; gap: var(--bp-tablet-gap) !important; }
    }
    @media (max-width: 640px) {
      [style*="--bp-mobile-columns"] { --component-columns: var(--bp-mobile-columns) !important; --dg-columns: var(--bp-mobile-columns) !important; grid-template-columns: var(--bp-mobile-columns) !important; }
      [style*="--bp-mobile-padding"] { --component-padding: var(--bp-mobile-padding) !important; }
      [style*="--bp-mobile-gap"] { --component-gap: var(--bp-mobile-gap) !important; --dg-gap: var(--bp-mobile-gap) !important; gap: var(--bp-mobile-gap) !important; }
    }
    @media (max-width: 720px) {
      .amana-navbar { align-items: flex-start; flex-direction: column; gap: var(--space-md); }
      .amana-navlinks { justify-content: flex-start; width: 100%; gap: var(--space-xs); }
      .amana-navlinks a { padding: 0.38rem 0.68rem; font-size: var(--text-xs); }
      .amana-hero { padding: var(--space-lg); }
      .amana-hero h1 { font-size: 2.25rem; }
      .amana-card { padding: var(--space-md); }
      .amana-timeline { padding-inline-start: 1.5rem; }
      [dir="rtl"] .amana-timeline-item::before { right: -1.92rem; }
      [dir="ltr"] .amana-timeline-item::before { left: -1.92rem; }
      .amana-split,
      .dg-flow-dashboard,
      .dg-layout-split-diagonal,
      .dg-layout-asymmetric,
      .dg-layout-editorial,
      .dg-layout-dashboard-shell,
      :where(.dg-layout-dashboard-shell) .amana-runtime-shell > :not(script):not(style):not(.amana-state-scope),
      :where(.dg-layout-dashboard-shell) .amana-runtime-shell > .amana-state-scope > :not(script):not(style),
      .dg-layout-magazine,
      .dg-layout-bento,
      .dg-layout-command-center,
      .dg-layout-showcase-rail { grid-template-columns: 1fr; }
      .dg-layout-magazine > *,
      .dg-layout-bento > *,
      .dg-layout-bento > *:first-child { grid-column: auto; grid-row: auto; }
      /* Mobile Content Density & Section Compaction */
      :where(.dg-layout-dashboard-shell) .reports-container {
        padding: 1.05rem !important;
        gap: 1rem !important;
      }
      :where(.dg-layout-dashboard-shell) .dashboard-grid,
      :where(.dg-layout-dashboard-shell) .settings-layout,
      :where(.dg-layout-dashboard-shell) .ticket-detail-grid,
      :where(.dg-layout-dashboard-shell) .reports-grid,
      :where(.dg-layout-dashboard-shell) .reports-secondary {
        gap: 0.85rem !important;
      }
      :where(.dg-layout-dashboard-shell) .dashboard-main-col,
      :where(.dg-layout-dashboard-shell) .dashboard-side-col {
        gap: 0.85rem !important;
      }
      :where(.dg-layout-dashboard-shell) .panel {
        padding: 1rem !important;
        border-radius: 12px !important;
      }
      :where(.dg-layout-dashboard-shell) .panel-header {
        margin-bottom: 0.85rem !important;
      }
      :where(.dg-layout-dashboard-shell) .kpi-row {
        padding: 1rem !important;
        gap: 0.75rem !important;
      }
      :where(.dg-layout-dashboard-shell) .dash-header {
        padding: 1rem !important;
      }
      :where(.dg-layout-dashboard-shell) .amana-kpi {
        padding: 0.85rem !important;
        border-radius: 10px !important;
        gap: 0.25rem !important;
      }
      :where(.dg-layout-dashboard-shell) .amana-kpi-value {
        font-size: 1.8rem !important;
      }
      :where(.dg-layout-dashboard-shell) .kpi-wide {
        padding: 0.75rem 0.85rem !important;
        border-radius: 10px !important;
      }
      :where(.dg-layout-dashboard-shell) .kpi-wide-value {
        font-size: 1.35rem !important;
        margin-bottom: 0.15rem !important;
      }
      :where(.dg-layout-dashboard-shell) .kpi-wide-label {
        font-size: 0.65rem !important;
        margin-bottom: 0.2rem !important;
      }
      :where(.dg-layout-dashboard-shell) .performance-table {
        max-height: 280px !important;
        overflow-y: auto !important;
      }
      :where(.dg-layout-dashboard-shell) .kb-list {
        max-height: 280px !important;
        overflow-y: auto !important;
      }
      :where(.dg-layout-dashboard-shell) .csat-list {
        max-height: 280px !important;
        overflow-y: auto !important;
      }
      :where(.dg-layout-dashboard-shell) .agent-status-list {
        max-height: 240px !important;
        overflow-y: auto !important;
      }
      :where(.dg-layout-dashboard-shell) .urgent-list {
        max-height: 240px !important;
        overflow-y: auto !important;
      }
      :where(.dg-layout-dashboard-shell) .agent-status-row {
        padding: 0.4rem 0.5rem !important;
      }
      :where(.dg-layout-dashboard-shell) .kb-row {
        padding: 0.5rem 0.25rem !important;
      }
      :where(.dg-layout-dashboard-shell) .perf-row {
        padding: 0.4rem 0.25rem !important;
      }
      :where(.dg-layout-dashboard-shell) .csat-row {
        padding: 0.6rem !important;
      }
      :where(.dg-layout-dashboard-shell) .amana-resource-item {
        padding: 0.65rem !important;
        gap: 0.4rem !important;
      }
      :where(.dg-layout-dashboard-shell) .amana-resource-list {
        gap: 0.5rem !important;
      }
      :where(.dg-layout-dashboard-shell) .report-panel-header {
        margin-bottom: 0.85rem !important;
        padding-bottom: 0.6rem !important;
      }
      :where(.dg-layout-dashboard-shell) .report-panel-title {
        font-size: 0.88rem !important;
      }
      :where(.dg-layout-dashboard-shell) .volume-chart {
        height: 130px !important;
      }
      :where(.dg-layout-dashboard-shell) .chart-bars-large {
        height: 110px !important;
      }
      :where(.dg-layout-dashboard-shell) .bar-wrap-lg {
        height: 80px !important;
      }
      :where(.dg-layout-dashboard-shell) .chart-wrap {
        min-height: 140px !important;
      }
      :where(.dg-layout-dashboard-shell) .chart-bars {
        height: 100px !important;
      }
      :where(.dg-layout-dashboard-shell) .bar-wrap {
        height: 70px !important;
      }
    }
    @media (prefers-reduced-motion: reduce) {
      .dg-reduce-motion-auto *,
      .dg-reduce-motion-strict *,
      .dg-motion-stagger-up > *,
      .dg-motion-fade,
      .dg-reveal-blur,
      .dg-reveal-clip { animation: none !important; transition: none !important; }
    }
"#;
