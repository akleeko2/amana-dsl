// src/codegen/express.rs
use crate::ast::DesignBlock;
use crate::semantic::ir::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;


pub(crate) mod base_css;
pub(crate) mod tokens;
pub(crate) mod theme;
pub(crate) mod hooks;
pub(crate) mod static_files;

fn clean_comments(content: &str) -> String {
    let mut clean = String::new();
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if in_line_comment {
            if chars[i] == '\n' {
                in_line_comment = false;
                clean.push('\n');
            }
        } else if in_block_comment {
            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '/' {
                in_block_comment = false;
                i += 1;
            }
        } else {
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                in_line_comment = true;
                i += 1;
            } else if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
                in_block_comment = true;
                i += 1;
            } else {
                clean.push(chars[i]);
            }
        }
        i += 1;
    }
    clean
}

fn extract_exports(clean: &str) -> Option<String> {
    if let Some(idx) = clean.find("module.exports")
        && let Some(start_brace) = clean[idx..].find('{')
    {
        let body_start = idx + start_brace + 1;
        let mut depth = 1;
        let mut end_idx = body_start;
        let chars: Vec<char> = clean[body_start..].chars().collect();
        for (offset, c) in chars.iter().enumerate() {
            if *c == '{' {
                depth += 1;
            } else if *c == '}' {
                depth -= 1;
                if depth == 0 {
                    end_idx = body_start + offset;
                    break;
                }
            }
        }
        if depth == 0 {
            return Some(clean[body_start..end_idx].to_string());
        }
    }
    None
}

fn validate_exports_body(body: &str) -> Result<(), String> {
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;
    let mut depth = 0;
    let mut in_string: Option<char> = None;

    while i < chars.len() {
        let c = chars[i];

        if let Some(quote) = in_string {
            if c == quote && (i == 0 || chars[i - 1] != '\\') {
                in_string = None;
            }
        } else {
            if c == '\'' || c == '"' || c == '`' {
                in_string = Some(c);
            } else if c == '{' {
                depth += 1;
            } else if c == '}' {
                depth -= 1;
            } else if depth == 0 && (c == ':' || c == '(') {
                let mut j = i;
                while j > 0 && chars[j - 1].is_whitespace() {
                    j -= 1;
                }
                let mut ident = String::new();
                while j > 0
                    && (chars[j - 1].is_alphanumeric()
                        || chars[j - 1] == '_'
                        || chars[j - 1] == '$')
                {
                    ident.insert(0, chars[j - 1]);
                    j -= 1;
                }

                let mut is_dot = false;
                let mut k = j;
                while k > 0 && chars[k - 1].is_whitespace() {
                    k -= 1;
                }
                if k > 0 && chars[k - 1] == '.' {
                    is_dot = true;
                }

                if !ident.is_empty() && !is_dot {
                    if ident == "beforeAll" {
                        let start_paren = if chars[i] == '(' {
                            Some(i)
                        } else {
                            let mut k = i + 1;
                            let mut found = None;
                            while k < chars.len() {
                                if chars[k] == '(' {
                                    found = Some(k);
                                    break;
                                }
                                if !chars[k].is_whitespace()
                                    && chars[k] != 'a'
                                    && chars[k] != 's'
                                    && chars[k] != 'y'
                                    && chars[k] != 'n'
                                    && chars[k] != 'c'
                                    && chars[k] != 'f'
                                    && chars[k] != 'u'
                                    && chars[k] != 'n'
                                    && chars[k] != 'c'
                                    && chars[k] != 't'
                                    && chars[k] != 'i'
                                    && chars[k] != 'o'
                                    && chars[k] != 'n'
                                {
                                    break;
                                }
                                k += 1;
                            }
                            found
                        };

                        if let Some(sp) = start_paren {
                            let mut ep = sp + 1;
                            let mut paren_depth = 1;
                            while ep < chars.len() {
                                if chars[ep] == '(' {
                                    paren_depth += 1;
                                } else if chars[ep] == ')' {
                                    paren_depth -= 1;
                                    if paren_depth == 0 {
                                        break;
                                    }
                                }
                                ep += 1;
                            }
                            if ep < chars.len() {
                                let args_str: String = chars[sp + 1..ep].iter().collect();
                                let args: Vec<&str> = args_str
                                    .split(',')
                                    .map(|s| s.trim())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                if args.len() != 3 {
                                    return Err(format!(
                                        "Custom hook signature mismatch: 'beforeAll' must accept exactly 3 parameters (req, res, next). Found {} parameters: {:?}",
                                        args.len(),
                                        args
                                    ));
                                }
                            }
                        }
                    } else if ident != "function" && ident != "async" {
                        return Err(format!(
                            "Custom hook contract violation: Unrecognized hook '{}'. Only 'beforeAll' is allowed.",
                            ident
                        ));
                    }
                }
            }
        }
        i += 1;
    }
    Ok(())
}

/// Validates developer-defined custom hooks file (`hooks.js`) to ensure signatures and hook names conform to Amana specifications.
pub fn validate_custom_hooks(content: &str) -> Result<(), String> {
    let clean = clean_comments(content);
    if let Some(exports_body) = extract_exports(&clean) {
        validate_exports_body(&exports_body)?;
    }
    Ok(())
}

fn canonical_format(content: &str) -> String {
    let mut formatted = String::new();
    for line in content.lines() {
        formatted.push_str(line.trim_end());
        formatted.push('\n');
    }
    let final_content = formatted.trim_end().to_string();
    if final_content.is_empty() {
        "".to_string()
    } else {
        format!("{}\n", final_content)
    }
}

/// Compiles the generated Amana IR, producing a fully configured Express.js project directory.
/// It creates models migrations, routing handlers, HTML/EJS layouts, and initializes SQLite connectivity.
pub fn generate_project(dest_dir: &str, ir: &AmanaIR) -> Result<(), String> {
    let dest_path = Path::new(dest_dir);

    // 1. التحقق من توافق ملف الخطافات المخصص للمطور قبل التصريف
    let custom_hooks_path = dest_path.join("custom/hooks.js");
    if custom_hooks_path.exists() {
        let content = fs::read_to_string(&custom_hooks_path).map_err(|e| e.to_string())?;
        validate_custom_hooks(&content)?;
    }

    // 2. استخدام BTreeMap لجمع وتوليد الملفات بترتيب حتمي (Alphabetical sorted order)
    let mut files_to_write: BTreeMap<String, String> = BTreeMap::new();

    // ملف الخطافات الافتراضي إذا لم يكن موجوداً
    if !custom_hooks_path.exists() {
        let default_hooks = r#"// Amana Custom Hooks
// This file is NOT overwritten on recompilation. Add your custom middlewares or route controllers here.
module.exports = {
  // beforeAll: (req, res, next) => { console.log(`Custom log: ${req.method} ${req.url}`); next(); }
};"#;
        files_to_write.insert("custom/hooks.js".to_string(), default_hooks.to_string());
    }

    // توليد package.json
    let package_json = r#"{
  "name": "amana-generated-app",
  "version": "1.0.0",
  "description": "Secure app generated by Amana Compiler",
  "main": "app.js",
  "scripts": {
    "start": "node app.js",
    "dev": "nodemon app.js"
  },
  "dependencies": {
    "express": "^4.19.2",
    "express-session": "^1.18.2",
    "sqlite3": "^6.0.1",
    "ejs": "^3.1.10",
    "argon2": "^0.41.1",
    "express-rate-limit": "^7.5.0",
    "helmet": "^8.1.0"
  },
  "devDependencies": {
    "nodemon": "^3.1.10"
  }
}"#;
    files_to_write.insert("package.json".to_string(), package_json.to_string());

    // توليد البرمجيات الوسيطة للأمان والتحقق middleware/security.js
    let security_js = r#"const rateLimit = require('express-rate-limit');
const crypto = require('crypto');

// 1. محدد معدل الطلبات (Rate Limiting)
const limiter = rateLimit({
  windowMs: 15 * 60 * 1000,
  max: 100,
  standardHeaders: true,
  legacyHeaders: false,
  message: 'لقد تجاوزت الحد المسموح من الطلبات. يرجى المحاولة لاحقاً.'
});

// 2. التحقق من توكن CSRF المخصص والمستقر
const authLimiter = rateLimit({
  windowMs: 15 * 60 * 1000,
  max: 20,
  standardHeaders: true,
  legacyHeaders: false,
  message: 'Too many authentication attempts. Please retry later.'
});

const apiLimiter = rateLimit({
  windowMs: 60 * 1000,
  max: 120,
  standardHeaders: true,
  legacyHeaders: false,
  message: { error: 'API rate limit exceeded. Please retry later.' }
});

const getCookie = (req, name) => {
  const cookies = req.headers.cookie;
  if (!cookies) return null;
  const match = cookies.match(new RegExp('(^|;)\\s*' + name + '\\s*=\\s*([^;]+)'));
  return match ? decodeURIComponent(match[2]) : null;
};

const csrfProtection = (req, res, next) => {
  let cookieToken = getCookie(req, 'csrfToken');
  if (!cookieToken) {
    cookieToken = crypto.randomBytes(32).toString('hex');
    res.cookie('csrfToken', cookieToken, { httpOnly: true, secure: process.env.NODE_ENV === 'production', sameSite: 'lax' });
  }

  if (req.session) {
    req.session.csrfToken = cookieToken;
  }
  
  if (req.method === 'POST') {
    const token = req.body._csrf || req.headers['x-csrf-token'];
    if (!token || token !== cookieToken) {
      return res.status(403).send('CSRF validation failed. Unauthorized request.');
    }
  }
  next();
};

function sanitizeValue(value) {
  if (typeof value === 'string') {
    return value
      .replace(/<script[\s\S]*?>[\s\S]*?<\/script>/gi, '')
      .replace(/\son\w+\s*=\s*(['"]).*?\1/gi, '')
      .replace(/javascript:/gi, '');
  }
  if (Array.isArray(value)) return value.map(sanitizeValue);
  if (value && typeof value === 'object') {
    for (const key of Object.keys(value)) {
      value[key] = sanitizeValue(value[key]);
    }
  }
  return value;
}

const inputSanitizer = (req, _res, next) => {
  req.body = sanitizeValue(req.body || {});
  req.query = sanitizeValue(req.query || {});
  req.params = sanitizeValue(req.params || {});
  next();
};

module.exports = {
  limiter,
  authLimiter,
  apiLimiter,
  csrfProtection,
  inputSanitizer
};"#;
    files_to_write.insert(
        "middleware/security.js".to_string(),
        security_js.to_string(),
    );

    // توليد ملف عامل VM معزول للخطافات middleware/hooks-worker.js
    let hooks_worker_js = r#"const fs = require('fs');
const vm = require('vm');
const path = require('path');

let sandboxHook = null;

try {
  const hookFilePath = path.resolve(__dirname, '../custom/hooks.js');
  if (fs.existsSync(hookFilePath)) {
    const hookCode = fs.readFileSync(hookFilePath, 'utf8');
    const scriptCode = `
      (function() {
        const exports = {};
        const module = { exports };
        ${hookCode}
        return module.exports;
      })()
    `;
    const script = new vm.Script(scriptCode, { filename: 'hooks.js' });
    const context = vm.createContext({
      console: {
        log: (...args) => console.log('[Sandbox Log]', ...args),
        error: (...args) => console.error('[Sandbox Error]', ...args)
      }
    });
    const exportsObj = script.runInContext(context, { timeout: 1000 });
    if (exportsObj && typeof exportsObj.beforeAll === 'function') {
      sandboxHook = exportsObj.beforeAll;
    }
  }
} catch (e) {
  console.error('[Amana Sandbox Compile Error]', e);
}

function validateExecuteRequest(msg) {
  if (!msg || typeof msg !== 'object') return false;
  if (msg.type !== 'EXECUTE_HOOK') return false;
  if (typeof msg.reqId !== 'number' || msg.reqId <= 0) return false;
  if (!msg.req || typeof msg.req !== 'object') return false;
  
  const req = msg.req;
  if (typeof req.method !== 'string') return false;
  if (typeof req.url !== 'string') return false;
  if (typeof req.headers !== 'object' || req.headers === null) return false;
  if (typeof req.body !== 'object' || req.body === null) return false;
  if (typeof req.query !== 'object' || req.query === null) return false;
  if (typeof req.params !== 'object' || req.params === null) return false;
  
  return true;
}

process.on('message', async (msg) => {
  if (!validateExecuteRequest(msg)) {
    console.warn('[Security Warning] IPC Request contract violation - message discarded:', msg);
    return;
  }

  const { reqId, req } = msg;
  if (!sandboxHook) {
    process.send({ type: 'HOOK_RESPONSE', reqId, action: 'next' });
    return;
  }

  try {
    let sent = false;
    const safeReq = {
      method: req.method,
      url: req.url,
      headers: { ...req.headers },
      query: { ...req.query },
      body: { ...req.body },
      params: { ...req.params }
    };

    const safeRes = {
      status: (code) => {
        safeRes.statusCode = code;
        return safeRes;
      },
      send: (body) => {
        if (!sent) {
          sent = true;
          process.send({
            type: 'HOOK_RESPONSE',
            reqId,
            action: 'send',
            status: safeRes.statusCode || 200,
            body
          });
        }
      },
      redirect: (url) => {
        if (!sent) {
          sent = true;
          process.send({
            type: 'HOOK_RESPONSE',
            reqId,
            action: 'redirect',
            url
          });
        }
      }
    };

    const result = sandboxHook(safeReq, safeRes, (err) => {
      if (err) {
        console.error('[Amana Hook Error] beforeAll passed an error:', err);
        process.send({ type: 'HOOK_RESPONSE', reqId, action: 'error', error: err.toString() });
      } else if (!sent) {
        process.send({ type: 'HOOK_RESPONSE', reqId, action: 'next' });
      }
    });

    if (result instanceof Promise) {
      await result;
    }
  } catch (err) {
    console.error('[Amana Hook Exception] beforeAll crashed:', err);
    process.send({ type: 'HOOK_RESPONSE', reqId, action: 'crash', error: err.toString() });
  }
});"#;
    files_to_write.insert(
        "middleware/hooks-worker.js".to_string(),
        hooks_worker_js.to_string(),
    );

    // توليد الملف الرئيسي app.js لتشغيل محرك أمانة
    let app_js = r#"const AmanaEngine = require('./runtime/engine');
const path = require('path');

const irPath = path.join(__dirname, 'amana_ir.json');
const engine = new AmanaEngine(irPath);
engine.start().catch(err => {
  console.error('[Amana Engine Startup Error]', err);
  process.exit(1);
});
"#;
    files_to_write.insert("app.js".to_string(), app_js.to_string());

        // توليد محرك التشغيل الموحد للغة أمانة runtime/engine.js
    let engine_js = static_files::engine_js();
    // Let's compile and write the EJS templates at compile-time in Rust!
    let html_dir = theme::theme_direction(ir.theme.as_ref());
    let html_lang = theme::theme_language(ir.theme.as_ref());
    let bootstrap_css = if html_dir == "rtl" { "bootstrap.rtl.min.css" } else { "bootstrap.min.css" };
    let theme_css_block = theme::theme_css(ir.theme.as_ref(), None);

    for view in &ir.views {
        let view_html = compile_view_ejs(
            view,
            html_lang,
            html_dir,
            bootstrap_css,
            &theme_css_block,
            ir,
        );
        files_to_write.insert(format!("views/{}.ejs", view.name.to_lowercase()), view_html);
    }

    // Generate EJS template files for custom components
    for comp in &ir.components {
        let comp_html = if let Some(body) = &comp.render_body {
            crate::codegen::html::generate_ejs(body, &Vec::new())
        } else {
            String::new()
        };
        files_to_write.insert(format!("views/components/{}.ejs", comp.name), comp_html);
    }

    // Default login page if not defined in DSL
    let has_dsl_login_route = ir.routes.iter().any(|r| r.path == "/login");
    if !has_dsl_login_route {
        let default_login_html = compile_default_login_ejs(html_lang, html_dir, &theme_css_block);
        files_to_write.insert("views/login.ejs".to_string(), default_login_html);
    }

    files_to_write.insert("runtime/engine.js".to_string(), engine_js.to_string());

    // 3. كتابة ملف التمثيل الوسيط المحسن بالكامل
    let ir_json = serde_json::to_string_pretty(ir).map_err(|e| e.to_string())?;
    files_to_write.insert("amana_ir.json".to_string(), ir_json);

    // 4. كتابة كافة الملفات بترتيب حتمي وبتنسيق موحد (Canonical format)
    for (rel_path, content) in &files_to_write {
        let full_path = dest_path.join(rel_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let formatted = canonical_format(content);
        fs::write(&full_path, formatted).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn compile_view_ejs(
    view: &ViewIR,
    html_lang: &str,
    html_dir: &str,
    _bootstrap_css: &str,
    theme_css_block: &str,
    _ir: &AmanaIR,
) -> String {
    let mut ejs_body = if let Some(body) = &view.render_body {
        crate::codegen::html::generate_ejs(body, &view.client_states)
    } else {
        String::new()
    };
    if !view.client_states.is_empty() {
        let mut state_fields = Vec::new();
        for state in &view.client_states {
            let initial_js = crate::codegen::html::compile_expression_to_js(&state.initial_value);
            state_fields.push(format!("{}: {}", state.name, initial_js));
        }
        let x_data_str = format!("{{ {} }}", state_fields.join(", "));
        let escaped_x_data = x_data_str.replace('&', "&amp;")
                                       .replace('"', "&quot;")
                                       .replace('<', "&lt;")
                                       .replace('>', "&gt;");
        ejs_body = format!("<div x-data=\"{}\">\n{}\n</div>", escaped_x_data, ejs_body);
    }

    let body_attrs = canvas_attributes(view.canvas.as_ref());
    let page_styles = view.styles.as_deref().unwrap_or("");

    format!(
        r#"<!DOCTYPE html>
<html lang="{}" dir="{}">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title><%= typeof title !== 'undefined' ? title : 'Amana Application' %></title>
  <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
  <script src="https://cdn.jsdelivr.net/npm/arabic-reshaper@1.1.0/index.js"></script>
  <script>
    function fixArabicText(str) {{
      if (!str) return str;
      if (typeof str !== 'string') str = String(str);
      if (typeof ArabicReshaper !== 'undefined') {{
        str = ArabicReshaper.convertArabic(str);
      }}
      const arabicRegex = /[\u0600-\u06FF\u0750-\u077F\u08A0-\u08FF\uFB50-\uFDFF\uFE70-\uFEFF]/;
      let words = str.split(/(\s+)/);
      let processed = words.map(word => {{
        if (arabicRegex.test(word)) {{
          return word.split('').reverse().join('');
        }}
        return word;
      }});
      return processed.reverse().join('');
    }}
  </script>
  <script defer src="https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js"></script>
  <script defer src="https://code.iconify.design/iconify-icon/2.1.0/iconify-icon.min.js"></script>
  <style>
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
      :where(.hero-grid, .split, .workflow-grid, .pricing-grid, .testimonial-grid, .cta-box, .amana-split, .dg-layout-split-diagonal, .dg-layout-asymmetric, .dg-layout-editorial, .dg-layout-dashboard-shell, :where(.dg-layout-dashboard-shell) .amana-runtime-shell > :not(script):not(style), .dg-layout-command-center, .dg-layout-showcase-rail):not([style*="--bp-laptop-columns"]):not([style*="--bp-tablet-columns"]):not([style*="--bp-mobile-columns"]) {{ grid-template-columns: minmax(0, 1fr) !important; }}
    }}
    @media (max-width: 720px) {{
      :where(.hero-title, .section-title, .auth-card h1, .cta-box h2, .amana-hero h1, h1) {{ font-size: clamp(2rem, 11vw, 3.4rem) !important; }}
      :where(.hero-panel, .workflow-box, .visual-card, .price-card, .testimonial-card, .cta-box, .auth-card, .contact-card, .amana-card) {{ padding: clamp(1rem, 5vw, 1.5rem) !important; border-radius: min(var(--radius-2xl), 22px) !important; }}
      :where(.hero-actions, .trust-strip, .badge-row, .token-list, .logo-row, .amana-hero-actions, .amana-cluster) {{ align-items: stretch; }}
      :where(.amana-btn, .plan-button, button) {{ white-space: normal; text-align: center; }}
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
        base_css::BASE_CSS,
        page_styles,
        body_attrs,
        ejs_body
    )
}

fn compile_default_login_ejs(
    html_lang: &str,
    html_dir: &str,
    theme_css: &str,
) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="{}" dir="{}">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>تسجيل الدخول</title>
  <style>
    {}
    {}
  </style>
</head>
<body class="amana-login-shell">
  <div class="amana-login-card">
    <h2 class="amana-login-title">تسجيل الدخول</h2>
    <% if (typeof error !== 'undefined' && error) {{ %>
      <div class="amana-login-error"><%= error %></div>
    <% }} %>
    <form action="/login" method="POST">
      <input type="hidden" name="_csrf" value="<%= typeof csrfToken !== 'undefined' ? csrfToken : '' %>">
      <div class="amana-field">
        <span>البريد الإلكتروني</span>
        <input class="amana-form-control" type="email" id="email" name="email" required>
      </div>
      <div class="amana-field">
        <span>كلمة المرور</span>
        <input class="amana-form-control" type="password" id="password" name="password" required>
      </div>
      <button class="amana-btn amana-btn-primary" style="width: 100%" type="submit">دخول</button>
    </form>
    <div style="margin-top: 1.5rem; text-align: center; color: var(--text-secondary); font-size: var(--text-xs)">
      <small>Use AMANA_SEED_ADMIN=true with AMANA_ADMIN_EMAIL and AMANA_ADMIN_PASSWORD to create an initial admin account.</small>
    </div>
  </div>
</body>
</html>"#,
        html_lang,
        html_dir,
        theme_css,
        base_css::BASE_CSS
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
        if kind == "canvas" && key == "layout" { classes.push(format!("dg-layout-{}", value)); }
        if kind == "canvas" && key == "surface" { classes.push(format!("dg-surface-{}", value)); }
        if kind == "canvas" && key == "density" { classes.push(format!("dg-density-{}", value)); }
        if kind == "canvas" && key == "rhythm" { classes.push(format!("dg-rhythm-{}", value)); }
        if kind == "canvas" && key == "mode" { classes.push(format!("dg-mode-{}", value)); }
        if kind == "canvas" && key == "palette" { classes.push(format!("dg-palette-{}", value)); }
        if kind == "compose" && key == "layout" { classes.push(format!("dg-layout-{}", value)); }
        if kind == "compose" && key == "rhythm" { classes.push(format!("dg-rhythm-{}", value)); }
        if kind == "compose" && key == "density" { classes.push(format!("dg-density-{}", value)); }
        if kind == "compose" && key == "flow" { classes.push(format!("dg-flow-{}", value)); }
        if kind == "compose" && key == "focus-path" { classes.push(format!("dg-focus-path-{}", value)); }
        if kind == "compose" && key == "alignment" { classes.push(format!("dg-align-{}", value)); }
        if kind == "visual" && key == "gradient" { classes.push(format!("dg-gradient-{}", value)); }
        if kind == "visual" && key == "surface" { classes.push(format!("dg-surface-{}", value)); }
        if kind == "visual" && key == "shape" { classes.push(format!("dg-shape-{}", value)); }
        if kind == "visual" && key == "mode" { classes.push(format!("dg-mode-{}", value)); }
        if kind == "visual" && key == "texture" { classes.push(format!("dg-texture-{}", value)); }
        if kind == "visual" && key == "palette" { classes.push(format!("dg-palette-{}", value)); }
        if kind == "visual" && key == "frame" { classes.push(format!("dg-frame-{}", value)); }
        if kind == "component" && key == "variant" { classes.push(format!("dg-component-variant-{}", value)); }
        if kind == "component" && key == "shape" { classes.push(format!("dg-component-shape-{}", value)); }
        if kind == "component" && key == "density" { classes.push(format!("dg-component-density-{}", value)); }
        if kind == "component" && key == "chrome" { classes.push(format!("dg-component-chrome-{}", value)); }
        if kind == "type" && key == "scale" { classes.push(format!("dg-type-scale-{}", value)); }
        if kind == "type" && key == "align" { classes.push(format!("dg-type-align-{}", value)); }
        if kind == "type" && key == "measure" { classes.push(format!("dg-type-measure-{}", value)); }
        if kind == "type" && key == "hierarchy" { classes.push(format!("dg-type-hierarchy-{}", value)); }
        if kind == "type" && key == "tone" { classes.push(format!("dg-type-tone-{}", value)); }
        if kind == "motion" && key == "entrance" { classes.push(format!("dg-motion-{}", value)); }
        if kind == "motion" && key == "hover" { classes.push(format!("dg-hover-{}", value)); }
        if kind == "motion" && key == "reveal" { classes.push(format!("dg-reveal-{}", value)); }
        if kind == "brand" && key == "voice" { classes.push(format!("dg-brand-voice-{}", value)); }
        if kind == "brand" && key == "personality" { classes.push(format!("dg-brand-personality-{}", value)); }
        if kind == "brand" && key == "colorway" { classes.push(format!("dg-colorway-{}", value)); }
        if kind == "brand" && key == "trust" { classes.push(format!("dg-brand-trust-{}", value)); }
        if kind == "art" && key == "direction" { classes.push(format!("dg-art-{}", value)); }
        if kind == "art" && key == "motif" { classes.push(format!("dg-motif-{}", value)); }
        if kind == "art" && key == "lighting" { classes.push(format!("dg-lighting-{}", value)); }
        if kind == "art" && key == "texture" { classes.push(format!("dg-texture-{}", value)); }
        if kind == "responsive" { classes.push(format!("dg-rsp-{}-{}", key, value)); }
        if kind == "interaction" && key == "feedback" { classes.push(format!("dg-feedback-{}", value)); }
        if kind == "interaction" && key == "affordance" { classes.push(format!("dg-affordance-{}", value)); }
        if kind == "interaction" && key == "cursor" { classes.push(format!("dg-cursor-{}", value)); }
        if kind == "a11y" && key == "contrast" { classes.push(format!("dg-a11y-contrast-{}", value)); }
        if kind == "a11y" && key == "focus" { classes.push(format!("dg-focus-visible-{}", value)); }
        if kind == "a11y" && key == "reduce-motion" { classes.push(format!("dg-reduce-motion-{}", value)); }
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
    
    if ["padding", "padding-x", "padding-y", "space-padding", "space-padding-x", "space-padding-y", "gap", "space-gap", "margin", "margin-x", "margin-y"].contains(&normalized_key.as_str())
        || normalized_key.ends_with(".padding")
        || normalized_key.ends_with(".gap") {
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
    
    if ["width", "height", "min-width", "min-height", "max-width", "max-height", "title-width", "copy-width", "text-width"].contains(&normalized_key.as_str())
        || ["size", "font-size", "font_size", "copy-size", "title-size"].contains(&normalized_key.as_str()) {
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
    
    if ["primary", "accent", "background", "bg", "surface-bg", "color-background", "surface-color", "fill", "text", "ink", "color-text", "muted", "subtle", "color-muted", "border", "border-color", "stroke", "outline"].contains(&normalized_key.as_str()) {
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
    if !val.chars().all(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || " .,%#()+-/*".contains(c)) {
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
            ("visual", "visual-weight") => styles.push(format!("--dg-visual-weight:{}", clean_value)),
            ("visual", "texture-opacity") => styles.push(format!("--dg-texture-opacity:{}", clean_value)),
            ("visual", "glow-strength") => styles.push(format!("--dg-glow-strength:{}", clean_value)),
            ("visual" | "tokens" | "component" | "canvas", "primary" | "color.primary" | "brand.primary") => {
                styles.push(format!("--custom-primary:{}", clean_value));
            }
            ("visual" | "tokens" | "component" | "canvas", "accent" | "color.accent" | "brand.accent") => {
                styles.push(format!("--custom-accent:{}", clean_value));
            }
            ("visual" | "tokens" | "component" | "canvas", "background" | "bg" | "surface.bg" | "color.background") => {
                styles.push(format!("--custom-bg:{};background:{}", clean_value, clean_value));
            }
            ("visual" | "tokens" | "component" | "canvas", "text" | "ink" | "color.text") => {
                styles.push(format!("--custom-text:{};color:{}", clean_value, clean_value));
            }
            ("visual" | "tokens" | "component" | "canvas", "muted" | "subtle" | "color.muted") => {
                styles.push(format!("--custom-muted:{}", clean_value));
            }
            ("visual" | "tokens" | "component" | "canvas", "border" | "border.color" | "stroke" | "outline") => {
                styles.push(format!("--custom-border:{};border-color:{}", clean_value, clean_value));
            }
            ("visual" | "tokens" | "component" | "canvas", "gradient.value" | "gradient-value" | "custom-gradient") => {
                styles.push(format!("--custom-gradient:{};background:{}", clean_value, clean_value));
            }
            ("visual" | "tokens" | "component" | "canvas", "radius" | "shape.radius") => {
                styles.push(format!("--custom-radius:{};border-radius:{}", clean_value, clean_value));
            }
            ("visual" | "tokens" | "component" | "canvas", "shadow" | "shadow.value") => {
                styles.push(format!("--custom-shadow:{};box-shadow:{}", clean_value, clean_value));
            }
            ("visual" | "tokens" | "component" | "canvas", "padding" | "space.padding") => {
                styles.push(format!("--custom-padding:{};padding:{}", clean_value, clean_value));
            }
            ("visual" | "tokens" | "component" | "canvas", "gap" | "space.gap") => {
                styles.push(format!("--custom-gap:{};gap:{}", clean_value, clean_value));
            }
            ("compose", "columns") => styles.push(format!("--dg-columns:{}", clean_value)),
            ("compose", "gap") => styles.push(format!("--dg-gap:{}", clean_value)),
            ("compose", "grid-min") => styles.push(format!("--grid-min:{}", clean_value)),
            ("compose", "max-width") => styles.push(format!("--dg-max-width:{}", clean_value)),
            ("compose", "template" | "grid-template") => styles.push(format!("--dg-template:{}", clean_value)),
            ("motion", "speed") => styles.push(format!("--dg-motion-speed:{}", clean_value)),
            ("type", "measure") => styles.push(format!("--dg-type-measure:{}", clean_value)),
            ("type", "weight") => styles.push(format!("--dg-type-weight:{}", clean_value)),
            ("type", "size" | "font-size") => styles.push(format!("font-size:{}", clean_value)),
            ("type", "leading" | "line-height") => styles.push(format!("line-height:{}", clean_value)),
            ("type", "tracking" | "letter-spacing") => styles.push(format!("letter-spacing:{}", clean_value)),
            ("canvas", "content-width") => styles.push(format!("--content-width:{}", clean_value)),
            ("canvas", "wide-width") => styles.push(format!("--wide-width:{}", clean_value)),
            ("canvas", "readable-width") => styles.push(format!("--readable-width:{}", clean_value)),
            ("responsive", "columns") => styles.push(format!("--dg-responsive-columns:{}", clean_value)),
            ("responsive", "desktop.columns") => styles.push(format!("--bp-desktop-columns:{}", clean_value)),
            ("responsive", "laptop.columns") => styles.push(format!("--bp-laptop-columns:{}", clean_value)),
            ("responsive", "tablet.columns") => styles.push(format!("--bp-tablet-columns:{}", clean_value)),
            ("responsive", "mobile.columns") => styles.push(format!("--bp-mobile-columns:{}", clean_value)),
            ("responsive", "desktop.padding") => styles.push(format!("--bp-desktop-padding:{}", clean_value)),
            ("responsive", "laptop.padding") => styles.push(format!("--bp-laptop-padding:{}", clean_value)),
            ("responsive", "tablet.padding") => styles.push(format!("--bp-tablet-padding:{}", clean_value)),
            ("responsive", "mobile.padding") => styles.push(format!("--bp-mobile-padding:{}", clean_value)),
            ("responsive", "desktop.gap") => styles.push(format!("--bp-desktop-gap:{}", clean_value)),
            ("responsive", "laptop.gap") => styles.push(format!("--bp-laptop-gap:{}", clean_value)),
            ("responsive", "tablet.gap") => styles.push(format!("--bp-tablet-gap:{}", clean_value)),
            ("responsive", "mobile.gap") => styles.push(format!("--bp-mobile-gap:{}", clean_value)),
            ("art", "texture-opacity") => styles.push(format!("--dg-texture-opacity:{}", clean_value)),
            ("interaction", "focus-strength") => styles.push(format!("--dg-focus-strength:{}", clean_value)),
            _ => {}
        }

        if can_size_component {
            match key.as_str() {
                "min-height" => styles.push(format!("--component-min-height:{};min-height:{}", clean_value, clean_value)),
                "height" => styles.push(format!("--component-height:{};height:{}", clean_value, clean_value)),
                "width" => styles.push(format!("--component-width:{};width:{}", clean_value, clean_value)),
                "max-width" => styles.push(format!("--component-max-width:{};max-width:{}", clean_value, clean_value)),
                "padding-x" => styles.push(format!("--component-padding-x:{}", clean_value)),
                "padding-y" => styles.push(format!("--component-padding-y:{}", clean_value)),
                "min-width" => styles.push(format!("--component-min-width:{};min-width:{}", clean_value, clean_value)),
                "max-height" => styles.push(format!("--component-max-height:{};max-height:{}", clean_value, clean_value)),
                "columns" | "layout.columns" => styles.push(format!("--component-columns:{};--dg-columns:{}", clean_value, clean_value)),
                "title-width" => styles.push(format!("--component-title-width:{}", clean_value)),
                "copy-width" | "text-width" => styles.push(format!("--component-copy-width:{}", clean_value)),
                "title-size" => styles.push(format!("--component-title-size:{}", clean_value)),
                "copy-size" | "text-size" => styles.push(format!("--component-copy-size:{}", clean_value)),
                "transition" | "motion.transition" => styles.push(format!("--component-transition:{};transition:{}", clean_value, clean_value)),
                "transform" => styles.push(format!("--component-transform:{};transform:{}", clean_value, clean_value)),
                "opacity" => styles.push(format!("--component-opacity:{};opacity:{}", clean_value, clean_value)),
                _ => {}
            }
        }
    }
    styles.join(";")
}

fn design_settings_summary(canvas: &DesignBlock) -> String {
    canvas.settings.iter()
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
            attrs.push(format!("data-dg-{}=\"{}\"", escape_html_attr(&kind), escape_html_attr(&summary)));
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

