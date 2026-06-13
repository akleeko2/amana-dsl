// src/codegen/express.rs
use crate::ast::DesignBlock;
use crate::semantic::ir::*;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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

const csrfProtection = (req, res, next) => {
  if (!req.session.csrfToken) {
    req.session.csrfToken = crypto.randomBytes(32).toString('hex');
  }
  
  if (req.method === 'POST') {
    const token = req.body._csrf || req.headers['x-csrf-token'];
    if (!token || token !== req.session.csrfToken) {
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
    let engine_js = r#"const express = require('express');
const session = require('express-session');
const path = require('path');
const fs = require('fs');
const sqlite3 = require('sqlite3').verbose();
const ejs = require('ejs');
const argon2 = require('argon2');
const crypto = require('crypto');
const { fork } = require('child_process');
const { limiter, authLimiter, apiLimiter, csrfProtection, inputSanitizer } = require('../middleware/security');
const helmet = require('helmet');

function validateHookResponse(msg) {
  if (!msg || typeof msg !== 'object') return false;
  if (msg.type !== 'HOOK_RESPONSE') return false;
  if (typeof msg.reqId !== 'number' || msg.reqId <= 0) return false;
  
  const validActions = ['send', 'redirect', 'error', 'crash', 'next'];
  if (!validActions.includes(msg.action)) return false;
  
  if (msg.action === 'send') {
    if (msg.status !== undefined && (typeof msg.status !== 'number' || msg.status < 100 || msg.status > 599)) return false;
  }
  
  if (msg.action === 'redirect') {
    if (typeof msg.url !== 'string') return false;
  }
  
  if (msg.action === 'error' || msg.action === 'crash') {
    if (typeof msg.error !== 'string') return false;
  }
  
  return true;
}

function verifyPluginSignature(manifest) {
  if (!manifest.signature) return false;
  const key = process.env.AMANA_PLUGIN_KEY || (process.env.NODE_ENV === 'production' ? null : 'dev_only_plugin_key');
  if (!key) return false;
  const data = JSON.stringify({
    name: manifest.name,
    version: manifest.version,
    capabilities: manifest.capabilities
  });
  const expectedSignature = crypto.createHmac('sha256', key).update(data).digest('hex');
  return manifest.signature === expectedSignature;
}

function compileExpressionToJs(expr) {
  if (!expr) return 'null';
  if (typeof expr === 'string') {
    // Handle serialized Null variant as string
    if (expr === 'Null') return 'null';
    return expr;
  }
  if (expr.Number !== undefined) return expr.Number.toString();
  if (expr.Boolean !== undefined) return expr.Boolean.toString();
  if (expr.Null !== undefined) return 'null';
  if (expr.StringLiteral !== undefined) {
    const s = expr.StringLiteral;
    if (s.startsWith('f"') && s.endsWith('"')) {
      const content = s.substring(2, s.length - 1);
      return '`' + content.replace(/{/g, '${') + '`';
    }
    return JSON.stringify(s);
  }
  if (expr.Identifier !== undefined) {
    const id = expr.Identifier;
    return id === 'User.current' ? 'currentUser' : id;
  }
  if (expr.Binary !== undefined) {
    const { left, op, right } = expr.Binary;
    const l = compileExpressionToJs(left);
    const r = compileExpressionToJs(right);
    const jsOp = op === 'and' ? '&&' : (op === 'or' ? '||' : op);
    return `(${l} ${jsOp} ${r})`;
  }
  if (expr.Unary !== undefined) {
    const { op, expr: innerExpr } = expr.Unary;
    const e = compileExpressionToJs(innerExpr);
    const jsOp = op === 'not' ? '!' : op;
    return `(${jsOp}${e})`;
  }
  if (expr.Ternary !== undefined) {
    const { cond, then_branch, else_branch } = expr.Ternary;
    const c = compileExpressionToJs(cond);
    const t = compileExpressionToJs(then_branch);
    const el = compileExpressionToJs(else_branch);
    return `(${c} ? ${t} : ${el})`;
  }
  if (expr.MemberAccess !== undefined) {
    const { object, property } = expr.MemberAccess;
    const obj = compileExpressionToJs(object);
    if (obj === 'User' && property === 'current') return 'currentUser';
    return `${obj}.${property}`;
  }
  if (expr.Call !== undefined) {
    const { callee, args } = expr.Call;
    if (callee.Identifier === 'env') {
      if (args.length === 1) {
        return `(process.env[${compileExpressionToJs(args[0])}] || "")`;
      } else if (args.length === 2) {
        return `(process.env[${compileExpressionToJs(args[0])}] || ${compileExpressionToJs(args[1])})`;
      }
    }
    const c = compileExpressionToJs(callee);
    const formattedArgs = args.map(compileExpressionToJs);
    return `${c}(${formattedArgs.join(', ')})`;
  }
  return 'null';
}

function referencesClientState(expr, clientStates) {
  if (!expr) return false;
  if (expr.Identifier !== undefined) {
    return clientStates.some(s => s.name === expr.Identifier);
  }
  if (expr.Binary !== undefined) {
    return referencesClientState(expr.Binary.left, clientStates) || referencesClientState(expr.Binary.right, clientStates);
  }
  if (expr.Unary !== undefined) {
    return referencesClientState(expr.Unary.expr, clientStates);
  }
  if (expr.Ternary !== undefined) {
    return referencesClientState(expr.Ternary.cond, clientStates) || 
           referencesClientState(expr.Ternary.then_branch, clientStates) || 
           referencesClientState(expr.Ternary.else_branch, clientStates);
  }
  if (expr.Call !== undefined) {
    return referencesClientState(expr.Call.callee, clientStates) || 
           expr.Call.args.some(arg => referencesClientState(arg, clientStates));
  }
  if (expr.MemberAccess !== undefined) {
    return referencesClientState(expr.MemberAccess.object, clientStates);
  }
  return false;
}

function textReferencesClientState(txt, clientStates) {
  if (txt.startsWith('f"') && txt.endsWith('"')) {
    const content = txt.substring(2, txt.length - 1);
    for (const state of clientStates) {
      if (content.includes(`{${state.name}}`)) return true;
    }
  }
  return false;
}

function themeSettings(theme) {
  const out = {};
  for (const [key, value] of (theme && theme.settings) || []) {
    out[String(key)] = String(value);
  }
  return out;
}

function themeDirection(theme) {
  const settings = themeSettings(theme);
  const raw = String(settings.direction || settings.dir || settings.writing_direction || '').toLowerCase();
  const rtl = String(settings.rtl || '').toLowerCase();
  if (raw === 'rtl' || rtl === 'true' || rtl === 'yes') return 'rtl';
  if (raw === 'ltr' || rtl === 'false' || rtl === 'no') return 'ltr';
  return 'rtl';
}

function themeLanguage(theme) {
  const settings = themeSettings(theme);
  const language = String(settings.language || settings.lang || settings.locale || '').trim();
  if (/^[a-zA-Z]{2,3}(-[a-zA-Z0-9]{2,8})*$/.test(language)) return language;
  return themeDirection(theme) === 'rtl' ? 'ar' : 'en';
}

function colorScale(name, fallback) {
  const scales = {
    indigo: ['#4f46e5', '#eef2ff', '#312e81'],
    cyan: ['#06b6d4', '#ecfeff', '#164e63'],
    violet: ['#7c3aed', '#f5f3ff', '#4c1d95'],
    emerald: ['#059669', '#ecfdf5', '#064e3b'],
    rose: ['#e11d48', '#fff1f2', '#881337'],
    slate: ['#334155', '#f1f5f9', '#0f172a']
  };
  return scales[name] || scales[fallback] || scales.indigo;
}

function namedColorScale(name) {
  const scales = {
    indigo: ['#4f46e5', '#eef2ff', '#312e81'],
    cyan: ['#06b6d4', '#ecfeff', '#164e63'],
    violet: ['#7c3aed', '#f5f3ff', '#4c1d95'],
    emerald: ['#059669', '#ecfdf5', '#064e3b'],
    rose: ['#e11d48', '#fff1f2', '#881337'],
    slate: ['#334155', '#f1f5f9', '#0f172a']
  };
  return scales[String(name || '').trim()] || null;
}

function safeCssLiteral(value, fallback = '') {
  const text = String(value || '').trim();
  if (!text || text.length > 260) return fallback;
  const lower = text.toLowerCase();
  if (/(javascript:|expression\s*\(|behavior\s*:|@import|url\s*\(|<|>|<\/|;|\{|\})/.test(lower)) {
    return fallback;
  }
  if (!/^[a-zA-Z0-9\s.,#%()+\-/*]+$/.test(text)) return fallback;
  return text;
}

function safeFontName(value, fallback) {
  const text = String(value || '').trim();
  if (!text || text.length > 80) return fallback;
  if (!/^[\p{L}\p{N} ._-]+$/u.test(text)) return fallback;
  return text.replace(/\s+/g, ' ');
}

function fontStack(primary, fallbacks) {
  const quoted = `'${primary.replace(/'/g, '')}'`;
  return `${quoted}, ${fallbacks}`;
}

function googleFontQuery(fonts) {
  const uniqueFonts = Array.from(new Set(fonts.filter(Boolean)));
  return uniqueFonts
    .map(font => `family=${encodeURIComponent(font).replace(/%20/g, '+')}:wght@400;500;600;700;800;900`)
    .join('&');
}

function themeColor(value, fallbackName) {
  const fallback = namedColorScale(fallbackName) || namedColorScale('indigo');
  const named = namedColorScale(value);
  if (named) return named;
  const base = safeCssLiteral(value, fallback[0]);
  return [
    base,
    `color-mix(in srgb, ${base} 16%, transparent)`,
    `color-mix(in srgb, ${base} 58%, #020617)`
  ];
}

function themeCss(theme) {
  const settings = themeSettings(theme);
  const primary = themeColor(settings.primary || 'indigo', 'indigo');
  const accent = themeColor(settings.accent || 'cyan', 'cyan');
  const dark = settings.mode === 'dark' || settings.mode === 'night';
  const direction = themeDirection(theme);
  const start = direction === 'rtl' ? 'right' : 'left';
  const end = direction === 'rtl' ? 'left' : 'right';
  const radiusMap = {
    none: ['0px', '0px', '0px', '0px', '0px'],
    sharp: ['4px', '8px', '12px', '16px', '20px'],
    soft: ['10px', '16px', '22px', '28px', '36px'],
    round: ['12px', '20px', '28px', '34px', '42px'],
    pill: ['9999px', '9999px', '9999px', '9999px', '9999px']
  };
  const densityMap = {
    compact: ['0.75rem', '1rem', '1.5rem'],
    comfortable: ['1rem', '1.5rem', '2.25rem'],
    spacious: ['1.25rem', '2rem', '3rem']
  };
  const radius = radiusMap[settings.radius] || radiusMap.soft;
  const density = densityMap[settings.density] || densityMap.comfortable;
  const surfaceGlass = settings.surface === 'glass';
  const canvas = safeCssLiteral(settings.canvas || settings.background || settings.bg, dark ? '#020617' : '#f8fafc');
  const base = safeCssLiteral(settings.base || settings.surface_base, dark ? '#0f172a' : '#ffffff');
  const muted = safeCssLiteral(settings.muted_surface || settings.surface_muted, dark ? '#111827' : '#f8fafc');
  const elevatedFallback = surfaceGlass ? (dark ? 'rgba(15,23,42,0.74)' : 'rgba(255,255,255,0.72)') : (dark ? '#1f2937' : '#ffffff');
  const elevated = safeCssLiteral(settings.elevated || settings.surface_elevated, elevatedFallback);
  const text = safeCssLiteral(settings.text || settings.ink, dark ? '#f8fafc' : '#0f172a');
  const textMuted = safeCssLiteral(settings.muted || settings.subtle, dark ? '#cbd5e1' : '#475569');
  const border = safeCssLiteral(settings.border, dark ? 'rgba(148,163,184,0.22)' : 'rgba(15,23,42,0.10)');
  const glassBg = safeCssLiteral(settings.glass || settings.glass_bg, dark ? 'rgba(15,23,42,0.66)' : 'rgba(255,255,255,0.58)');
  const glassBorder = safeCssLiteral(settings.glass_border, dark ? 'rgba(148,163,184,0.20)' : 'rgba(255,255,255,0.38)');
  const gradientPrimary = safeCssLiteral(settings.gradient_primary || settings.gradient, `linear-gradient(135deg, ${primary[0]}, ${accent[0]})`);
  const gradientAccent = safeCssLiteral(settings.gradient_accent, `linear-gradient(135deg, ${accent[0]}, ${primary[0]})`);
  const gradientHero = safeCssLiteral(settings.gradient_hero, `radial-gradient(circle at top right, ${primary[1]}, transparent 30%), linear-gradient(135deg, ${dark ? '#0f172a' : '#ffffff'}, ${accent[1]})`);
  const radius2xl = safeCssLiteral(settings.radius_2xl, radius[4]);
  const successColor = safeCssLiteral(settings.success, '#16a34a');
  const warningColor = safeCssLiteral(settings.warning, '#ca8a04');
  const dangerColor = safeCssLiteral(settings.danger, '#dc2626');
  
  const fontProvider = settings.font_provider || 'system';
  const hasGoogleFonts = fontProvider === 'google';
  const bodyFamily = safeFontName(settings.font_family || settings.body_font || settings.font, 'Plus Jakarta Sans');
  const headingFamily = safeFontName(settings.heading_font_family || settings.heading_font || settings.display_font, 'Outfit');
  const arabicFamily = safeFontName(settings.arabic_font_family || settings.arabic_font, 'IBM Plex Sans Arabic');
  const systemFallbacks = "-apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif";
  const bodyStack = fontStack(bodyFamily, `'${arabicFamily}', 'Tajawal', ${systemFallbacks}`);
  const headingStack = fontStack(headingFamily, `'${bodyFamily}', '${arabicFamily}', 'Tajawal', ${systemFallbacks}`);
  const fontImport = hasGoogleFonts
    ? `@import url('https://fonts.googleapis.com/css2?${googleFontQuery([headingFamily, bodyFamily, arabicFamily, 'Tajawal'])}&display=swap');\n`
    : '';
  const bodyFont = `400 1rem/1.6 ${bodyStack}`;
  const headingFont = `700 1.75rem/1.2 ${headingStack}`;

  return `
    ${fontImport}
    :root {
      --color-primary: ${primary[0]};
      --color-primary-soft: ${primary[1]};
      --color-accent: ${accent[0]};
      --amana-direction: ${direction};
      --amana-start: ${start};
      --amana-end: ${end};
      --bg-primary: ${base};
      --bg-secondary: ${canvas};
      --text-primary: ${text};
      --text-secondary: ${textMuted};
      --surface-base: ${base};
      --surface-muted: ${muted};
      --surface-elevated: ${elevated};
      --border-subtle: ${border};
      --glass-bg: ${glassBg};
      --glass-border: ${glassBorder};
      --glass-blur: blur(16px);
      --radius-sm: ${radius[0]};
      --radius-md: ${radius[1]};
      --radius-lg: ${radius[2]};
      --radius-xl: ${radius[3]};
      --radius-2xl: ${radius2xl};
      --radius-small: var(--radius-sm);
      --radius-medium: var(--radius-md);
      --radius-large: var(--radius-lg);
      --radius-soft: var(--radius-md);
      --space-xs: 0.25rem;
      --space-sm: 0.5rem;
      --space-md: ${density[0]};
      --space-lg: ${density[1]};
      --space-xl: ${density[2]};
      --space-2xl: 3rem;
      --space-3xl: 4.5rem;
      --space-4xl: 6rem;

      --text-xs: 0.75rem;
      --text-sm: 0.875rem;
      --text-md: 1rem;
      --text-lg: 1.125rem;
      --text-xl: 1.35rem;
      --text-2xl: 1.75rem;
      --text-3xl: 2.4rem;

      --shadow-sm: 0 1px 3px rgba(15,23,42,0.08), 0 1px 2px rgba(15,23,42,0.04);
      --shadow-md: 0 4px 6px -1px rgba(15,23,42,0.10), 0 2px 4px -1px rgba(15,23,42,0.06);
      --shadow-lg: 0 10px 24px -8px rgba(15,23,42,0.18);
      --shadow-xl: 0 20px 40px -12px rgba(15,23,42,0.28);

      --transition-fast: all 0.12s ease-in-out;

      --color-success: ${successColor};
      --color-warning: ${warningColor};
      --color-danger: ${dangerColor};

      --border-color: var(--border-subtle);

      --content-width: 1120px;
      --wide-width: 1360px;
      --readable-width: 72ch;
      --gradient-primary: ${gradientPrimary};
      --gradient-accent: ${gradientAccent};
      --gradient-hero: ${gradientHero};
      --gradient-mesh: ${safeCssLiteral(settings.gradient_mesh, `radial-gradient(circle at 10% 20%, ${primary[1]}, transparent 34%), radial-gradient(circle at 80% 0%, ${accent[1]}, transparent 38%), ${base}`)};
      --gradient-aurora: ${safeCssLiteral(settings.gradient_aurora, `radial-gradient(circle at 15% 20%, ${primary[1]}, transparent 30%), radial-gradient(circle at 80% 20%, ${accent[1]}, transparent 35%), ${canvas}`)};
      --gradient-spotlight: ${safeCssLiteral(settings.gradient_spotlight, `radial-gradient(circle at 50% 0%, ${primary[1]}, transparent 48%), ${base}`)};
      --shadow-soft: 0 10px 24px -18px rgba(15,23,42,0.35);
      --shadow-floating: 0 24px 55px -28px rgba(15,23,42,0.45);
      --shadow-strong: 0 30px 70px -30px rgba(2,6,23,0.62);
      --elevation-1: 0 1px 2px rgba(15,23,42,0.08);
      --elevation-2: 0 8px 18px -14px rgba(15,23,42,0.35);
      --elevation-3: 0 18px 36px -22px rgba(15,23,42,0.45);
      --elevation-4: 0 28px 55px -30px rgba(15,23,42,0.55);
      --elevation-5: 0 35px 80px -35px rgba(15,23,42,0.68);
      --layer-base: 0;
      --layer-raised: 10;
      --layer-sticky: 30;
      --layer-dropdown: 60;
      --layer-overlay: 80;
      --layer-modal: 100;
      --layer-toast: 120;
      --glow-primary: 0 0 0 4px ${primary[1]}, 0 18px 40px -24px ${primary[0]};
      --glow-accent: 0 0 0 4px ${accent[1]}, 0 18px 40px -24px ${accent[0]};
      --font-body: ${bodyFont};
      --font-heading: ${headingFont};
    }
    html { direction: ${direction}; }
    body { direction: ${direction}; color-scheme: ${dark ? 'dark' : 'light'}; background: var(--bg-secondary); color: var(--text-primary); font: var(--font-body); }
    :where(h1, h2, h3, h4, h5, h6) { font: var(--font-heading); }
  `;
}

function exprStaticValue(expr, fallback = '') {
  if (!expr) return fallback;
  if (expr.StringLiteral !== undefined) return expr.StringLiteral;
  if (expr.Identifier !== undefined) return `<%= ${expr.Identifier} %>`;
  if (expr.Number !== undefined) return String(expr.Number);
  if (expr.Boolean !== undefined) return String(expr.Boolean);
  return `<%= ${compileExpressionToJs(expr)} %>`;
}

function getAttr(attributes, name, fallback = '') {
  const found = attributes.find(([key]) => key === name);
  return found ? exprStaticValue(found[1], fallback) : fallback;
}

function escapeAttr(value) {
  return String(value || '')
    .replace(/&/g, '&amp;')
    .replace(/"/g, '&quot;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

function designToken(value) {
  return String(value || '')
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

function isDesignBlockElement(element) {
  return element && element.DesignBlock !== undefined;
}

function splitDesignChildren(children) {
  const designBlocks = [];
  const renderChildren = [];
  for (const child of children || []) {
    if (isDesignBlockElement(child)) {
      designBlocks.push(child.DesignBlock);
    } else {
      renderChildren.push(child);
    }
  }
  return { designBlocks, renderChildren };
}

function settingValue(block, key, fallback = '') {
  const found = (block.settings || []).find(([settingKey]) => String(settingKey) === key);
  return found ? String(found[1]) : fallback;
}

function safeDesignVarValue(value) {
  const text = String(value || '').trim();
  if (!text || text.length > 260) return '';
  const lower = text.toLowerCase();
  if (/(javascript:|expression\s*\(|behavior\s*:|@import|url\s*\(|<|>|<\/|;|\{|\})/.test(lower)) return '';
  if (!/^[a-zA-Z0-9 .,%#()+\-/*]+$/.test(text)) return '';
  return escapeAttr(text);
}

const designSpacingTokens = {
  none: '0',
  '0': '0',
  xs: 'var(--space-xs)',
  sm: 'var(--space-sm)',
  small: 'var(--padding-small)',
  md: 'var(--space-md)',
  medium: 'var(--padding-medium)',
  lg: 'var(--space-lg)',
  large: 'var(--padding-large)',
  xl: 'var(--space-xl)',
  '2xl': 'var(--space-2xl)',
  xxl: 'var(--space-2xl)',
  '3xl': 'var(--space-3xl)',
  '4xl': 'var(--space-4xl)'
};

const designSizeTokens = {
  full: '100%',
  screen: '100vh',
  fit: 'fit-content',
  min: 'min-content',
  max: 'max-content',
  content: 'var(--content-width)',
  readable: 'var(--readable-width)',
  wide: 'var(--wide-width)',
  'fluid-xs': 'clamp(0.75rem, 1.4vw, 0.9rem)',
  'fluid-sm': 'clamp(0.875rem, 1.6vw, 1rem)',
  'fluid-md': 'clamp(1rem, 1.8vw, 1.15rem)',
  'fluid-lg': 'clamp(1.125rem, 2.2vw, 1.35rem)',
  'fluid-xl': 'clamp(1.5rem, 4vw, 2.4rem)',
  'fluid-2xl': 'clamp(2rem, 6vw, 4rem)',
  'fluid-3xl': 'clamp(2.6rem, 8vw, 6rem)'
};

const designColorTokens = {
  primary: 'var(--color-primary)',
  'primary-soft': 'var(--color-primary-soft)',
  accent: 'var(--color-accent)',
  success: 'var(--color-success)',
  warning: 'var(--color-warning)',
  danger: 'var(--color-danger)',
  canvas: 'var(--bg-secondary)',
  surface: 'var(--surface-base)',
  'surface-muted': 'var(--surface-muted)',
  'surface-elevated': 'var(--surface-elevated)',
  text: 'var(--text-primary)',
  ink: 'var(--text-primary)',
  muted: 'var(--text-secondary)',
  subtle: 'var(--text-secondary)',
  border: 'var(--border-subtle)',
  'custom-primary': 'var(--custom-primary, var(--color-primary))',
  'custom-accent': 'var(--custom-accent, var(--color-accent))',
  'custom-bg': 'var(--custom-bg, var(--bg-secondary))',
  'custom-text': 'var(--custom-text, var(--text-primary))'
};

const designRadiusTokens = {
  full: '9999px',
  pill: '9999px',
  xl: 'var(--radius-xl)',
  '2xl': 'var(--radius-2xl)',
  soft: 'var(--radius-soft)',
  lg: 'var(--radius-large)',
  large: 'var(--radius-large)',
  md: 'var(--radius-medium)',
  medium: 'var(--radius-medium)',
  sm: 'var(--radius-small)',
  small: 'var(--radius-small)'
};

const designShadowTokens = {
  none: 'none',
  soft: 'var(--shadow-soft)',
  smooth: 'var(--shadow-smooth)',
  floating: 'var(--shadow-floating)',
  strong: 'var(--shadow-strong)',
  lg: 'var(--shadow-large)',
  large: 'var(--shadow-large)',
  primary: 'var(--glow-primary)',
  accent: 'var(--glow-accent)'
};

const designGradientTokens = {
  primary: 'var(--gradient-primary)',
  accent: 'var(--gradient-accent)',
  hero: 'var(--gradient-hero)',
  mesh: 'var(--gradient-mesh)',
  aurora: 'var(--gradient-aurora)',
  spotlight: 'var(--gradient-spotlight)',
  custom: 'var(--custom-gradient, var(--gradient-primary))',
  brand: 'linear-gradient(var(--gradient-angle, 135deg), var(--custom-primary, var(--color-primary)), var(--custom-accent, var(--color-accent)))'
};

function normalizeDesignStyleValue(key, value) {
  const raw = String(value || '').trim();
  const normalizedKey = String(key || '').toLowerCase().replace(/_/g, '-');
  const token = raw.toLowerCase();
  if (!raw) return '';
  if (['padding', 'padding-x', 'padding-y', 'space.padding', 'space.padding-x', 'space.padding-y', 'gap', 'space.gap', 'margin', 'margin-x', 'margin-y'].includes(normalizedKey)
    || normalizedKey.endsWith('.padding')
    || normalizedKey.endsWith('.gap')) {
    return designSpacingTokens[token] || raw;
  }
  if (['width', 'height', 'min-width', 'min-height', 'max-width', 'max-height', 'title-width', 'copy-width', 'text-width'].includes(normalizedKey)) {
    return designSizeTokens[token] || raw;
  }
  if (['size', 'font-size', 'font_size', 'copy-size', 'title-size'].includes(normalizedKey)) {
    return designSizeTokens[token] || raw;
  }
  if (['primary', 'accent', 'background', 'bg', 'surface.bg', 'color.background', 'surface.color', 'fill', 'text', 'ink', 'color.text', 'muted', 'subtle', 'color.muted', 'border', 'border.color', 'stroke', 'outline'].includes(normalizedKey)) {
    return designColorTokens[token] || raw;
  }
  if (['radius', 'shape.radius'].includes(normalizedKey)) return designRadiusTokens[token] || raw;
  if (['shadow', 'shadow.value'].includes(normalizedKey)) return designShadowTokens[token] || raw;
  if (['gradient', 'gradient.value', 'gradient-value', 'custom-gradient'].includes(normalizedKey)) return designGradientTokens[token] || raw;
  return raw;
}

function designAttributeStyleVars(attributes) {
  const styles = [];
  const consumed = new Set();
  const attrMap = [
    ['width', '--component-width', 'width'],
    ['height', '--component-height', 'height'],
    ['min_height', '--component-min-height', 'min-height'],
    ['min-height', '--component-min-height', 'min-height'],
    ['max_height', '--component-max-height', 'max-height'],
    ['max-height', '--component-max-height', 'max-height'],
    ['min_width', '--component-min-width', 'min-width'],
    ['min-width', '--component-min-width', 'min-width'],
    ['max_width', '--component-max-width', 'max-width'],
    ['max-width', '--component-max-width', 'max-width'],
    ['padding', '--component-padding', 'padding'],
    ['padding_x', '--component-padding-x', null],
    ['padding-x', '--component-padding-x', null],
    ['padding_y', '--component-padding-y', null],
    ['padding-y', '--component-padding-y', null],
    ['gap', '--component-gap', 'gap'],
    ['columns', '--component-columns', null],
    ['template', '--dg-template', null],
    ['title_size', '--component-title-size', null],
    ['title-size', '--component-title-size', null],
    ['copy_size', '--component-copy-size', null],
    ['copy-size', '--component-copy-size', null],
    ['title_width', '--component-title-width', null],
    ['title-width', '--component-title-width', null],
    ['copy_width', '--component-copy-width', null],
    ['copy-width', '--component-copy-width', null],
    ['background', '--custom-bg', 'background'],
    ['bg', '--custom-bg', 'background'],
    ['text', '--custom-text', 'color'],
    ['color', '--custom-text', 'color'],
    ['muted', '--custom-muted', null],
    ['border', '--custom-border', 'border-color'],
    ['radius', '--custom-radius', 'border-radius'],
    ['shadow', '--custom-shadow', 'box-shadow'],
    ['gradient', '--custom-gradient', 'background'],
    ['transition', '--component-transition', 'transition'],
    ['transform', '--component-transform', 'transform'],
    ['opacity', '--component-opacity', 'opacity']
  ];
  for (const [attrName, cssVar, cssProp] of attrMap) {
    const found = attributes.find(([key]) => key === attrName);
    if (!found) continue;
    consumed.add(attrName);
    const value = normalizeDesignStyleValue(attrName, exprStaticValue(found[1], ''));
    const cleanValue = safeDesignVarValue(value);
    if (!cleanValue) continue;
    styles.push(`${cssVar}:${cleanValue}`);
    if (cssProp) styles.push(`${cssProp}:${cleanValue}`);
    if (attrName === 'columns') styles.push(`--dg-columns:${cleanValue}`);
  }
  return { style: styles.join(';'), consumed };
}

function designSettingsSummary(block) {
  return (block.settings || [])
    .slice(0, 12)
    .map(([key, value]) => `${key}:${value}`)
    .join(';');
}

function designClassList(blocks) {
  const classes = [];
  for (const block of blocks || []) {
    const kind = designToken(block.kind);
    for (const [rawKey, rawValue] of block.settings || []) {
      const key = designToken(String(rawKey).replace(/\./g, '-'));
      const value = designToken(rawValue);
      if (!kind || !key || !value) continue;
      classes.push(`dg-${kind}-${key}-${value}`);
      if (kind === 'canvas' && key.startsWith('responsive-')) classes.push(`dg-rsp-${key.replace(/^responsive-/, '')}-${value}`);
      if (kind === 'canvas' && key === 'layout') classes.push(`dg-layout-${value}`);
      if (kind === 'canvas' && key === 'surface') classes.push(`dg-surface-${value}`);
      if (kind === 'canvas' && key === 'density') classes.push(`dg-density-${value}`);
      if (kind === 'canvas' && key === 'rhythm') classes.push(`dg-rhythm-${value}`);
      if (kind === 'canvas' && key === 'mode') classes.push(`dg-mode-${value}`);
      if (kind === 'canvas' && key === 'palette') classes.push(`dg-palette-${value}`);
      if (kind === 'compose' && key === 'layout') classes.push(`dg-layout-${value}`);
      if (kind === 'compose' && key === 'rhythm') classes.push(`dg-rhythm-${value}`);
      if (kind === 'compose' && key === 'density') classes.push(`dg-density-${value}`);
      if (kind === 'compose' && key === 'flow') classes.push(`dg-flow-${value}`);
      if (kind === 'compose' && key === 'focus-path') classes.push(`dg-focus-path-${value}`);
      if (kind === 'compose' && key === 'alignment') classes.push(`dg-align-${value}`);
      if (kind === 'visual' && key === 'gradient') classes.push(`dg-gradient-${value}`);
      if (kind === 'visual' && key === 'surface') classes.push(`dg-surface-${value}`);
      if (kind === 'visual' && key === 'shape') classes.push(`dg-shape-${value}`);
      if (kind === 'visual' && key === 'mode') classes.push(`dg-mode-${value}`);
      if (kind === 'visual' && key === 'texture') classes.push(`dg-texture-${value}`);
      if (kind === 'visual' && key === 'palette') classes.push(`dg-palette-${value}`);
      if (kind === 'visual' && key === 'frame') classes.push(`dg-frame-${value}`);
      if (kind === 'component' && key === 'variant') classes.push(`dg-component-variant-${value}`);
      if (kind === 'component' && key === 'shape') classes.push(`dg-component-shape-${value}`);
      if (kind === 'component' && key === 'density') classes.push(`dg-component-density-${value}`);
      if (kind === 'component' && key === 'chrome') classes.push(`dg-component-chrome-${value}`);
      if (kind === 'type' && key === 'scale') classes.push(`dg-type-scale-${value}`);
      if (kind === 'type' && key === 'align') classes.push(`dg-type-align-${value}`);
      if (kind === 'type' && key === 'measure') classes.push(`dg-type-measure-${value}`);
      if (kind === 'type' && key === 'hierarchy') classes.push(`dg-type-hierarchy-${value}`);
      if (kind === 'type' && key === 'tone') classes.push(`dg-type-tone-${value}`);
      if (kind === 'motion' && key === 'entrance') classes.push(`dg-motion-${value}`);
      if (kind === 'motion' && key === 'hover') classes.push(`dg-hover-${value}`);
      if (kind === 'motion' && key === 'reveal') classes.push(`dg-reveal-${value}`);
      if (kind === 'brand' && key === 'voice') classes.push(`dg-brand-voice-${value}`);
      if (kind === 'brand' && key === 'personality') classes.push(`dg-brand-personality-${value}`);
      if (kind === 'brand' && key === 'colorway') classes.push(`dg-colorway-${value}`);
      if (kind === 'brand' && key === 'trust') classes.push(`dg-brand-trust-${value}`);
      if (kind === 'art' && key === 'direction') classes.push(`dg-art-${value}`);
      if (kind === 'art' && key === 'motif') classes.push(`dg-motif-${value}`);
      if (kind === 'art' && key === 'lighting') classes.push(`dg-lighting-${value}`);
      if (kind === 'art' && key === 'texture') classes.push(`dg-texture-${value}`);
      if (kind === 'responsive') classes.push(`dg-rsp-${key}-${value}`);
      if (kind === 'interaction' && key === 'feedback') classes.push(`dg-feedback-${value}`);
      if (kind === 'interaction' && key === 'affordance') classes.push(`dg-affordance-${value}`);
      if (kind === 'interaction' && key === 'cursor') classes.push(`dg-cursor-${value}`);
      if (kind === 'a11y' && key === 'contrast') classes.push(`dg-a11y-contrast-${value}`);
      if (kind === 'a11y' && key === 'focus') classes.push(`dg-focus-visible-${value}`);
      if (kind === 'a11y' && key === 'reduce-motion') classes.push(`dg-reduce-motion-${value}`);
    }
  }
  return Array.from(new Set(classes));
}

function designStyleVars(blocks) {
  const styles = [];
  for (const block of blocks || []) {
    const kind = String(block.kind);
    for (const [key, value] of block.settings || []) {
      const normalizedValue = normalizeDesignStyleValue(key, value);
      const cleanValue = safeDesignVarValue(normalizedValue);
      if (!cleanValue) continue;
      const canSizeComponent = kind === 'visual' || kind === 'tokens' || kind === 'component';
      if (kind === 'visual' && key === 'depth') styles.push(`--dg-depth:${cleanValue}`);
      if (kind === 'visual' && key === 'visual_weight') styles.push(`--dg-visual-weight:${cleanValue}`);
      if (kind === 'visual' && key === 'texture_opacity') styles.push(`--dg-texture-opacity:${cleanValue}`);
      if (kind === 'visual' && key === 'glow_strength') styles.push(`--dg-glow-strength:${cleanValue}`);
      if ((kind === 'visual' || kind === 'tokens' || kind === 'component') && ['primary', 'color.primary', 'brand.primary'].includes(key)) styles.push(`--custom-primary:${cleanValue}`);
      if ((kind === 'visual' || kind === 'tokens' || kind === 'component') && ['accent', 'color.accent', 'brand.accent'].includes(key)) styles.push(`--custom-accent:${cleanValue}`);
      if ((kind === 'visual' || kind === 'tokens' || kind === 'component') && ['background', 'bg', 'surface.bg', 'color.background'].includes(key)) styles.push(`--custom-bg:${cleanValue};background:${cleanValue}`);
      if ((kind === 'visual' || kind === 'tokens' || kind === 'component') && ['surface', 'surface.color', 'fill'].includes(key) && !['glass', 'layered', 'glass layered', 'elevated', 'base', 'muted'].includes(String(value))) styles.push(`--custom-bg:${cleanValue};background:${cleanValue}`);
      if ((kind === 'visual' || kind === 'tokens' || kind === 'component') && ['text', 'ink', 'color.text'].includes(key)) styles.push(`--custom-text:${cleanValue};color:${cleanValue}`);
      if ((kind === 'visual' || kind === 'tokens' || kind === 'component') && ['muted', 'subtle', 'color.muted'].includes(key)) styles.push(`--custom-muted:${cleanValue}`);
      if ((kind === 'visual' || kind === 'tokens' || kind === 'component') && ['border', 'border.color', 'stroke', 'outline'].includes(key)) styles.push(`--custom-border:${cleanValue};border-color:${cleanValue}`);
      if ((kind === 'visual' || kind === 'tokens' || kind === 'component') && ['gradient.value', 'gradient_value', 'custom_gradient'].includes(key)) styles.push(`--custom-gradient:${cleanValue};background:${cleanValue}`);
      if (kind === 'visual' && key === 'gradient' && /gradient\(/i.test(String(value))) styles.push(`--custom-gradient:${cleanValue};background:${cleanValue}`);
      if ((kind === 'visual' || kind === 'tokens' || kind === 'component') && ['radius', 'shape.radius'].includes(key)) styles.push(`--custom-radius:${cleanValue};border-radius:${cleanValue}`);
      if ((kind === 'visual' || kind === 'tokens' || kind === 'component') && ['shadow', 'shadow.value'].includes(key)) styles.push(`--custom-shadow:${cleanValue};box-shadow:${cleanValue}`);
      if ((kind === 'visual' || kind === 'tokens' || kind === 'component') && ['padding', 'space.padding'].includes(key)) styles.push(`--custom-padding:${cleanValue};padding:${cleanValue}`);
      if ((kind === 'visual' || kind === 'tokens' || kind === 'component') && ['gap', 'space.gap'].includes(key)) styles.push(`--custom-gap:${cleanValue};gap:${cleanValue}`);
      if (canSizeComponent && ['min_height', 'min-height'].includes(key)) styles.push(`--component-min-height:${cleanValue};min-height:${cleanValue}`);
      if (canSizeComponent && key === 'height') styles.push(`--component-height:${cleanValue};height:${cleanValue}`);
      if (canSizeComponent && key === 'width') styles.push(`--component-width:${cleanValue};width:${cleanValue}`);
      if (canSizeComponent && ['max_width', 'max-width'].includes(key)) styles.push(`--component-max-width:${cleanValue};max-width:${cleanValue}`);
      if (canSizeComponent && ['padding_x', 'padding-x', 'space.padding_x'].includes(key)) styles.push(`--component-padding-x:${cleanValue}`);
      if (canSizeComponent && ['padding_y', 'padding-y', 'space.padding_y'].includes(key)) styles.push(`--component-padding-y:${cleanValue}`);
      if (canSizeComponent && ['min_width', 'min-width'].includes(key)) styles.push(`--component-min-width:${cleanValue};min-width:${cleanValue}`);
      if (canSizeComponent && ['max_height', 'max-height'].includes(key)) styles.push(`--component-max-height:${cleanValue};max-height:${cleanValue}`);
      if (canSizeComponent && ['columns', 'layout.columns'].includes(key)) styles.push(`--component-columns:${cleanValue};--dg-columns:${cleanValue}`);
      if (canSizeComponent && ['title_width', 'title-width'].includes(key)) styles.push(`--component-title-width:${cleanValue}`);
      if (canSizeComponent && ['copy_width', 'copy-width', 'text_width', 'text-width'].includes(key)) styles.push(`--component-copy-width:${cleanValue}`);
      if (canSizeComponent && ['title_size', 'title-size'].includes(key)) styles.push(`--component-title-size:${cleanValue}`);
      if (canSizeComponent && ['copy_size', 'copy-size', 'text_size', 'text-size'].includes(key)) styles.push(`--component-copy-size:${cleanValue}`);
      if (canSizeComponent && ['transition', 'motion.transition'].includes(key)) styles.push(`--component-transition:${cleanValue};transition:${cleanValue}`);
      if (canSizeComponent && key === 'transform') styles.push(`--component-transform:${cleanValue};transform:${cleanValue}`);
      if (canSizeComponent && key === 'opacity') styles.push(`--component-opacity:${cleanValue};opacity:${cleanValue}`);
      if (kind === 'type' && ['size', 'font_size', 'font-size'].includes(key)) styles.push(`font-size:${cleanValue}`);
      if (kind === 'type' && ['fluid', 'fluid_size'].includes(key)) styles.push(`font-size:clamp(${cleanValue})`);
      if (kind === 'type' && ['leading', 'line_height', 'line-height'].includes(key)) styles.push(`line-height:${cleanValue}`);
      if (kind === 'type' && ['tracking', 'letter_spacing', 'letter-spacing'].includes(key)) styles.push(`letter-spacing:${cleanValue}`);
      if (kind === 'states' && key === 'hover.bg') styles.push(`--state-hover-bg:${cleanValue}`);
      if (kind === 'states' && key === 'hover.text') styles.push(`--state-hover-text:${cleanValue}`);
      if (kind === 'states' && key === 'hover.shadow') styles.push(`--state-hover-shadow:${cleanValue}`);
      if (kind === 'states' && key === 'focus.ring') styles.push(`--state-focus-ring:${cleanValue}`);
      if (kind === 'compose' && key === 'columns') styles.push(`--dg-columns:${cleanValue}`);
      if (kind === 'compose' && key === 'gap') styles.push(`--dg-gap:${cleanValue}`);
      if (kind === 'compose' && key === 'grid_min') styles.push(`--grid-min:${cleanValue}`);
      if (kind === 'compose' && key === 'max_width') styles.push(`--dg-max-width:${cleanValue}`);
      if (kind === 'compose' && ['template', 'grid_template', 'grid-template'].includes(key)) styles.push(`--dg-template:${cleanValue}`);
      if (kind === 'motion' && key === 'speed') styles.push(`--dg-motion-speed:${cleanValue}`);
      if (kind === 'type' && key === 'measure') styles.push(`--dg-type-measure:${cleanValue}`);
      if (kind === 'type' && key === 'weight') styles.push(`--dg-type-weight:${cleanValue}`);
      if (kind === 'canvas' && key === 'content_width') styles.push(`--content-width:${cleanValue}`);
      if (kind === 'canvas' && key === 'wide_width') styles.push(`--wide-width:${cleanValue}`);
      if (kind === 'canvas' && key === 'readable_width') styles.push(`--readable-width:${cleanValue}`);
      if (kind === 'responsive' && key === 'columns') styles.push(`--dg-responsive-columns:${cleanValue}`);
      if (kind === 'responsive' && key === 'desktop.columns') styles.push(`--bp-desktop-columns:${cleanValue}`);
      if (kind === 'responsive' && key === 'laptop.columns') styles.push(`--bp-laptop-columns:${cleanValue}`);
      if (kind === 'responsive' && key === 'tablet.columns') styles.push(`--bp-tablet-columns:${cleanValue}`);
      if (kind === 'responsive' && key === 'mobile.columns') styles.push(`--bp-mobile-columns:${cleanValue}`);
      if (kind === 'responsive' && key === 'desktop.padding') styles.push(`--bp-desktop-padding:${cleanValue}`);
      if (kind === 'responsive' && key === 'laptop.padding') styles.push(`--bp-laptop-padding:${cleanValue}`);
      if (kind === 'responsive' && key === 'tablet.padding') styles.push(`--bp-tablet-padding:${cleanValue}`);
      if (kind === 'responsive' && key === 'mobile.padding') styles.push(`--bp-mobile-padding:${cleanValue}`);
      if (kind === 'responsive' && key === 'desktop.gap') styles.push(`--bp-desktop-gap:${cleanValue}`);
      if (kind === 'responsive' && key === 'laptop.gap') styles.push(`--bp-laptop-gap:${cleanValue}`);
      if (kind === 'responsive' && key === 'tablet.gap') styles.push(`--bp-tablet-gap:${cleanValue}`);
      if (kind === 'responsive' && key === 'mobile.gap') styles.push(`--bp-mobile-gap:${cleanValue}`);
      if (kind === 'art' && key === 'texture_opacity') styles.push(`--dg-texture-opacity:${cleanValue}`);
      if (kind === 'interaction' && key === 'focus_strength') styles.push(`--dg-focus-strength:${cleanValue}`);
    }
  }
  return styles.join(';');
}

function designDataAttrs(blocks) {
  const attrs = [];
  const blockNames = [];
  for (const block of blocks || []) {
    const kind = String(block.kind || '');
    if (kind) {
      blockNames.push(kind);
      const summary = designSettingsSummary(block);
      if (summary) attrs.push(`data-dg-${escapeAttr(kind)}="${escapeAttr(summary)}"`);
    }
    if (block.kind === 'creative') {
      const signature = settingValue(block, 'signature');
      const freedom = settingValue(block, 'freedom');
      const uniqueness = settingValue(block, 'uniqueness');
      const reference = settingValue(block, 'reference');
      const avoidRepetition = settingValue(block, 'avoid_repetition');
      if (signature) attrs.push(`data-ai-signature="${escapeAttr(signature)}"`);
      if (freedom) attrs.push(`data-ai-freedom="${escapeAttr(freedom)}"`);
      if (uniqueness) attrs.push(`data-ai-uniqueness="${escapeAttr(uniqueness)}"`);
      if (reference) attrs.push(`data-ai-reference="${escapeAttr(reference)}"`);
      if (avoidRepetition) attrs.push(`data-ai-avoid-repetition="${escapeAttr(avoidRepetition)}"`);
    }
    if (block.kind === 'brand') {
      const voice = settingValue(block, 'voice');
      const personality = settingValue(block, 'personality');
      if (voice) attrs.push(`data-ai-brand-voice="${escapeAttr(voice)}"`);
      if (personality) attrs.push(`data-ai-brand-personality="${escapeAttr(personality)}"`);
    }
    if (block.kind === 'art') {
      const direction = settingValue(block, 'direction');
      const motif = settingValue(block, 'motif');
      if (direction) attrs.push(`data-ai-art-direction="${escapeAttr(direction)}"`);
      if (motif) attrs.push(`data-ai-art-motif="${escapeAttr(motif)}"`);
    }
  }
  if (blockNames.length > 0) attrs.unshift(`data-dg-blocks="${escapeAttr(Array.from(new Set(blockNames)).join(' '))}"`);
  return attrs.join(' ');
}

function designAttrs(baseClass, extraClasses = [], blocks = [], attrStyle = '') {
  const classList = []
    .concat(baseClass ? [baseClass] : [])
    .concat(extraClasses || [])
    .concat(designClassList(blocks))
    .filter(Boolean);
  const classAttr = classList.length > 0 ? ` class="${escapeAttr(Array.from(new Set(classList)).join(' '))}"` : '';
  const style = designStyleVars(blocks);
  const joinedStyle = [style, attrStyle].filter(Boolean).join(';');
  const styleAttr = joinedStyle ? ` style="${joinedStyle}"` : '';
  const dataAttrs = designDataAttrs(blocks);
  return `${classAttr}${styleAttr}${dataAttrs ? ` ${dataAttrs}` : ''}`;
}

function canvasAttributes(canvas) {
  const blocks = canvas ? [canvas] : [];
  return designAttrs('amana-page', [], blocks);
}

function renderIcon(name, className = 'amana-icon') {
  const raw = String(name || '').trim();
  if (!raw) return '';
  const iconName = escapeAttr(raw);
  if (/^[a-z0-9-]+:[a-z0-9:_-]+$/i.test(raw)) {
    return `<iconify-icon class="${escapeAttr(className)}" icon="${iconName}" aria-hidden="true"></iconify-icon>`;
  }
  const fallback = {
    check: '✓',
    close: '×',
    x: '×',
    menu: '☰',
    search: '⌕',
    arrow: '→',
    'arrow-right': '→',
    'arrow-left': '←',
    plus: '+',
    minus: '-',
    star: '★'
  };
  return `<span class="${escapeAttr(className)}" aria-hidden="true">${fallback[raw] || iconName}</span>`;
}

function renderStandardComponent(tag, classes, attributes, children, clientStates, dataVar) {
  const { designBlocks, renderChildren } = splitDesignChildren(children);
  const inner = renderChildren.map(c => generateEjs(c, clientStates, dataVar)).join('');
  const attrDesign = designAttributeStyleVars(attributes || []);
  const attrsFor = (base) => designAttrs(base, classes, designBlocks, attrDesign.style);
  if (tag === 'Button') {
    const href = getAttr(attributes, 'href', '');
    const label = getAttr(attributes, 'label', '') || getAttr(attributes, 'text', '');
    const variant = getAttr(attributes, 'variant', 'primary');
    const size = getAttr(attributes, 'size', 'md');
    const intent = getAttr(attributes, 'intent', 'default');
    const icon = getAttr(attributes, 'icon', '');
    const body = inner || label;
    const iconMarkup = icon ? renderIcon(icon, 'amana-btn-icon') : '';
    const content = `${iconMarkup}<span>${body}</span>`;
    return href
      ? `<a${attrsFor(`amana-btn amana-btn-${variant} amana-btn-${size} amana-btn-intent-${intent}`)} href="${escapeAttr(href)}">${content}</a>`
      : `<button${attrsFor(`amana-btn amana-btn-${variant} amana-btn-${size} amana-btn-intent-${intent}`)} type="button">${content}</button>`;
  }
  if (tag === 'Card' || tag === 'FeatureCard' || tag === 'PricingCard') {
    const eyebrow = getAttr(attributes, 'eyebrow', '');
    const badge = getAttr(attributes, 'badge', '');
    const title = getAttr(attributes, 'title', '');
    const subtitle = getAttr(attributes, 'subtitle', '') || getAttr(attributes, 'description', '');
    const price = getAttr(attributes, 'price', '');
    const meta = getAttr(attributes, 'meta', '');
    const actionLabel = getAttr(attributes, 'action_label', '');
    const actionHref = getAttr(attributes, 'action_href', '#');
    const density = getAttr(attributes, 'density', 'comfortable');
    const kind = tag === 'PricingCard' ? ' amana-pricing-card' : tag === 'FeatureCard' ? ' amana-feature-card' : '';
    const cardTop = (eyebrow || badge || meta)
      ? `<div class="amana-card-top">${eyebrow ? `<span class="amana-eyebrow">${eyebrow}</span>` : ''}${badge ? `<span class="amana-badge">${badge}</span>` : ''}${meta ? `<span class="amana-card-meta">${meta}</span>` : ''}</div>`
      : '';
    const action = actionLabel ? `<a class="amana-card-action" href="${escapeAttr(actionHref)}">${actionLabel}</a>` : '';
    return `<article${attrsFor(`amana-card${kind} amana-card-density-${density}`)}>${cardTop}${title ? `<h3>${title}</h3>` : ''}${subtitle ? `<p class="amana-muted">${subtitle}</p>` : ''}${price ? `<div class="amana-price">${price}</div>` : ''}${inner}${action}</article>`;
  }
  if (tag === 'Container') {
    const width = getAttr(attributes, 'width', 'default');
    return `<div${attrsFor(`amana-container amana-container-${width}`)}>${inner}</div>`;
  }
  if (tag === 'Section') {
    const eyebrow = getAttr(attributes, 'eyebrow', '');
    const title = getAttr(attributes, 'title', '');
    const subtitle = getAttr(attributes, 'subtitle', '') || getAttr(attributes, 'description', '');
    const header = (eyebrow || title || subtitle)
      ? `<header class="amana-section-head">${eyebrow ? `<p class="amana-eyebrow">${eyebrow}</p>` : ''}${title ? `<h2>${title}</h2>` : ''}${subtitle ? `<p class="amana-section-copy">${subtitle}</p>` : ''}</header>`
      : '';
    return `<section${attrsFor('amana-section')}>${header}${inner}</section>`;
  }
  if (tag === 'Grid') {
    const min = getAttr(attributes, 'min', '16rem');
    const columns = getAttr(attributes, 'columns', '');
    const rawAttrs = attrsFor('amana-grid');
    const gridVars = `--grid-min:${escapeAttr(min)};${columns ? `--dg-columns:${escapeAttr(columns)};` : ''}`;
    const gridAttrs = rawAttrs.includes(' style="')
      ? rawAttrs.replace(' style="', ` style="${gridVars}`)
      : `${rawAttrs} style="${gridVars}"`;
    return `<div${gridAttrs}>${inner}</div>`;
  }
  if (tag === 'Stack') {
    const gap = getAttr(attributes, 'gap', 'md');
    return `<div${attrsFor(`amana-stack amana-stack-gap-${gap}`)}>${inner}</div>`;
  }
  if (tag === 'Navbar') {
    const brand = getAttr(attributes, 'brand', '<%= title %>');
    const sticky = getAttr(attributes, 'sticky', 'false') === 'true';
    return `<nav${attrsFor(`amana-navbar${sticky ? ' amana-navbar-sticky' : ''}`)}><a class="amana-brand" href="/">${brand}</a><div class="amana-navlinks">${inner}</div></nav>`;
  }
  if (tag === 'Hero') {
    const eyebrow = getAttr(attributes, 'eyebrow', '');
    const title = getAttr(attributes, 'title', '');
    const subtitle = getAttr(attributes, 'subtitle', '');
    const media = getAttr(attributes, 'media', '');
    const proof = getAttr(attributes, 'proof', '');
    const text = `<div class="amana-hero-content">${eyebrow ? `<p class="amana-eyebrow">${eyebrow}</p>` : ''}${title ? `<h1>${title}</h1>` : ''}${subtitle ? `<p class="amana-hero-copy">${subtitle}</p>` : ''}${proof ? `<p class="amana-hero-proof">${proof}</p>` : ''}<div class="amana-hero-actions">${inner}</div></div>`;
    const mediaMarkup = media ? `<div class="amana-hero-media" style="background-image:url('${escapeAttr(media)}')"></div>` : '';
    return `<section${attrsFor('amana-hero')}>${text}${mediaMarkup}</section>`;
  }
  if (tag === 'FormField') {
    const name = getAttr(attributes, 'name', '');
    const label = getAttr(attributes, 'label', name);
    const placeholder = getAttr(attributes, 'placeholder', '');
    const type = getAttr(attributes, 'type', 'text');
    return `<label${attrsFor('amana-field')}><span>${label}</span><input type="${type}" name="${name}" id="${name}" placeholder="${placeholder}"></label>`;
  }
  if (tag === 'Alert') {
    const tone = getAttr(attributes, 'tone', 'info');
    return `<div${attrsFor(`amana-alert amana-alert-${tone}`)}>${inner || getAttr(attributes, 'message', '')}</div>`;
  }
  if (tag === 'Footer') return `<footer${attrsFor('amana-footer')}>${inner}</footer>`;
  if (tag === 'Icon') {
    const name = getAttr(attributes, 'name', '') || getAttr(attributes, 'icon', '');
    return renderIcon(name, Array.from(new Set(['amana-icon'].concat(classes || []))).join(' '));
  }
  if (tag === 'Modal') {
    const open = getAttr(attributes, 'open', 'modal_open');
    return `<div${attrsFor('amana-modal')} x-show="${open}"><div class="amana-modal-panel">${inner}</div></div>`;
  }
  if (tag === 'Tabs') return `<div${attrsFor('amana-tabs')}>${inner}</div>`;
  if (tag === 'Badge') {
    const label = getAttr(attributes, 'label', '') || inner;
    const tone = getAttr(attributes, 'tone', 'neutral');
    return `<span${attrsFor(`amana-badge amana-badge-${tone}`)}>${label}</span>`;
  }
  if (tag === 'Kpi' || tag === 'Stat') {
    const label = getAttr(attributes, 'label', '');
    const value = getAttr(attributes, 'value', '');
    const trend = getAttr(attributes, 'trend', '');
    return `<article${attrsFor('amana-kpi')}>${label ? `<span class="amana-kpi-label">${label}</span>` : ''}${value ? `<strong class="amana-kpi-value">${value}</strong>` : ''}${trend ? `<span class="amana-kpi-trend">${trend}</span>` : ''}${inner}</article>`;
  }
  if (tag === 'LogoCloud') {
    const title = getAttr(attributes, 'title', '');
    return `<section${attrsFor('amana-logo-cloud')}>${title ? `<p class="amana-muted">${title}</p>` : ''}<div class="amana-logo-row">${inner}</div></section>`;
  }
  if (tag === 'TestimonialCard') {
    const quote = getAttr(attributes, 'quote', '');
    const author = getAttr(attributes, 'author', '');
    const role = getAttr(attributes, 'role', '');
    return `<figure${attrsFor('amana-testimonial')}>${quote ? `<blockquote>${quote}</blockquote>` : inner}${author || role ? `<figcaption>${author ? `<strong>${author}</strong>` : ''}${role ? `<span>${role}</span>` : ''}</figcaption>` : ''}</figure>`;
  }
  if (tag === 'Timeline') return `<ol${attrsFor('amana-timeline')}>${inner}</ol>`;
  if (tag === 'TimelineItem') {
    const title = getAttr(attributes, 'title', '');
    const meta = getAttr(attributes, 'meta', '');
    return `<li${attrsFor('amana-timeline-item')}>${meta ? `<span class="amana-card-meta">${meta}</span>` : ''}${title ? `<h3>${title}</h3>` : ''}${inner}</li>`;
  }
  if (tag === 'EmptyState') {
    const title = getAttr(attributes, 'title', '');
    const description = getAttr(attributes, 'description', '');
    const actionLabel = getAttr(attributes, 'action_label', '');
    const actionHref = getAttr(attributes, 'action_href', '#');
    return `<section${attrsFor('amana-empty-state')}>${title ? `<h2>${title}</h2>` : ''}${description ? `<p>${description}</p>` : ''}${inner}${actionLabel ? `<a class="amana-btn amana-btn-primary" href="${escapeAttr(actionHref)}">${actionLabel}</a>` : ''}</section>`;
  }
  if (tag === 'Split') return `<div${attrsFor('amana-split')}>${inner}</div>`;
  if (tag === 'Cluster') return `<div${attrsFor('amana-cluster')}>${inner}</div>`;
  if (tag === 'Sidebar') return `<aside${attrsFor('amana-sidebar')}>${inner}</aside>`;
  return null;
}

function generateEjs(element, clientStates, dataVar = null) {
  if (!element) return '';
  if (element.DesignBlock !== undefined) return '';
  
  if (element.Text !== undefined) {
    const txt = element.Text;
    if (textReferencesClientState(txt, clientStates)) {
      const content = txt.substring(2, txt.length - 1);
      const jsTemplate = content.replace(/{/g, '${');
      return `<span x-text="\`${jsTemplate}\` shadow-smooth"></span>`;
    } else if (txt.startsWith('f"') && txt.endsWith('"')) {
      const content = txt.substring(2, txt.length - 1);
      
      // Replace User.current variations with currentUser before converting to template
      let jsTemplate = content
        .replace(/\\{User\\.current\\.name\\}/g, '${currentUser.name}')
        .replace(/\\{User\\.current\\.email\\}/g, '${currentUser.email}')
        .replace(/\\{User\\.current\\.role\\}/g, '${currentUser.role}')
        .replace(/\\{User\\.current\\.id\\}/g, '${currentUser.id}')
        .replace(/\\{User\\.current\\}/g, '${currentUser}');
      
      // Replace remaining { with ${
      jsTemplate = jsTemplate.replace(/{/g, '${');
      
      return `<%= \`${jsTemplate}\` %>`;
    }
    return txt;
  }
  
  if (element.FormattedText !== undefined) {
    const exprs = element.FormattedText;
    if (exprs.some(e => referencesClientState(e, clientStates))) {
      const jsExpr = exprs.map(compileExpressionToJs).join(' + ');
      return `<span x-text="${jsExpr}"></span>`;
    } else {
      return exprs.map(e => `<%= ${compileExpressionToJs(e)} %>`).join('');
    }
  }
  
  if (element.ForEach !== undefined) {
    const { item_var, list_expr, body } = element.ForEach;
    const inner = body.map(c => generateEjs(c, clientStates, item_var)).join('');
    return `<% for (let ${item_var} of ${compileExpressionToJs(list_expr)}) { %>\n${inner}<% } %>\n`;
  }
  
  if (element.IfBlock !== undefined) {
    const { condition, then_branch, else_branch } = element.IfBlock;
    const thenHtml = then_branch.map(c => generateEjs(c, clientStates, dataVar)).join('');
    const elseHtml = else_branch ? `<% } else { %>\n${else_branch.map(c => generateEjs(c, clientStates, dataVar)).join('')}` : '';
    return `<% if (${compileExpressionToJs(condition)}) { %>\n${thenHtml}${elseHtml}<% } %>\n`;
  }
  
  if (element.FormBlock !== undefined) {
    const { fields, connect_action, ui, submit_label, field_options } = element.FormBlock;
    const actionPath = `/form-submit/${connect_action.replace(/\./g, '/').toLowerCase()}`;
    let formInner = '  <input type="hidden" name="_csrf" value="<%= csrfToken %>">\n';
    const actionName = connect_action.split('.').pop().toLowerCase();
    const fieldConfig = new Map((field_options || []).map(opt => [opt.name.toLowerCase(), opt]));
    for (const f of fields) {
      const fieldLower = f.toLowerCase();
      if (fieldLower === 'id' && dataVar) {
        formInner += `  <input type="hidden" name="${f}" value="<%= ${dataVar}.id %>">\n`;
        continue;
      }
      const opts = fieldConfig.get(fieldLower) || {};
      const inputType = opts.input_type || (fieldLower.includes('password') ? 'password' : (fieldLower.includes('email') ? 'email' : 'text'));
      const label = opts.label || f;
      const placeholder = opts.placeholder ? ` placeholder="${opts.placeholder}"` : '';
      const help = opts.help ? `\n    <small class="amana-help">${opts.help}</small>` : '';
      const requiredAttr = opts.required === false || (actionName === 'update' && inputType === 'password') ? '' : ' required';
      
      if (inputType === 'textarea') {
        const textareaValue = dataVar
          ? `<%= typeof ${dataVar} !== 'undefined' && ${dataVar}.${f} !== undefined ? ${dataVar}.${f} : '' %>`
          : '';
        formInner += `  <label class="amana-field" for="${f}">\n    <span>${label}</span>\n    <textarea class="amana-form-control" id="${f}" name="${f}"${placeholder}${requiredAttr} rows="4">${textareaValue}</textarea>${help}\n  </label>\n`;
      } else {
        const valueAttr = dataVar && inputType !== 'password'
          ? ` value="<%= typeof ${dataVar} !== 'undefined' && ${dataVar}.${f} !== undefined ? ${dataVar}.${f} : '' %>"`
          : '';
        formInner += `  <label class="amana-field" for="${f}">\n    <span>${label}</span>\n    <input class="amana-form-control" type="${inputType}" id="${f}" name="${f}"${placeholder}${valueAttr}${requiredAttr}>${help}\n  </label>\n`;
      }
    }
    const submitText = submit_label || 'Submit';
    formInner += `  <button type="submit" class="amana-btn amana-btn-primary">${submitText}</button>\n`;
    const formClass = ui === 'card' ? ' class="amana-form-card"' : '';
    return `<form${formClass} action="${actionPath}" method="POST">\n${formInner}</form>\n`;
  }
  
  if (element.Chart !== undefined) {
    const { data_expr, chart_type, x_field, y_field } = element.Chart;
    return `<div class="chart-container mb-4" style="position: relative; height:40vh; width:80vw">\n  <canvas id="chart_${data_expr}"></canvas>\n</div>\n\
<script>\n\
document.addEventListener('DOMContentLoaded', () => {\n\
  const ctx = document.getElementById('chart_${data_expr}').getContext('2d');\n\
  const rawData = <%- JSON.stringify(${data_expr}) %>;\n\
  new Chart(ctx, {\n\
    type: '${chart_type}',\n\
    data: {\n\
      labels: rawData.map(row => row.${x_field}),\n\
      datasets: [{\n\
        label: 'بيانات ${data_expr}',\n\
        data: rawData.map(row => row.${y_field}),\n\
        backgroundColor: 'rgba(99, 102, 241, 0.2)',\n\
        borderColor: 'rgba(99, 102, 241, 1)',\n\
        borderWidth: 2\n\
      }]\n\
    },\n\
    options: {\n\
      responsive: true,\n\
      maintainAspectRatio: false\n\
    }\n\
  });\n\
});\n\
</script>\n`;
  }
  
  if (element.Element !== undefined) {
    const { tag, classes, attributes, children } = element.Element;
    const standard = renderStandardComponent(tag, classes, attributes, children, clientStates, dataVar);
    if (standard !== null) return standard;
    const { designBlocks, renderChildren } = splitDesignChildren(children);
    const attrDesign = designAttributeStyleVars(attributes || []);
    const classStr = designAttrs(classes.join(' '), [], designBlocks, attrDesign.style);
    let attrs = '';
    const eventKeys = ['click', 'submit', 'change', 'input', 'keydown', 'keyup', 'focus', 'blur', 'mouseenter', 'mouseleave'];
    for (const [key, expr] of attributes) {
      if (attrDesign.consumed.has(key)) continue;
      if (key === 'bind' || key === 'model') {
        if (expr.Identifier !== undefined) {
          const id = expr.Identifier;
          if (clientStates.some(s => s.name === id) || key === 'model') {
            attrs += ` x-model="${id}" name="${id}" id="${id}"`;
          } else {
            attrs += ` value="<%= typeof ${id} !== 'undefined' ? ${id} : '' %>" name="${id}" id="${id}"`;
          }
        }
      } else if (eventKeys.includes(key)) {
        attrs += ` x-on:${key}="${compileExpressionToJs(expr)}"`;
      } else if (key === 'show') {
        attrs += ` x-show="${compileExpressionToJs(expr)}"`;
      } else if (key === 'text') {
        attrs += ` x-text="${compileExpressionToJs(expr)}"`;
      } else if (key === 'init') {
        const code = expr.StringLiteral !== undefined ? expr.StringLiteral : compileExpressionToJs(expr);
        attrs += ` x-init="${escapeAttr(code)}"`;
      } else if (['disabled', 'checked', 'selected', 'readonly'].includes(key)) {
        attrs += ` :${key}="${compileExpressionToJs(expr)}"`;
      } else {
        attrs += ` ${key}="<%= ${compileExpressionToJs(expr)} %>"`;
      }
    }
    const inner = renderChildren.map(c => generateEjs(c, clientStates, dataVar)).join('');
    return `<${tag}${classStr}${attrs}>${inner}</${tag}>`;
  }
  
  return '';
}

function generateTableDdl(model) {
  const hasExplicitPrimaryKey = model.fields.some(f => f.is_primary_key);
  let columnsDdl = hasExplicitPrimaryKey ? [] : ['"id" INTEGER PRIMARY KEY AUTOINCREMENT'];
  for (const f of model.fields) {
    if (f.name.toLowerCase() === 'id') continue;
    let typeStr = 'TEXT';
    const dt = f.data_type;
    if (dt === 'Int' || dt === 'Bool') typeStr = 'INTEGER';
    else if (dt === 'Float' || dt === 'Money') typeStr = 'REAL';
    
    let fieldDdl = `${quoteSqlIdentifier(f.name.toLowerCase())} ${typeStr}`;
    if (f.is_primary_key) {
      fieldDdl += ' PRIMARY KEY';
      if (dt === 'Int') fieldDdl += ' AUTOINCREMENT';
    }
    if (f.is_unique) fieldDdl += ' UNIQUE';
    if (f.is_required && !f.is_primary_key && (f.default_value === null || f.default_value === undefined)) fieldDdl += ' NOT NULL';
    if (f.default_value !== null && f.default_value !== undefined) {
      fieldDdl += ` DEFAULT ${sqlDefaultLiteral(f.default_value, dt)}`;
    }
    const checks = [];
    if (f.min_value !== null && f.min_value !== undefined) checks.push(sqlConstraintExpression(f, '>=', f.min_value));
    if (f.max_value !== null && f.max_value !== undefined) checks.push(sqlConstraintExpression(f, '<=', f.max_value));
    if (checks.length > 0) fieldDdl += ` CHECK (${checks.join(' AND ')})`;
    if (f.foreign_key) {
      const deleteAction = f.on_delete || 'CASCADE';
      fieldDdl += ` REFERENCES ${quoteSqlIdentifier(f.foreign_key[0].toLowerCase())}(${quoteSqlIdentifier(f.foreign_key[1].toLowerCase())}) ON DELETE ${deleteAction}`;
    }
    columnsDdl.push(fieldDdl);
  }
  return `CREATE TABLE IF NOT EXISTS ${quoteSqlIdentifier(model.table_name)} (\n  ${columnsDdl.join(',\n  ')}\n);`;
}

function quoteSqlIdentifier(identifier) {
  return `"${String(identifier).replace(/"/g, '""')}"`;
}

function sqlDefaultLiteral(value, dataType) {
  const text = String(value);
  if (/^CURRENT_(TIMESTAMP|DATE|TIME)$/i.test(text)) return text.toUpperCase();
  if (dataType === 'Int' || dataType === 'Float' || dataType === 'Money' || dataType === 'Bool') return text;
  return `'${text.replace(/'/g, "''")}'`;
}

function isTextDataType(dataType) {
  return dataType === 'Str' || dataType === 'Email' || dataType === 'Password' || dataType === 'DateTime' || typeof dataType === 'object';
}

function sqlConstraintExpression(field, operator, value) {
  const col = quoteSqlIdentifier(field.name.toLowerCase());
  return isTextDataType(field.data_type)
    ? `length(${col}) ${operator} ${value}`
    : `${col} ${operator} ${value}`;
}

function generateColumnDdl(field) {
  let typeStr = 'TEXT';
  const dt = field.data_type;
  if (dt === 'Int' || dt === 'Bool') typeStr = 'INTEGER';
  else if (dt === 'Float' || dt === 'Money') typeStr = 'REAL';
  
  let fieldDdl = `${quoteSqlIdentifier(field.name.toLowerCase())} ${typeStr}`;
  if (field.is_unique) fieldDdl += ' UNIQUE';
  if (field.is_required && (field.default_value === null || field.default_value === undefined)) fieldDdl += ' NOT NULL';
  if (field.default_value !== null && field.default_value !== undefined) {
    fieldDdl += ` DEFAULT ${sqlDefaultLiteral(field.default_value, dt)}`;
  }
  const checks = [];
  if (field.min_value !== null && field.min_value !== undefined) checks.push(sqlConstraintExpression(field, '>=', field.min_value));
  if (field.max_value !== null && field.max_value !== undefined) checks.push(sqlConstraintExpression(field, '<=', field.max_value));
  if (checks.length > 0) fieldDdl += ` CHECK (${checks.join(' AND ')})`;
  if (field.foreign_key) {
    const deleteAction = field.on_delete || 'CASCADE';
    fieldDdl += ` REFERENCES ${quoteSqlIdentifier(field.foreign_key[0].toLowerCase())}(${quoteSqlIdentifier(field.foreign_key[1].toLowerCase())}) ON DELETE ${deleteAction}`;
  }
  return fieldDdl;
}

function validateRuntimeFieldValue(model, fieldName, value, { partial = false } = {}) {
  const modelField = model.fields.find(f => f.name.toLowerCase() === fieldName.toLowerCase());
  if (!modelField) return;
  const missing = value === undefined || value === null || value === '';
  if (modelField.is_required && !partial && missing) {
    throw new Error(`Field '${fieldName}' is required.`);
  }
  if (missing) return;

  const dt = modelField.data_type;
  if ((dt === 'Int' || dt === 'Float' || dt === 'Money') && Number.isNaN(Number(value))) {
    throw new Error(`Field '${fieldName}' must be numeric.`);
  }
  if (dt === 'Email' && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(String(value))) {
    throw new Error(`Field '${fieldName}' must be a valid email address.`);
  }

  if (modelField.min_value !== null && modelField.min_value !== undefined) {
    if (dt === 'Int' || dt === 'Float' || dt === 'Money') {
      if (Number(value) < modelField.min_value) throw new Error(`Field '${fieldName}' must be at least ${modelField.min_value}.`);
    } else if (String(value).length < modelField.min_value) {
      throw new Error(`Field '${fieldName}' must be at least ${modelField.min_value} characters.`);
    }
  }
  if (modelField.max_value !== null && modelField.max_value !== undefined) {
    if (dt === 'Int' || dt === 'Float' || dt === 'Money') {
      if (Number(value) > modelField.max_value) throw new Error(`Field '${fieldName}' must be at most ${modelField.max_value}.`);
    } else if (String(value).length > modelField.max_value) {
      throw new Error(`Field '${fieldName}' must be at most ${modelField.max_value} characters.`);
    }
  }
}

const DEFAULT_QUERY_LIMIT = Math.max(1, Math.min(Number(process.env.AMANA_DEFAULT_QUERY_LIMIT || 100), 1000));

function isPaginationArg(key) {
  return ['limit', 'offset', 'page'].includes(String(key || '').toLowerCase());
}

function findNamedQueryArg(queryArgs, key) {
  const found = queryArgs.find(([argKey]) => argKey && String(argKey).toLowerCase() === key);
  return found ? found[1] : undefined;
}

function appendPaginationClause(sql, paramsJs, queryArgs) {
  const limit = findNamedQueryArg(queryArgs, 'limit');
  const offset = findNamedQueryArg(queryArgs, 'offset');
  const page = findNamedQueryArg(queryArgs, 'page');

  if (offset !== undefined && page !== undefined) {
    throw new Error("Query execution failed: use either 'offset' or 'page', not both.");
  }

  if (limit !== undefined) {
    sql += ' LIMIT ?';
    paramsJs.push(limit);
  } else {
    sql += ` LIMIT ${DEFAULT_QUERY_LIMIT}`;
  }

  if (offset !== undefined) {
    sql += ' OFFSET ?';
    paramsJs.push(offset);
  } else if (page !== undefined) {
    sql += ' OFFSET ((? - 1) * ?)';
    paramsJs.push(page);
    paramsJs.push(limit !== undefined ? limit : { Number: DEFAULT_QUERY_LIMIT });
  }

  return sql;
}

function generateSafeQuery(models, modelName, queryMethod, queryArgs) {
  const tableKey = modelName.toLowerCase();
  const model = models.find(m => m.table_name === tableKey);
  if (!model) {
    throw new Error(`Security Exception: Access to table '${modelName}' is restricted or table is undefined.`);
  }

  const tableSql = quoteSqlIdentifier(model.table_name);
  let sql = `SELECT * FROM ${tableSql}`;
  let paramsJs = [];

  switch (queryMethod) {
    case 'all':
      for (const [argKey] of queryArgs) {
        if (!argKey || !isPaginationArg(argKey)) {
          throw new Error("Query execution failed: 'all' accepts only named pagination arguments (limit, offset, page). Use filter(...) for column filters.");
        }
      }
      sql = appendPaginationClause(sql, paramsJs, queryArgs);
      break;
    case 'find':
      sql += ' WHERE "id" = ? LIMIT 1';
      if (queryArgs.length > 0) {
        paramsJs.push(queryArgs[0][1]);
      } else {
        throw new Error("Query execution failed: 'find' method requires an identifier argument.");
      }
      break;
    case 'filter': {
      let filterClauses = [];
      for (const [colOpt, expr] of queryArgs) {
        if (colOpt) {
          if (isPaginationArg(colOpt)) {
            continue;
          }
          const col = colOpt.toLowerCase();
          const hasCol = model.fields.some(f => f.name.toLowerCase() === col) || col === 'id';
          if (!hasCol) {
            throw new Error(`SQL Compilation Error: Column '${colOpt}' not found in model '${modelName}'`);
          }
          filterClauses.push(`${quoteSqlIdentifier(col)} = ?`);
          paramsJs.push(expr);
        } else {
          throw new Error("Query execution failed: 'filter' method requires keyword arguments.");
        }
      }
      if (filterClauses.length > 0) {
        sql += ' WHERE ' + filterClauses.join(' AND ');
      }
      sql = appendPaginationClause(sql, paramsJs, queryArgs);
      break;
    }
    case 'count': {
      sql = `SELECT COUNT(*) AS count FROM ${tableSql}`;
      let filterClauses = [];
      for (const [colOpt, expr] of queryArgs) {
        if (colOpt) {
          const col = colOpt.toLowerCase();
          const hasCol = model.fields.some(f => f.name.toLowerCase() === col) || col === 'id';
          if (!hasCol) {
            throw new Error(`SQL Compilation Error: Column '${colOpt}' not found in model '${modelName}'`);
          }
          filterClauses.push(`${quoteSqlIdentifier(col)} = ?`);
          paramsJs.push(expr);
        }
      }
      if (filterClauses.length > 0) {
        sql += ' WHERE ' + filterClauses.join(' AND ');
      }
      break;
    }
    default:
      throw new Error(`Unsupported query method '${queryMethod}' for SQL Codegen.`);
  }

  return { sql, paramsJs };
}

function evalAmanaExpression(expr, req, currentUser, scope = {}) {
  if (expr === null || expr === undefined) return null;
  if (typeof expr === 'number' || typeof expr === 'boolean') return expr;
  if (typeof expr === 'string') return scope[expr] ?? expr;
  if (expr.Number !== undefined) return expr.Number;
  if (expr.StringLiteral !== undefined) return expr.StringLiteral;
  if (expr.Boolean !== undefined) return expr.Boolean;
  if (expr.Null !== undefined) return null;
  if (expr.Identifier !== undefined) {
    const id = expr.Identifier;
    if (Object.prototype.hasOwnProperty.call(scope, id)) return scope[id];
    if (id === 'currentUser') return currentUser;
    if (id === 'csrfToken') return req.session ? req.session.csrfToken : null;
    if (id === 'params') return req.params || {};
    if (id === 'query') return req.query || {};
    if (id === 'body') return req.body || {};
    return undefined;
  }
  if (expr.MemberAccess !== undefined) {
    const { object, property } = expr.MemberAccess;
    if (object && object.Identifier === 'User' && property === 'current') return currentUser;
    const obj = evalAmanaExpression(object, req, currentUser, scope);
    if (obj === null || obj === undefined) return undefined;
    return obj[property];
  }
  if (expr.Unary !== undefined) {
    const value = evalAmanaExpression(expr.Unary.expr, req, currentUser, scope);
    if (expr.Unary.op === 'not' || expr.Unary.op === '!') return !value;
    if (expr.Unary.op === '-') return -Number(value);
    throw new Error(`Unsupported unary operator '${expr.Unary.op}'.`);
  }
  if (expr.Binary !== undefined) {
    const { left, op, right } = expr.Binary;
    if (op === 'and') return Boolean(evalAmanaExpression(left, req, currentUser, scope)) && Boolean(evalAmanaExpression(right, req, currentUser, scope));
    if (op === 'or') return Boolean(evalAmanaExpression(left, req, currentUser, scope)) || Boolean(evalAmanaExpression(right, req, currentUser, scope));
    const l = evalAmanaExpression(left, req, currentUser, scope);
    const r = evalAmanaExpression(right, req, currentUser, scope);
    switch (op) {
      case '+': return l + r;
      case '-': return Number(l) - Number(r);
      case '*': return Number(l) * Number(r);
      case '/': return Number(l) / Number(r);
      case '==': return l == r;
      case '!=': return l != r;
      case '>': return l > r;
      case '<': return l < r;
      case '>=': return l >= r;
      case '<=': return l <= r;
      default: throw new Error(`Unsupported binary operator '${op}'.`);
    }
  }
  if (expr.Ternary !== undefined) {
    return evalAmanaExpression(expr.Ternary.cond, req, currentUser, scope)
      ? evalAmanaExpression(expr.Ternary.then_branch, req, currentUser, scope)
      : evalAmanaExpression(expr.Ternary.else_branch, req, currentUser, scope);
  }
  if (expr.Call !== undefined) {
    const { callee, args } = expr.Call;
    if (callee.Identifier === 'env') {
      const key = evalAmanaExpression(args[0], req, currentUser, scope);
      const fallback = args.length > 1 ? evalAmanaExpression(args[1], req, currentUser, scope) : '';
      return process.env[key] || fallback;
    }
    throw new Error('Only env(...) calls are allowed in server-side expressions.');
  }
  return null;
}

function routeErrorResponse(req, res, err) {
  const requestId = crypto.randomUUID ? crypto.randomUUID() : crypto.randomBytes(12).toString('hex');
  console.error(`[Amana Route Error ${requestId}]`, err);
  const acceptsJson = req.accepts(['html', 'json']) === 'json';
  if (acceptsJson) {
    return res.status(500).json({
      ok: false,
      error: 'Route render failed.',
      request_id: requestId
    });
  }
  return res.status(500).send(`Route render failed. Request id: ${requestId}`);
}

function expressRoutePath(pathValue) {
  return String(pathValue || '/').replace(/\[([A-Za-z_][A-Za-z0-9_]*)\]/g, ':$1');
}

class AmanaEngine {
  constructor(irPath) {
    this.irPath = irPath;
    this.ir = JSON.parse(fs.readFileSync(irPath, 'utf8'));
    this.dbPath = path.resolve(path.dirname(irPath), this.ir.app.db_path);
    this.db = new sqlite3.Database(this.dbPath);
    this.pendingRequests = new Map();
    this.reqIdCounter = 0;
    this.hooksWorker = null;
    this.plugins = new Map();
  }

  async start() {
    console.log(`[Amana Engine] Booting app: ${this.ir.app.name}...`);
    await this.runMigrations();
    if (this.shouldRunSeeds()) {
      await this.seedData();
    } else {
      console.log('[Amana Seeds] Skipped in production. Set AMANA_RUN_SEEDS=true to run seed data explicitly.');
    }
    await this.seedAdmin();
    await this.loadPlugins();
    this.setupExpress();
  }

  shouldRunSeeds() {
    const explicit = process.env.AMANA_RUN_SEEDS;
    if (explicit === 'true') return true;
    if (explicit === 'false') return false;
    return process.env.NODE_ENV !== 'production';
  }

  dbAll(sql, params = []) {
    return new Promise((resolve, reject) => {
      this.db.all(sql, params, (err, rows) => {
        if (err) reject(err);
        else resolve(rows);
      });
    });
  }

  dbGet(sql, params = []) {
    return new Promise((resolve, reject) => {
      this.db.get(sql, params, (err, row) => {
        if (err) reject(err);
        else resolve(row);
      });
    });
  }

  dbRun(sql, params = []) {
    return new Promise((resolve, reject) => {
      this.db.run(sql, params, function(err) {
        if (err) reject(err);
        else resolve(this);
      });
    });
  }

  async runMigrations() {
    console.log('[Amana Migrator] Inspecting SQLite database schema...');
    
    const getTableInfo = (tableName) => {
      return new Promise((resolve) => {
        this.db.all(`PRAGMA table_info(${quoteSqlIdentifier(tableName)})`, (err, rows) => {
          if (err || !rows) resolve([]);
          else resolve(rows);
        });
      });
    };

    const tableExists = (tableName) => {
      return new Promise((resolve) => {
        this.db.get(`SELECT name FROM sqlite_schema WHERE type='table' AND name=?`, [tableName], (err, row) => {
          if (err || !row) resolve(false);
          else resolve(true);
        });
      });
    };

    for (const model of this.ir.models) {
      const tableName = model.table_name;
      const exists = await tableExists(tableName);

      if (!exists) {
        console.log(`[Amana Migrator] Table '${tableName}' does not exist. Creating table...`);
        const ddl = generateTableDdl(model);
        await this.dbRun(ddl);
      } else {
        const dbCols = await getTableInfo(tableName);
        const dbColNames = dbCols.map(c => c.name.toLowerCase());
        const modelColNames = model.fields.map(f => f.name.toLowerCase());

        const missing = modelColNames.filter(name => !dbColNames.includes(name) && name !== 'id');
        const extra = dbColNames.filter(name => !modelColNames.includes(name) && name !== 'id');

        if (missing.length > 0 || extra.length > 0) {
          console.log(`[Amana Migrator] Schema mismatch detected for table '${tableName}'.`);
          
          let canAlterIncrementally = extra.length === 0;
          if (canAlterIncrementally) {
            for (const colName of missing) {
              const field = model.fields.find(f => f.name.toLowerCase() === colName);
              if (field.is_primary_key || field.is_unique || field.foreign_key) {
                canAlterIncrementally = false;
                break;
              }
            }
          }
          
          if (canAlterIncrementally) {
            console.log(`[Amana Migrator] Performing SQLite-compliant Incremental ALTER TABLE for table '${tableName}'...`);
            for (const colName of missing) {
              const field = model.fields.find(f => f.name.toLowerCase() === colName);
              const colDdl = generateColumnDdl(field);
              await this.dbRun(`ALTER TABLE ${quoteSqlIdentifier(tableName)} ADD COLUMN ${colDdl}`);
              console.log(`[Amana Migrator] Added column '${colName}' to table '${tableName}'.`);
            }
          } else {
            console.log(`[Amana Migrator] Performing SQLite-compliant Table Rebuild for table '${tableName}'...`);
            const tempTableName = `${tableName}_old`;
            await this.dbRun(`ALTER TABLE ${quoteSqlIdentifier(tableName)} RENAME TO ${quoteSqlIdentifier(tempTableName)}`);
            
            const ddl = generateTableDdl(model);
            await this.dbRun(ddl);

            const commonCols = ['id'];
            for (const field of model.fields) {
              const name = field.name.toLowerCase();
              if (dbColNames.includes(name)) {
                commonCols.push(name);
              }
            }

            const colsStr = commonCols.map(quoteSqlIdentifier).join(', ');
            await this.dbRun(`INSERT INTO ${quoteSqlIdentifier(tableName)} (${colsStr}) SELECT ${colsStr} FROM ${quoteSqlIdentifier(tempTableName)}`);
            await this.dbRun(`DROP TABLE ${quoteSqlIdentifier(tempTableName)}`);
            console.log(`[Amana Migrator] Rebuilt table '${tableName}' successfully.`);
          }
        }
      }
    }
  }

  async seedAdmin() {
    if (process.env.AMANA_SEED_ADMIN !== 'true') {
      return;
    }
    const hasUserTable = this.ir.models.some(m => m.name.toLowerCase() === 'user');
    if (hasUserTable) {
      const adminEmail = process.env.AMANA_ADMIN_EMAIL;
      const adminPassword = process.env.AMANA_ADMIN_PASSWORD;
      if (!adminEmail || !adminPassword) {
        throw new Error('AMANA_ADMIN_EMAIL and AMANA_ADMIN_PASSWORD are required when AMANA_SEED_ADMIN=true.');
      }
      const userTable = quoteSqlIdentifier('user');
      const adminExists = await this.dbGet(`SELECT * FROM ${userTable} WHERE "email" = ?`, [adminEmail]);
      if (!adminExists) {
        const hash = await argon2.hash(adminPassword);
        await this.dbRun(`INSERT INTO ${userTable} ("email", "password") VALUES (?, ?)`, [adminEmail, hash]);
        console.log('[Amana Engine] Seeded default administrator accounts.');
      }
    }
  }

  async seedData() {
    for (const seed of this.ir.seeds || []) {
      const model = this.ir.models.find(m => m.name.toLowerCase() === seed.model_name.toLowerCase());
      if (!model) {
        throw new Error(`Seed references unknown model '${seed.model_name}'.`);
      }
      for (const row of seed.rows || []) {
        const fields = [];
        const values = [];
        for (const [fieldName, expr] of row) {
          const field = String(fieldName).toLowerCase();
          const isImplicitId = field === 'id';
          if (!isImplicitId && !model.fields.some(f => f.name.toLowerCase() === field)) {
            throw new Error(`Seed field '${fieldName}' does not exist in model '${model.name}'.`);
          }
          const modelField = isImplicitId ? { name: 'id', data_type: 'Int' } : model.fields.find(f => f.name.toLowerCase() === field);
          let value = evalAmanaExpression(expr, { body: {}, query: {}, params: {} }, null);
          if (modelField && modelField.data_type === 'Password') {
            value = await argon2.hash(String(value || ''));
          }
          fields.push(field);
          values.push(value);
        }
        if (fields.length === 0) continue;
        for (const required of model.fields.filter(f => f.is_required && !f.is_primary_key && (f.default_value === null || f.default_value === undefined))) {
          if (!fields.includes(required.name.toLowerCase())) {
            throw new Error(`Seed for model '${model.name}' is missing required field '${required.name}'.`);
          }
        }
        const placeholders = fields.map(() => '?').join(', ');
        await this.dbRun(`INSERT OR IGNORE INTO ${quoteSqlIdentifier(model.table_name)} (${fields.map(quoteSqlIdentifier).join(', ')}) VALUES (${placeholders})`, values);
      }
    }
  }

  async loadPlugins() {
    console.log('[Amana Engine] Loading plugins...');
    const pluginsDir = path.resolve(path.dirname(this.irPath), 'custom/plugins');
    if (!fs.existsSync(pluginsDir)) {
      fs.mkdirSync(pluginsDir, { recursive: true });
      return;
    }

    const pluginFolders = fs.readdirSync(pluginsDir);
    for (const folder of pluginFolders) {
      const pluginPath = path.join(pluginsDir, folder);
      if (fs.statSync(pluginPath).isDirectory()) {
        const manifestPath = path.join(pluginPath, 'plugin.json');
        if (!fs.existsSync(manifestPath)) {
          console.warn(`[Amana Security Warning] Plugin folder '${folder}' is missing plugin.json manifest. Skipping.`);
          continue;
        }

        try {
          const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
          
          if (!verifyPluginSignature(manifest)) {
            console.error(`[Amana Security Error] Signature verification failed for plugin '${manifest.name}'. Load blocked.`);
            continue;
          }

          const appCaps = this.ir.app.capabilities || [];
          const unauthorizedCaps = manifest.capabilities.filter(cap => !appCaps.includes(cap));

          if (unauthorizedCaps.length > 0) {
            console.error(`[Amana Security Error] Plugin '${manifest.name}' requests unauthorized capabilities: [${unauthorizedCaps.join(', ')}]. Skipping.`);
            continue;
          }

          const indexJsPath = path.join(pluginPath, 'index.js');
          if (fs.existsSync(indexJsPath)) {
            const pluginModule = require(indexJsPath);
            this.plugins.set(manifest.name, {
              manifest,
              module: pluginModule
            });
            console.log(`[Amana Engine] Loaded Plugin: ${manifest.name} v${manifest.version}`);
          }
        } catch (err) {
          console.error(`[Amana Engine Error] Failed to load plugin '${folder}':`, err);
        }
      }
    }
  }

  compileApiRoutes(router) {
    if (!this.ir.app.capabilities.includes('api.rest')) {
      return;
    }

    router.use('/api', apiLimiter);

    const requireRestAccess = (req, res) => {
      if (process.env.AMANA_ALLOW_PUBLIC_REST === 'true') {
        return true;
      }
      if (this.ir.app.auth_model) {
        if (!req.session.user) {
          res.status(401).json({ ok: false, error: 'Authentication required for REST API.' });
          return false;
        }
        return true;
      }
      if (process.env.NODE_ENV === 'production') {
        res.status(403).json({ ok: false, error: 'Public REST API is disabled in production. Set AMANA_ALLOW_PUBLIC_REST=true to opt in.' });
        return false;
      }
      return true;
    };

    for (const model of this.ir.models) {
      const table = model.table_name;
      const tableSql = quoteSqlIdentifier(table);
      const base = `/api/${table}`;
      const fields = model.fields.map(f => f.name.toLowerCase());

      router.get(base, async (req, res) => {
        if (!requireRestAccess(req, res)) return;
        try {
          const requestedLimit = Number(req.query.limit || DEFAULT_QUERY_LIMIT);
          const requestedPage = Number(req.query.page || 1);
          const requestedOffset = req.query.offset !== undefined ? Number(req.query.offset) : undefined;
          const limit = Number.isFinite(requestedLimit) ? Math.max(1, Math.min(requestedLimit, 1000)) : DEFAULT_QUERY_LIMIT;
          const page = Number.isFinite(requestedPage) ? Math.max(1, requestedPage) : 1;
          const offset = Number.isFinite(requestedOffset) ? Math.max(0, requestedOffset) : (page - 1) * limit;
          const rows = await this.dbAll(`SELECT * FROM ${tableSql} LIMIT ? OFFSET ?`, [limit, offset]);
          res.json({ data: rows, page, limit, offset });
        } catch (err) {
          console.error('[Amana API Error]', err);
          res.status(500).json({ error: 'Failed to load records.' });
        }
      });

      router.get(`${base}/:id`, async (req, res) => {
        if (!requireRestAccess(req, res)) return;
        try {
          const row = await this.dbGet(`SELECT * FROM ${tableSql} WHERE "id" = ? LIMIT 1`, [req.params.id]);
          if (!row) return res.status(404).json({ error: 'Record not found.' });
          res.json({ data: row });
        } catch (err) {
          console.error('[Amana API Error]', err);
          res.status(500).json({ error: 'Failed to load record.' });
        }
      });

      router.post(base, async (req, res) => {
        if (!requireRestAccess(req, res)) return;
        try {
          const insertFields = fields.filter(f => Object.prototype.hasOwnProperty.call(req.body, f));
          if (insertFields.length === 0) return res.status(400).json({ error: 'No accepted fields submitted.' });
          const values = [];
          for (const field of insertFields) {
            const modelField = model.fields.find(f => f.name.toLowerCase() === field);
            validateRuntimeFieldValue(model, field, req.body[field]);
            if (modelField && modelField.data_type === 'Password') {
              values.push(await argon2.hash(req.body[field] || ''));
            } else {
              values.push(req.body[field]);
            }
          }
          const placeholders = insertFields.map(() => '?').join(', ');
          await this.dbRun(`INSERT INTO ${tableSql} (${insertFields.map(quoteSqlIdentifier).join(', ')}) VALUES (${placeholders})`, values);
          res.status(201).json({ ok: true });
        } catch (err) {
          console.error('[Amana API Error]', err);
          res.status(String(err.message || '').startsWith('Field ') ? 400 : 500).json({ error: err.message || 'Failed to create record.' });
        }
      });

      router.put(`${base}/:id`, async (req, res) => {
        if (!requireRestAccess(req, res)) return;
        try {
          const updateFields = fields.filter(f => Object.prototype.hasOwnProperty.call(req.body, f));
          if (updateFields.length === 0) return res.status(400).json({ error: 'No accepted fields submitted.' });
          const values = [];
          for (const field of updateFields) {
            const modelField = model.fields.find(f => f.name.toLowerCase() === field);
            validateRuntimeFieldValue(model, field, req.body[field], { partial: true });
            if (modelField && modelField.data_type === 'Password') {
              values.push(await argon2.hash(req.body[field] || ''));
            } else {
              values.push(req.body[field]);
            }
          }
          values.push(req.params.id);
          await this.dbRun(`UPDATE ${tableSql} SET ${updateFields.map(f => `${quoteSqlIdentifier(f)} = ?`).join(', ')} WHERE "id" = ?`, values);
          res.json({ ok: true });
        } catch (err) {
          console.error('[Amana API Error]', err);
          res.status(String(err.message || '').startsWith('Field ') ? 400 : 500).json({ error: err.message || 'Failed to update record.' });
        }
      });

      router.delete(`${base}/:id`, async (req, res) => {
        if (!requireRestAccess(req, res)) return;
        try {
          await this.dbRun(`DELETE FROM ${tableSql} WHERE "id" = ?`, [req.params.id]);
          res.json({ ok: true });
        } catch (err) {
          console.error('[Amana API Error]', err);
          res.status(500).json({ error: 'Failed to delete record.' });
        }
      });
    }
  }

  startHooksWorker() {
    this.hooksWorker = fork(path.join(__dirname, '../middleware/hooks-worker.js'), [], {
      execArgv: ['--max-old-space-size=64']
    });

    this.hooksWorker.on('message', (message) => {
      if (!validateHookResponse(message)) {
        console.warn('[Security Warning] IPC Hook Response contract violation - message discarded:', message);
        return;
      }

      if (message.type === 'HOOK_RESPONSE') {
        const pending = this.pendingRequests.get(message.reqId);
        if (pending) {
          this.pendingRequests.delete(message.reqId);
          if (pending.timeoutId) clearTimeout(pending.timeoutId);
          
          if (message.action === 'send') {
            pending.res.status(message.status || 200).send(message.body);
          } else if (message.action === 'redirect') {
            pending.res.redirect(message.url);
          } else if (message.action === 'error' || message.action === 'crash') {
            pending.res.status(500).send('Custom Security Hook terminated with error.');
          } else {
            pending.next();
          }
        }
      }
    });

    this.hooksWorker.on('exit', (code) => {
      console.error('[Amana Hooks Worker] Exited with code', code, '- restarting worker...');
      for (const [reqId, pending] of this.pendingRequests.entries()) {
        if (pending.timeoutId) clearTimeout(pending.timeoutId);
        pending.next();
      }
      this.pendingRequests.clear();
      this.startHooksWorker();
    });
  }

  setupExpress() {
    const app = express();
    const router = express.Router();

    this.startHooksWorker();

    app.set('view engine', 'ejs');
    app.set('views', path.join(__dirname, '../views'));
    const isProduction = process.env.NODE_ENV === 'production';
    if (isProduction) {
      app.set('trust proxy', 1);
    }
    if (isProduction && process.env.AMANA_FORCE_HTTPS !== 'false') {
      app.use((req, res, next) => {
        const forwardedProto = req.headers['x-forwarded-proto'];
        if (!req.secure && forwardedProto !== 'https') {
          return res.redirect(308, `https://${req.headers.host}${req.originalUrl}`);
        }
        next();
      });
    }
    
    app.use(helmet({
      hsts: isProduction ? {
        maxAge: 15552000,
        includeSubDomains: true,
        preload: false
      } : false,
      contentSecurityPolicy: {
        directives: {
          defaultSrc: ["'self'"],
          scriptSrc: ["'self'", "'unsafe-inline'", "cdn.jsdelivr.net", "code.iconify.design"],
          styleSrc: ["'self'", "'unsafe-inline'", "cdn.jsdelivr.net", "fonts.googleapis.com"],
          fontSrc: ["'self'", "fonts.gstatic.com"],
          connectSrc: ["'self'", "cdn.jsdelivr.net", "api.iconify.design"],
          imgSrc: ["'self'", "data:", "cdn.jsdelivr.net", "images.unsplash.com"]
        }
      }
    }));
    app.use(limiter);

    app.use(express.json());
    app.use(express.urlencoded({ extended: true }));
    app.use(inputSanitizer);

    app.use((req, res, next) => {
      const reqId = ++this.reqIdCounter;
      const timeoutId = setTimeout(() => {
        const pending = this.pendingRequests.get(reqId);
        if (pending) {
          this.pendingRequests.delete(reqId);
          console.warn('[Amana Hook Timeout] Custom hook took too long, bypassing...');
          pending.next();
        }
      }, 1000);

      this.pendingRequests.set(reqId, { res, next, timeoutId });

      this.hooksWorker.send({
        type: 'EXECUTE_HOOK',
        reqId,
        req: {
          method: req.method,
          url: req.url,
          headers: req.headers,
          body: req.body,
          query: req.query,
          params: req.params
        }
      });
    });

    const sessionSecret = process.env.SESSION_SECRET || (isProduction ? null : 'dev_only_change_me_session_secret');
    if (!sessionSecret) {
      throw new Error('SESSION_SECRET must be set in production.');
    }
    const weakSessionSecrets = new Set(['dev_only_change_me_session_secret', 'change_me', 'changeme', 'secret', 'password']);
    if (isProduction && (weakSessionSecrets.has(sessionSecret) || sessionSecret.length < 32)) {
      throw new Error('SESSION_SECRET must be at least 32 characters and not use a known default in production.');
    }
    if (!isProduction && weakSessionSecrets.has(sessionSecret)) {
      console.warn('[Amana Security Warning] Development SESSION_SECRET is using the built-in fallback. Set SESSION_SECRET for shared dev environments.');
    }

    app.use(session({
      secret: sessionSecret,
      resave: false,
      saveUninitialized: false,
      cookie: {
        secure: isProduction,
        httpOnly: true,
        sameSite: 'lax'
      }
    }));

    app.use(csrfProtection);

    this.compileApiRoutes(router);
    this.compileRouteTable(router);
    app.use('/', router);

    const PORT = process.env.PORT || 3000;
    app.listen(PORT, () => {
      console.log(`\n[Amana App] Server is running on http://localhost:${PORT}`);
    });
  }

  compileRouteTable(router) {
    const stdLib = {
      time: {
        now: () => new Date().toISOString()
      },
      http: {
        get: async (url) => {
          const res = await fetch(url);
          return res.json();
        },
        post: async (url, body) => {
          const res = await fetch(url, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body)
          });
          return res.json();
        }
      },
      auth: {
        verify: async (hash, password) => {
          return argon2.verify(hash, password);
        },
        hash: async (password) => {
          return argon2.hash(password);
        }
      }
    };

    // Views are precompiled by the Rust generator into ../views/*.ejs.
    // The JS HTML helpers above stay available for compatibility checks only.
    for (const view of []) {
      let ejs_template = generateEjs(view.render_body, view.client_states);
      if (view.client_states && view.client_states.length > 0) {
        const stateFields = view.client_states.map(state => {
          const initialJs = compileExpressionToJs(state.initial_value);
          return `${state.name}: ${initialJs}`;
        });
        const xDataStr = `{ ${stateFields.join(', ')} }`;
        ejs_template = `<div x-data="${xDataStr}">\n${ejs_template}\n</div>`;
      }
      const bodyAttrs = canvasAttributes(view.canvas);
      
      let viewHtml = `<!DOCTYPE html>
<html lang="${htmlLang}" dir="${htmlDir}">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title><%= typeof title !== 'undefined' ? title : 'Amana Application' %></title>
  <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
  <script defer src="https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js"></script>
  <script defer src="https://code.iconify.design/iconify-icon/2.1.0/iconify-icon.min.js"></script>
  <style>
    :root {
      --bg-primary: #ffffff;
      --bg-secondary: #f8f9fa;
      --border-color: #e5e7eb;
      --color-primary: #4f46e5;
      --color-primary-soft: #eef2ff;
      --color-accent: #0891b2;
      --color-success: #16a34a;
      --color-warning: #ca8a04;
      --color-danger: #dc2626;
      --text-primary: #111827;
      --text-secondary: #4b5563;
      --font-body: 400 1rem/1.6 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      --font-heading: 700 1.75rem/1.2 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      --font-mono: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      --transition-smooth: all 0.2s ease-in-out;
      --transition-fast: all 0.12s ease-in-out;

      --space-xs: 0.25rem;
      --space-sm: 0.5rem;
      --space-md: 1rem;
      --space-lg: 1.5rem;
      --space-xl: 2rem;
      --space-2xl: 3rem;
      --space-3xl: 4.5rem;
      --space-4xl: 6rem;
      
      --radius-sm:  10px;
      --radius-md:  16px;
      --radius-lg:  22px;
      --radius-xl:  28px;
      --radius-2xl: 36px;
      --radius-small:  var(--radius-sm);
      --radius-medium: var(--radius-md);
      --radius-large:  var(--radius-lg);
      --radius-soft:   var(--radius-md);
      
      --padding-large: 1.5rem;
      --padding-medium: 1rem;
      --padding-small: 0.5rem;
      
      --text-xs: 0.75rem;
      --text-sm: 0.875rem;
      --text-md: 1rem;
      --text-lg: 1.125rem;
      --text-xl: 1.35rem;
      --text-2xl: 1.75rem;
      --text-3xl: 2.4rem;

      --content-width: 1120px;
      --wide-width: 1360px;
      --readable-width: 72ch;

      --shadow-smooth: 0 4px 6px -1px rgba(0,0,0,0.1), 0 2px 4px -1px rgba(0,0,0,0.06);
      --shadow-large: 0 20px 35px -15px rgba(15,23,42,0.28);
      --shadow-soft: 0 10px 24px -18px rgba(15,23,42,0.35);
      --shadow-floating: 0 28px 70px -30px rgba(15,23,42,0.48), 0 12px 28px -24px rgba(15,23,42,0.32);
      --shadow-strong: 0 40px 95px -38px rgba(2,6,23,0.72);
      --elevation-1: 0 1px 2px rgba(15,23,42,0.08);
      --elevation-2: 0 8px 18px -14px rgba(15,23,42,0.35);
      --elevation-3: 0 18px 36px -22px rgba(15,23,42,0.45);
      --elevation-4: 0 28px 55px -30px rgba(15,23,42,0.55);
      --elevation-5: 0 35px 80px -35px rgba(2,6,23,0.68);
      --glow-primary: 0 0 0 4px var(--color-primary-soft), 0 18px 40px -24px var(--color-primary);
      --glow-accent: 0 0 0 4px rgba(8,145,178,0.15), 0 18px 40px -24px var(--color-accent);
      --surface-base: #ffffff;
      --surface-muted: #f8fafc;
      --surface-elevated: #ffffff;
      --border-subtle: rgba(15,23,42,0.10);
      --gradient-primary: linear-gradient(135deg, var(--color-primary), var(--color-accent));
      --gradient-accent: linear-gradient(135deg, var(--color-accent), var(--color-primary));
      --gradient-hero: radial-gradient(circle at 12% 8%, rgba(34,211,238,0.22), transparent 32%), radial-gradient(circle at 90% 10%, rgba(79,70,229,0.22), transparent 36%), linear-gradient(135deg, #ffffff, var(--color-primary-soft));
      --gradient-mesh: radial-gradient(circle at 10% 20%, rgba(34,211,238,0.22), transparent 34%), radial-gradient(circle at 80% 0%, rgba(79,70,229,0.24), transparent 38%), radial-gradient(circle at 70% 90%, rgba(16,185,129,0.16), transparent 35%), var(--surface-base);
      --gradient-aurora: radial-gradient(circle at 15% 20%, rgba(34,211,238,0.32), transparent 30%), radial-gradient(circle at 80% 20%, rgba(168,85,247,0.24), transparent 35%), radial-gradient(circle at 50% 100%, rgba(16,185,129,0.18), transparent 40%), var(--bg-secondary);
      --gradient-spotlight: radial-gradient(circle at 50% 0%, var(--color-primary-soft), transparent 48%), var(--surface-base);
      
      --glass-bg: rgba(255, 255, 255, 0.45);
      --glass-blur: blur(12px);
      --glass-border: rgba(255, 255, 255, 0.25);
    }
    @media (prefers-color-scheme: dark) {
      :root {
        --bg-primary: #111827;
        --bg-secondary: #050816;
        --border-color: rgba(148,163,184,0.22);
        --color-primary: #6366f1;
        --color-primary-soft: #312e81;
        --color-accent: #22d3ee;
        --color-success: #22c55e;
        --color-warning: #facc15;
        --color-danger: #f87171;
        --text-primary: #f9fafb;
        --text-secondary: #cbd5e1;
        
        --shadow-smooth: 0 10px 30px -22px rgba(0,0,0,0.75);
        --shadow-large: 0 32px 70px -30px rgba(0,0,0,0.78);
        --shadow-soft: 0 16px 40px -30px rgba(0,0,0,0.82);
        --shadow-floating: 0 32px 85px -36px rgba(0,0,0,0.86), 0 18px 34px -28px rgba(15,23,42,0.8);
        --shadow-strong: 0 45px 110px -42px rgba(0,0,0,0.9);
        --surface-base: #0b1020;
        --surface-muted: #111827;
        --surface-elevated: #151d31;
        --border-subtle: rgba(148,163,184,0.18);
        --gradient-hero: radial-gradient(circle at 12% 8%, rgba(34,211,238,0.18), transparent 32%), radial-gradient(circle at 88% 12%, rgba(99,102,241,0.28), transparent 38%), linear-gradient(135deg, #080b16, #111827);
        --gradient-mesh: radial-gradient(circle at 10% 20%, rgba(34,211,238,0.20), transparent 34%), radial-gradient(circle at 80% 0%, rgba(99,102,241,0.26), transparent 38%), radial-gradient(circle at 70% 90%, rgba(16,185,129,0.10), transparent 35%), #0b1020;
        --gradient-aurora: radial-gradient(circle at 15% 20%, rgba(34,211,238,0.28), transparent 30%), radial-gradient(circle at 80% 20%, rgba(168,85,247,0.23), transparent 35%), radial-gradient(circle at 50% 100%, rgba(16,185,129,0.14), transparent 40%), #050816;
        --gradient-spotlight: radial-gradient(circle at 50% 0%, rgba(99,102,241,0.24), transparent 48%), #0b1020;
        
        --glass-bg: rgba(15, 23, 42, 0.64);
        --glass-blur: blur(18px);
        --glass-border: rgba(255, 255, 255, 0.12);
      }
    }
    ${themeCss(this.ir.theme)}
    *, *::before, *::after { box-sizing: border-box; }
    html { width: 100%; max-width: 100%; overflow-x: hidden; scroll-behavior: smooth; }
    body { width: 100%; max-width: 100%; min-width: 0; margin: 0; overflow-x: hidden; background-color: var(--bg-secondary); color: var(--text-primary); font: var(--font-body); text-rendering: geometricPrecision; }
    body.amana-page { display: block; padding: 0 !important; gap: normal !important; }
    :where(main, section, article, aside, header, footer, nav, div, form) { min-width: 0; }
    :where(h1, h2, h3, h4, h5, h6, p, span, strong, a, button, label, input, textarea, pre) { max-width: 100%; overflow-wrap: anywhere; word-break: normal; letter-spacing: 0; }
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
    .amana-section h2, .amana-section-head h2 { margin: 0; font-size: clamp(2rem, 5vw, 4.2rem); line-height: 1.05; letter-spacing: 0; font-weight: 900; }
    .amana-section-copy { color: var(--text-secondary); max-width: 68ch; font-size: clamp(1rem, 2vw, 1.2rem); line-height: 1.8; }
    .amana-grid { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, var(--dg-columns, repeat(auto-fit, minmax(var(--grid-min, 16rem), 1fr))))); gap: var(--component-gap, var(--custom-gap, var(--dg-gap, var(--space-lg)))); }
    .amana-grid > *, .amana-split > *, .dg-layout-split-diagonal > *, .dg-layout-asymmetric > *, .dg-layout-editorial > *, .dg-layout-dashboard-shell > *, .dg-layout-command-center > *, .dg-layout-showcase-rail > * { min-width: 0; }
    .amana-stack { display: flex; flex-direction: column; gap: var(--space-md); }
    .amana-stack-gap-xs { gap: var(--space-xs); }
    .amana-stack-gap-sm { gap: var(--space-sm); }
    .amana-stack-gap-lg { gap: var(--space-lg); }
    .amana-stack-gap-xl { gap: var(--space-xl); }
    .amana-navbar { width: var(--component-width, min(100% - 2rem, var(--wide-width))); max-width: var(--component-max-width, none); margin-inline: auto; display: flex; align-items: center; justify-content: space-between; gap: var(--component-gap, var(--space-lg)); padding: var(--component-padding, 0.85rem 0); min-height: var(--component-min-height, 4.25rem); }
    .amana-brand { display: inline-flex; align-items: center; gap: 0.65rem; color: var(--text-primary); font-weight: 900; text-decoration: none; letter-spacing: 0; }
    .amana-brand::before { content: ""; width: 0.72rem; height: 0.72rem; border-radius: 999px; background: var(--gradient-primary); box-shadow: var(--glow-accent); }
    .amana-navlinks { display: flex; align-items: center; justify-content: flex-end; gap: var(--space-sm); flex-wrap: wrap; }
    .amana-hero { position: relative; isolation: isolate; display: grid; grid-template-columns: var(--component-columns, var(--dg-template, var(--dg-columns, minmax(0, 1fr)))); gap: var(--component-gap, var(--custom-gap, var(--dg-gap, clamp(1.5rem, 4vw, 3.5rem)))); align-items: center; width: var(--component-width, auto); max-width: var(--component-max-width, none); min-width: var(--component-min-width, 0); min-height: var(--component-min-height, auto); height: var(--component-height, auto); padding: var(--component-padding, clamp(1.5rem, 4vw, 4rem)); background: var(--custom-bg, var(--custom-gradient, var(--gradient-hero))); border: 1px solid var(--custom-border, var(--border-subtle)); border-radius: var(--custom-radius, var(--radius-2xl)); overflow: hidden; box-shadow: var(--custom-shadow, var(--shadow-floating)); opacity: var(--component-opacity, 1); transform: var(--component-transform, none); transition: var(--component-transition, transform 180ms ease, box-shadow 180ms ease, border-color 180ms ease); }
    .amana-hero::before { content: ""; position: absolute; inset: -20%; background: radial-gradient(circle at 15% 20%, rgba(34,211,238,0.18), transparent 28%), radial-gradient(circle at 85% 20%, rgba(99,102,241,0.22), transparent 32%); z-index: -1; }
    .amana-hero-content { display: grid; gap: var(--component-gap, var(--space-md)); max-width: var(--component-copy-width, 780px); min-width: 0; }
    .amana-hero h1 { margin: 0; font-size: var(--component-title-size, clamp(2.25rem, 5.4vw, 5.4rem)); line-height: var(--component-title-leading, 1.02); max-width: var(--component-title-width, min(100%, 16ch)); letter-spacing: 0; font-weight: var(--dg-type-weight, 900); }
    .amana-hero-copy { margin: 0; max-width: var(--component-copy-width, 66ch); color: var(--custom-muted, var(--text-secondary)); font-size: var(--component-copy-size, clamp(1rem, 1.8vw, 1.2rem)); line-height: 1.8; }
    .amana-hero-actions { display: flex; gap: var(--component-gap, var(--space-md)); flex-wrap: wrap; margin-top: var(--space-md); align-items: center; }
    .amana-hero-proof { color: var(--text-secondary); font-weight: 800; }
    .amana-hero-media { min-height: clamp(16rem, 34vw, 28rem); border-radius: var(--radius-2xl); background-size: cover; background-position: center; border: 1px solid var(--border-subtle); box-shadow: var(--shadow-floating); }
    .amana-eyebrow { color: var(--color-accent); font-weight: 900; text-transform: uppercase; letter-spacing: 0.1em; font-size: var(--text-sm); }
    .amana-card { position: relative; display: grid; gap: var(--component-gap, var(--custom-gap, var(--space-md))); width: var(--component-width, auto); max-width: var(--component-max-width, none); min-width: var(--component-min-width, 0); min-height: var(--component-min-height, 100%); height: var(--component-height, auto); background: var(--custom-bg, var(--custom-gradient, linear-gradient(180deg, color-mix(in srgb, var(--surface-elevated) 92%, transparent), color-mix(in srgb, var(--surface-muted) 82%, transparent)))); border: 1px solid var(--custom-border, var(--border-subtle)); border-radius: var(--custom-radius, var(--radius-2xl)); padding: var(--component-padding, var(--custom-padding, clamp(1.1rem, 2.6vw, 1.8rem))); box-shadow: var(--custom-shadow, var(--shadow-soft)); opacity: var(--component-opacity, 1); transform: var(--component-transform, none); overflow: hidden; transition: var(--component-transition, transform 180ms ease, box-shadow 180ms ease, border-color 180ms ease); }
    .amana-card::before { content: ""; position: absolute; inset: 0; pointer-events: none; background: linear-gradient(135deg, rgba(255,255,255,0.08), transparent 38%); opacity: 0.75; }
    .amana-card:hover { transform: translateY(-4px); box-shadow: var(--shadow-floating); border-color: color-mix(in srgb, var(--color-accent) 32%, var(--border-subtle)); }
    .amana-card > * { position: relative; }
    .amana-card h3 { margin: 0; font-size: clamp(1.25rem, 2vw, 1.75rem); line-height: 1.15; font-weight: 900; }
    .amana-feature-card { min-height: 13rem; }
    .amana-pricing-card { display: flex; flex-direction: column; gap: var(--space-md); }
    .amana-price { font-size: clamp(2rem, 5vw, 3.5rem); line-height: 1; font-weight: 950; }
    .amana-muted { color: var(--text-secondary); line-height: 1.75; }
    .amana-btn { position: relative; display: inline-flex; align-items: center; justify-content: center; gap: var(--component-gap, 0.65rem); width: var(--component-width, auto); max-width: var(--component-max-width, 100%); min-width: var(--component-min-width, 0); min-height: var(--component-min-height, 3rem); height: var(--component-height, auto); padding: var(--component-padding, 0.78rem 1.12rem); border-radius: var(--custom-radius, 999px); font-weight: 900; text-decoration: none; border: 1px solid var(--custom-border, transparent); transition: var(--component-transition, transform 160ms ease, box-shadow 160ms ease, border-color 160ms ease, background 160ms ease); white-space: nowrap; line-height: 1.15; overflow: hidden; opacity: var(--component-opacity, 1); transform: var(--component-transform, none); }
    .amana-btn:hover { transform: translateY(-2px); }
    .amana-btn:active { transform: translateY(0) scale(0.99); }
    .amana-btn:focus-visible { outline: 3px solid color-mix(in srgb, var(--color-accent) 60%, transparent); outline-offset: 3px; }
    .amana-btn-primary { background: var(--gradient-primary); color: white; box-shadow: var(--glow-primary); }
    .amana-btn-primary:hover { box-shadow: var(--shadow-floating), var(--glow-primary); }
    .amana-btn-secondary { color: var(--text-primary); background: color-mix(in srgb, var(--surface-elevated) 82%, transparent); border-color: var(--border-subtle); box-shadow: var(--shadow-soft); }
    .amana-btn-ghost { color: var(--text-primary); background: transparent; border-color: var(--border-subtle); }
    .amana-btn-sm { min-height: 2.35rem; padding: 0.52rem 0.82rem; font-size: var(--text-sm); }
    .amana-btn-lg { min-height: 3.45rem; padding: 0.96rem 1.35rem; font-size: var(--text-lg); }
    .amana-icon, .amana-btn-icon, iconify-icon { display: inline-grid; place-items: center; width: 1.25em; min-width: 1.25em; height: 1.25em; line-height: 1; vertical-align: -0.18em; transition: transform 160ms ease; }
    .amana-btn:hover .amana-btn-icon { transform: translateX(-2px); }
    .amana-btn-intent-danger { background: var(--color-danger); color: white; }
    .amana-btn-intent-success { background: var(--color-success); color: white; }
    .amana-field { display: flex; flex-direction: column; gap: 0.45rem; margin-bottom: var(--space-md); }
    .amana-field span { color: var(--text-primary); font-weight: 800; }
    .amana-field input, .amana-form-control { width: 100%; border: 1px solid var(--border-subtle); border-radius: var(--radius-soft); min-height: 3rem; padding: 0.78rem 0.92rem; background: color-mix(in srgb, var(--surface-base) 84%, transparent); color: var(--text-primary); box-shadow: inset 0 1px 0 rgba(255,255,255,0.04); }
    .amana-field input:focus, .amana-form-control:focus { outline: 3px solid color-mix(in srgb, var(--color-accent) 34%, transparent); border-color: var(--color-accent); }
    .amana-form-card { background: color-mix(in srgb, var(--surface-elevated) 88%, transparent); border: 1px solid var(--border-subtle); border-radius: var(--radius-2xl); padding: clamp(1.25rem, 3vw, 2rem); box-shadow: var(--shadow-floating); }
    .amana-help { color: var(--text-secondary); font-size: var(--text-sm); }
    .amana-alert { border-radius: var(--radius-soft); border: 1px solid var(--border-subtle); padding: var(--space-md); background: var(--surface-muted); }
    .amana-alert-success { border-color: rgba(22,163,74,0.35); }
    .amana-alert-danger { border-color: rgba(220,38,38,0.35); }
    .amana-footer { width: min(100% - 2rem, var(--wide-width)); margin: var(--space-3xl) auto 0; padding-block: var(--space-xl); color: var(--text-secondary); border-top: 1px solid var(--border-subtle); }
    .amana-modal { position: fixed; inset: 0; display: grid; place-items: center; background: rgba(2,6,23,0.55); padding: var(--space-lg); backdrop-filter: blur(10px); }
    .amana-modal-panel { width: min(100%, 36rem); background: var(--surface-elevated); border-radius: var(--radius-2xl); padding: var(--space-lg); box-shadow: var(--shadow-strong); }
    .amana-tabs { display: flex; gap: var(--space-sm); flex-wrap: wrap; border-bottom: 1px solid var(--border-subtle); padding-bottom: var(--space-sm); }
    .amana-card-top { display: flex; align-items: center; justify-content: space-between; gap: var(--space-sm); margin-bottom: var(--space-xs); }
    .amana-card-meta { color: var(--text-secondary); font-size: var(--text-sm); }
    .amana-card-action { color: var(--color-accent); font-weight: 900; text-decoration: none; margin-top: auto; }
    .amana-card-density-compact { padding: var(--space-md); }
    .amana-card-density-spacious { padding: var(--space-xl); }
    .amana-badge { display: inline-flex; align-items: center; width: fit-content; gap: 0.35rem; border: 1px solid var(--border-subtle); border-radius: 999px; padding: 0.38rem 0.78rem; font-size: var(--text-sm); font-weight: 900; background: color-mix(in srgb, var(--surface-muted) 78%, transparent); color: var(--text-primary); box-shadow: var(--shadow-soft); }
    .amana-badge-success { border-color: rgba(22,163,74,0.35); color: var(--color-success); }
    .amana-badge-warning { border-color: rgba(202,138,4,0.35); color: var(--color-warning); }
    .amana-badge-danger { border-color: rgba(220,38,38,0.35); color: var(--color-danger); }
    .amana-kpi { display: grid; gap: 0.35rem; padding: clamp(1.25rem, 3vw, 2rem); border: 1px solid var(--border-subtle); border-radius: var(--radius-2xl); background: linear-gradient(180deg, var(--surface-elevated), var(--surface-muted)); box-shadow: var(--shadow-soft); }
    .amana-kpi-label { color: var(--text-secondary); font-size: var(--text-sm); }
    .amana-kpi-value { font-size: clamp(2rem, 5vw, 4rem); line-height: 1; }
    .amana-kpi-trend { color: var(--color-success); font-weight: 700; }
    .amana-logo-cloud { display: grid; gap: var(--space-md); padding-block: var(--space-lg); }
    .amana-logo-row { display: flex; flex-wrap: wrap; gap: var(--space-md); align-items: center; color: var(--text-secondary); }
    .amana-testimonial { margin: 0; display: grid; gap: var(--space-md); border: 1px solid var(--border-subtle); border-radius: var(--radius-xl); padding: var(--space-lg); background: var(--surface-elevated); box-shadow: var(--shadow-soft); }
    .amana-testimonial blockquote { margin: 0; font-size: var(--text-lg); color: var(--text-primary); }
    .amana-testimonial figcaption { display: grid; gap: 0.1rem; color: var(--text-secondary); }
    .amana-timeline { display: grid; gap: var(--space-md); list-style: none; padding: 0; margin: 0; }
    .amana-timeline-item { position: relative; padding: var(--space-lg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xl); background: var(--surface-elevated); }
    .amana-empty-state { display: grid; place-items: center; text-align: center; gap: var(--space-md); min-height: 18rem; border: 1px dashed var(--border-subtle); border-radius: var(--radius-xl); padding: var(--space-xl); background: var(--surface-muted); }
    .amana-split { display: grid; grid-template-columns: minmax(0, 1fr) minmax(16rem, 0.85fr); gap: var(--space-xl); align-items: center; }
    .amana-cluster { display: flex; flex-wrap: wrap; gap: var(--space-md); align-items: center; }
    .amana-sidebar { border: 1px solid var(--border-subtle); border-radius: var(--radius-xl); background: var(--surface-elevated); padding: var(--space-lg); box-shadow: var(--shadow-soft); }
    .amana-navbar-sticky { position: sticky; top: 0; z-index: 20; background: color-mix(in srgb, var(--surface-base) 78%, transparent); backdrop-filter: blur(12px); border-bottom: 1px solid var(--border-subtle); }
    .amana-page { min-height: 100vh; background: var(--bg-secondary); color: var(--text-primary); overflow-x: hidden; }
    .amana-runtime-shell { display: block; width: 100%; max-width: 100%; min-height: 100vh; margin: 0; padding: 0; overflow-x: hidden; }
    .amana-runtime-shell > :not(script):not(style) { max-width: 100%; }
    .dg-canvas-width-full .amana-container { width: 100%; max-width: none; }
    .dg-canvas-width-wide .amana-container { width: min(100% - 2rem, var(--wide-width)); }
    .dg-canvas-width-readable .amana-container { width: min(100% - 2rem, var(--readable-width)); }
    .dg-layout-split-diagonal,
    .dg-layout-asymmetric { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, minmax(0, 1.1fr) minmax(16rem, 0.85fr))); align-items: center; gap: var(--component-gap, var(--dg-gap, clamp(1.5rem, 5vw, 4rem))); }
    :where(.dg-layout-split-diagonal, .dg-layout-asymmetric, .dg-layout-editorial, .dg-layout-dashboard-shell, .dg-layout-magazine, .dg-layout-bento, .dg-layout-command-center, .dg-layout-showcase-rail) > .amana-container { grid-column: 1 / -1; width: min(100% - 2rem, var(--content-width)); }
    .dg-layout-centered { text-align: center; justify-items: center; }
    .dg-layout-editorial { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, minmax(14rem, 0.55fr) minmax(0, 1fr))); gap: var(--component-gap, var(--dg-gap, clamp(2rem, 6vw, 5rem))); align-items: start; }
    .dg-layout-dashboard-shell,
    :where(.dg-layout-dashboard-shell) .amana-runtime-shell > :not(script):not(style) { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, minmax(14rem, 18rem) minmax(0, 1fr))); gap: var(--component-gap, var(--dg-gap, var(--space-lg))); align-items: start; }
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
      .amana-navbar { align-items: flex-start; flex-direction: column; }
      .amana-hero { padding-inline: var(--space-lg); }
      .amana-hero h1 { font-size: 2.6rem; }
      .amana-split,
      .dg-flow-dashboard,
      .dg-layout-split-diagonal,
      .dg-layout-asymmetric,
      .dg-layout-editorial,
      .dg-layout-dashboard-shell,
      :where(.dg-layout-dashboard-shell) .amana-runtime-shell > :not(script):not(style),
      .dg-layout-magazine,
      .dg-layout-bento,
      .dg-layout-command-center,
      .dg-layout-showcase-rail { grid-template-columns: 1fr; }
      .dg-layout-magazine > *,
      .dg-layout-bento > *,
      .dg-layout-bento > *:first-child { grid-column: auto; grid-row: auto; }
    }
    @media (prefers-reduced-motion: reduce) {
      .dg-reduce-motion-auto *,
      .dg-reduce-motion-strict *,
      .dg-motion-stagger-up > *,
      .dg-motion-fade,
      .dg-reveal-blur,
      .dg-reveal-clip { animation: none !important; transition: none !important; }
    }
     :where(.amana-runtime-shell, .amana-page, .page) { width: 100%; max-width: 100%; overflow-x: hidden; }
    :where(.amana-runtime-shell, .amana-page, .page) :where(section, header, main, footer, div, article, aside, form) { min-width: 0; }
    :where(.amana-runtime-shell, .amana-page, .page) :where(h1, h2, h3, p, a, button, span, strong, label, input, textarea, pre) { max-width: 100%; overflow-wrap: anywhere; }
    :where(.dg-layout-split-diagonal, .dg-layout-asymmetric, .dg-layout-editorial, .dg-layout-dashboard-shell, .dg-layout-magazine, .dg-layout-bento, .dg-layout-command-center, .dg-layout-showcase-rail) > .amana-container { grid-column: 1 / -1; }
    @media (max-width: 1200px) {
      :where(.hero-title, .section-title, .auth-card h1, .cta-box h2, .amana-hero h1, h1) { font-size: clamp(2.15rem, 8vw, 4.4rem) !important; line-height: 1.08 !important; max-width: 100% !important; }
      :where(.hero-lead, .section-lead, .amana-hero-copy, p) { font-size: clamp(1rem, 2.4vw, 1.2rem); }
      :where(.hero-shell, .section-space, .workflow, .pricing-section, .testimonials, .cta-section, .auth-shell) { padding-inline: clamp(1rem, 4vw, 2rem) !important; }
      :where(.hero-grid, .split, .workflow-grid, .pricing-grid, .testimonial-grid, .cta-box, .amana-split, .dg-layout-split-diagonal, .dg-layout-asymmetric, .dg-layout-editorial, .dg-layout-dashboard-shell, .dg-layout-command-center, .dg-layout-showcase-rail):not([style*="--bp-laptop-columns"]):not([style*="--bp-tablet-columns"]):not([style*="--bp-mobile-columns"]) { grid-template-columns: minmax(0, 1fr) !important; }
    }
    @media (max-width: 720px) {
      :where(.hero-title, .section-title, .auth-card h1, .cta-box h2, .amana-hero h1, h1) { font-size: clamp(2rem, 11vw, 3.4rem) !important; }
      :where(.hero-panel, .workflow-box, .visual-card, .price-card, .testimonial-card, .cta-box, .auth-card, .contact-card, .amana-card) { padding: clamp(1rem, 5vw, 1.5rem) !important; border-radius: min(var(--radius-2xl), 22px) !important; }
      :where(.hero-actions, .trust-strip, .badge-row, .token-list, .logo-row, .amana-hero-actions, .amana-cluster) { align-items: stretch; }
      :where(.amana-btn, .plan-button, button) { white-space: normal; text-align: center; }
    }
  </style>
  <%- typeof styles !== 'undefined' && styles ? '<style>' + styles + '</style>' : '' %>
</head>
<body${bodyAttrs}>
  <main class="amana-runtime-shell">
  ${ejs_template}
  </main>
</body>
</html>`;
      void viewHtml;
    }

    const hasDslLoginRoute = this.ir.routes.some(r => r.path === '/login');
    if (!hasDslLoginRoute) {
      router.get('/login', (req, res) => {
        res.render('login', { csrfToken: req.session.csrfToken, error: null });
      });

      router.post('/login', authLimiter, async (req, res) => {
        const { email, password } = req.body;
        try {
          const user = await this.dbGet(`SELECT * FROM ${quoteSqlIdentifier('user')} WHERE "email" = ?`, [email]);
          if (user && await argon2.verify(user.password, password)) {
            req.session.user = user;
            return res.redirect('/dashboard');
          }
          res.render('login', { csrfToken: req.session.csrfToken, error: "بيانات الاعتماد غير صحيحة" });
        } catch (err) {
          res.status(500).send("Login failed.");
        }
      });
    }

    router.get('/logout', (req, res) => {
      req.session.destroy();
      res.redirect('/login');
    });

    for (const r of this.ir.routes) {
      router.get(expressRoutePath(r.path), async (req, res) => {
        const currentUser = req.session.user;
        if (r.guard) {
          if (!currentUser) {
            return res.redirect(r.guard.unauth_path);
          }
          const allowed = Boolean(evalAmanaExpression(r.guard.cond_expr, req, currentUser));
          if (!allowed) {
            return res.redirect(r.guard.deny_path);
          }
        }

        try {
          const viewIr = this.ir.views.find(v => v.name.toLowerCase() === r.view_name.toLowerCase());
          const styles = viewIr ? viewIr.styles || '' : '';
          const renderVars = {
            csrfToken: req.session.csrfToken,
            currentUser,
            params: req.params || {},
            query: req.query || {},
            body: req.body || {},
            styles
          };

          for (const fetch of r.fetches) {
            // Runtime Capability Enforcement Check
            if (fetch.model_name === 'time' && !this.ir.app.capabilities.includes('time')) {
              throw new Error("Security Policy Violation: Access to standard library 'time' requires 'time' capability.");
            }
            if (fetch.model_name === 'http' && !this.ir.app.capabilities.includes('network.outbound')) {
              throw new Error("Security Policy Violation: Access to standard library 'http' requires 'network.outbound' capability.");
            }
            if (fetch.model_name === 'auth' && !this.ir.app.capabilities.includes('auth')) {
              throw new Error("Security Policy Violation: Access to standard library 'auth' requires 'auth' capability.");
            }

            if (fetch.model_name === 'time' || fetch.model_name === 'http' || fetch.model_name === 'auth') {
              const evaluatedArgs = [];
              for (const argExpr of fetch.query_args) {
                evaluatedArgs.push(evalAmanaExpression(argExpr[1], req, currentUser, renderVars));
              }
              
              let fetchVal;
              if (fetch.model_name === 'time' && fetch.query_method === 'now') {
                fetchVal = stdLib.time.now();
              } else if (fetch.model_name === 'http') {
                if (fetch.query_method === 'get') {
                  fetchVal = await stdLib.http.get(evaluatedArgs[0]);
                } else if (fetch.query_method === 'post') {
                  fetchVal = await stdLib.http.post(evaluatedArgs[0], evaluatedArgs[1]);
                }
              } else if (fetch.model_name === 'auth') {
                if (fetch.query_method === 'verify') {
                  fetchVal = await stdLib.auth.verify(evaluatedArgs[0], evaluatedArgs[1]);
                } else if (fetch.query_method === 'hash') {
                  fetchVal = await stdLib.auth.hash(evaluatedArgs[0]);
                }
              }
              renderVars[fetch.var_name] = fetchVal;
            } else {
              const { sql, paramsJs } = generateSafeQuery(this.ir.models, fetch.model_name, fetch.query_method, fetch.query_args);
              const queryParams = [];
              for (const paramExprJs of paramsJs) {
                queryParams.push(evalAmanaExpression(paramExprJs, req, currentUser, renderVars));
              }

              let result;
              if (fetch.query_method === 'find') {
                result = await this.dbGet(sql, queryParams);
              } else if (fetch.query_method === 'count') {
                const row = await this.dbGet(sql, queryParams);
                result = row ? row.count : 0;
              } else {
                result = await this.dbAll(sql, queryParams);
              }
              renderVars[fetch.var_name] = result;
            }
          }

          res.render(r.view_name.toLowerCase(), renderVars);
        } catch (err) {
          return routeErrorResponse(req, res, err);
          res.status(500).send("خطأ في تشغيل خادم العرض");
        }
      });

      for (const form of r.form_actions) {
        const formPath = `/form-submit/${form.model_name.toLowerCase()}/${form.action}`;
        router.post(formPath, authLimiter, async (req, res) => {
          try {
            const currentUser = req.session.user;
            const modelLowercase = form.model_name.toLowerCase();
            const action = form.action.toLowerCase();

            if (r.guard) {
              if (!currentUser) {
                return res.redirect(r.guard.unauth_path);
              }
              const allowed = Boolean(evalAmanaExpression(r.guard.cond_expr, req, currentUser));
              if (!allowed) {
                return res.redirect(r.guard.deny_path);
              }
            }

            const model = this.ir.models.find(m => m.table_name === modelLowercase);
            if (!model) {
              throw new Error(`Form action references unknown model '${form.model_name}'.`);
            }
            const modelTableSql = quoteSqlIdentifier(modelLowercase);
            const validColumns = new Set(model.fields.map(f => f.name.toLowerCase()).concat(['id']));
            for (const f of form.fields) {
              if (!validColumns.has(f.toLowerCase())) {
                throw new Error(`Rejected form field '${f}' for model '${form.model_name}'.`);
              }
            }
            const resolvedDefaults = {};
            for (const binding of form.defaults || []) {
              const field = String(binding[0]).toLowerCase();
              const expr = binding[1];
              if (!validColumns.has(field)) {
                throw new Error(`Rejected default field '${field}' for model '${form.model_name}'.`);
              }
              const value = evalAmanaExpression(expr, req, currentUser);
              if (value === undefined) {
                throw new Error(`Default field '${field}' evaluated to undefined.`);
              }
              resolvedDefaults[field] = value;
            }
            const resolvedConstraints = [];
            for (const binding of form.constraints || []) {
              const field = String(binding[0]).toLowerCase();
              const expr = binding[1];
              if (!validColumns.has(field)) {
                throw new Error(`Rejected where field '${field}' for model '${form.model_name}'.`);
              }
              const value = evalAmanaExpression(expr, req, currentUser);
              if (value === undefined) {
                throw new Error(`Where field '${field}' evaluated to undefined.`);
              }
              resolvedConstraints.push({ field, value });
            }
            const readFormValue = (field) => {
              const key = field.toLowerCase();
              if (Object.prototype.hasOwnProperty.call(resolvedDefaults, key)) {
                return resolvedDefaults[key];
              }
              return req.body[field] ?? req.body[key];
            };
            const validateFieldValue = (fieldName, value, options = {}) => {
              validateRuntimeFieldValue(model, fieldName, value, options);
            };

            if (action === 'login') {
              const user = await this.dbGet(`SELECT * FROM ${modelTableSql} WHERE "email" = ? LIMIT 1`, [req.body.email]);
              if (user && user.password && await argon2.verify(user.password, req.body.password || '')) {
                req.session.user = user;
                return res.redirect(form.redirect_success || '/');
              }
              return res.status(401).send('Invalid email or password.');
            }

            if (action === 'logout') {
              req.session.destroy(() => res.redirect(form.redirect_success || '/login'));
              return;
            }

            if (action === 'register') {
              const existing = await this.dbGet(`SELECT "id" FROM ${modelTableSql} WHERE "email" = ? LIMIT 1`, [req.body.email]);
              if (existing) {
                return res.status(409).send('Email already exists.');
              }
            }

            let querySql = '';
            let queryParams = [];

            if (action === 'update') {
              let setClauses = [];
              const updateFields = Array.from(new Set(
                form.fields
                  .map(f => f.toLowerCase())
                  .concat(Object.keys(resolvedDefaults))
              ));
              for (const f of updateFields) {
                if (f !== 'id') {
                  const value = readFormValue(f);
                  validateFieldValue(f, value, { partial: true });
                  if (f.includes('password') && !value) {
                    continue;
                  }
                  setClauses.push(`${quoteSqlIdentifier(f)} = ?`);
                  if (f.includes('password')) {
                    queryParams.push(await argon2.hash(value));
                  } else {
                    queryParams.push(value);
                  }
                }
              }
              if (setClauses.length === 0) {
                return res.redirect(form.redirect_success);
              }
              queryParams.push(req.body.id);
              const whereClauses = ['"id" = ?'];
              for (const constraint of resolvedConstraints) {
                whereClauses.push(`${quoteSqlIdentifier(constraint.field)} = ?`);
                queryParams.push(constraint.value);
              }
              querySql = `UPDATE ${modelTableSql} SET ${setClauses.join(', ')} WHERE ${whereClauses.join(' AND ')}`;
            } else if (action === 'delete') {
              queryParams.push(req.body.id);
              const whereClauses = ['"id" = ?'];
              for (const constraint of resolvedConstraints) {
                whereClauses.push(`${quoteSqlIdentifier(constraint.field)} = ?`);
                queryParams.push(constraint.value);
              }
              querySql = `DELETE FROM ${modelTableSql} WHERE ${whereClauses.join(' AND ')}`;
            } else if (action === 'create' || action === 'register') {
              let fieldsBinding = [];
              let placeholders = [];
              const insertFields = Array.from(new Set(
                form.fields
                  .map(f => f.toLowerCase())
                  .concat(Object.keys(resolvedDefaults))
              ));
              for (const f of insertFields) {
                fieldsBinding.push(f);
                placeholders.push('?');
                const value = readFormValue(f);
                validateFieldValue(f, value);
                if (f.includes('password')) {
                  queryParams.push(await argon2.hash(value || ''));
                } else {
                  queryParams.push(value);
                }
              }
              querySql = `INSERT INTO ${modelTableSql} (${fieldsBinding.map(quoteSqlIdentifier).join(', ')}) VALUES (${placeholders.join(', ')})`;
            } else {
              throw new Error(`Unsupported form action '${action}'.`);
            }

            const writeResult = await this.dbRun(querySql, queryParams);
            if ((action === 'update' || action === 'delete') && writeResult.changes === 0) {
              return res.status(resolvedConstraints.length > 0 ? 403 : 404).send('Record not found or action is not authorized.');
            }
            res.redirect(form.redirect_success);
          } catch (err) {
            console.error('[Amana Form Action Error]', err);
            return res.status(String(err.message || '').startsWith('Field ') ? 400 : 500).send(err.message || 'Form submission failed.');
          }
        });
      }
    }
  }
}

module.exports = AmanaEngine;
"#;
    // Let's compile and write the EJS templates at compile-time in Rust!
    let html_dir = theme_direction(ir.theme.as_ref());
    let html_lang = theme_language(ir.theme.as_ref());
    let bootstrap_css = if html_dir == "rtl" { "bootstrap.rtl.min.css" } else { "bootstrap.min.css" };
    let theme_css_block = theme_css(ir.theme.as_ref());

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

    // Default login page if not defined in DSL
    let has_dsl_login_route = ir.routes.iter().any(|r| r.path == "/login");
    if !has_dsl_login_route {
        let default_login_html = compile_default_login_ejs(html_lang, html_dir, bootstrap_css);
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

// --- Compile-Time View Compilation & Theme CSS Utilities in Rust ---

fn theme_direction(theme: Option<&ThemeIR>) -> &str {
    if let Some(t) = theme {
        for (key, val) in &t.settings {
            if key == "direction" {
                return val;
            }
        }
    }
    "ltr"
}

fn theme_language(theme: Option<&ThemeIR>) -> &str {
    if let Some(t) = theme {
        for (key, val) in &t.settings {
            if key == "language" {
                return val;
            }
        }
    }
    if theme_direction(theme) == "rtl" {
        "ar"
    } else {
        "en"
    }
}

fn named_color_scale(name: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match name.trim() {
        "indigo" => Some(("#4f46e5", "#eef2ff", "#312e81")),
        "cyan" => Some(("#06b6d4", "#ecfeff", "#164e63")),
        "violet" => Some(("#7c3aed", "#f5f3ff", "#4c1d95")),
        "emerald" => Some(("#059669", "#ecfdf5", "#064e3b")),
        "rose" => Some(("#e11d48", "#fff1f2", "#881337")),
        "slate" => Some(("#334155", "#f1f5f9", "#0f172a")),
        _ => None,
    }
}

fn safe_css_literal(value: &str, fallback: &str) -> String {
    let text = value.trim();
    if text.is_empty() || text.len() > 260 {
        return fallback.to_string();
    }
    let lower = text.to_lowercase();
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
        return fallback.to_string();
    }
    if !text.chars().all(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || ".,#%()+-/*".contains(c)) {
        return fallback.to_string();
    }
    text.to_string()
}

fn safe_font_name(value: &str, fallback: &str) -> String {
    let text = value.trim();
    if text.is_empty() || text.len() > 80 {
        return fallback.to_string();
    }
    if !text
        .chars()
        .all(|c| c.is_alphanumeric() || c.is_whitespace() || matches!(c, '.' | '_' | '-'))
    {
        return fallback.to_string();
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn font_stack(primary: &str, fallbacks: &str) -> String {
    format!("'{}', {}", primary.replace('\'', ""), fallbacks)
}

fn google_font_query(fonts: &[String]) -> String {
    let mut seen = std::collections::BTreeSet::new();
    let mut families = Vec::new();
    for font in fonts {
        if font.is_empty() || !seen.insert(font.clone()) {
            continue;
        }
        let encoded = font.replace(' ', "+");
        families.push(format!(
            "family={}:wght@400;500;600;700;800;900",
            encoded
        ));
    }
    families.join("&")
}

fn theme_color(value: &str, fallback_name: &str) -> (String, String, String) {
    let fallback = named_color_scale(fallback_name).unwrap_or(("#4f46e5", "#eef2ff", "#312e81"));
    if let Some(named) = named_color_scale(value) {
        return (named.0.to_string(), named.1.to_string(), named.2.to_string());
    }
    let base = safe_css_literal(value, fallback.0);
    let mix1 = format!("color-mix(in srgb, {} 16%, transparent)", base);
    let mix2 = format!("color-mix(in srgb, {} 58%, #020617)", base);
    (base, mix1, mix2)
}

fn theme_css(theme: Option<&ThemeIR>) -> String {
    let mut settings = HashMap::new();
    if let Some(t) = theme {
        for (k, v) in &t.settings {
            settings.insert(k.as_str(), v.as_str());
        }
    }
    
    let primary_val = settings.get("primary").copied().unwrap_or("indigo");
    let primary = theme_color(primary_val, "indigo");
    
    let accent_val = settings.get("accent").copied().unwrap_or("cyan");
    let accent = theme_color(accent_val, "cyan");
    
    let dark = settings.get("mode").map(|&v| v == "dark" || v == "night").unwrap_or(false);
    let direction = settings.get("direction").copied().unwrap_or("ltr");
    
    let start = if direction == "rtl" { "right" } else { "left" };
    let end = if direction == "rtl" { "left" } else { "right" };
    
    let radius_none = ("0px", "0px", "0px", "0px", "0px");
    let radius_sharp = ("4px", "8px", "12px", "16px", "20px");
    let radius_soft = ("10px", "16px", "22px", "28px", "36px");
    let radius_round = ("12px", "20px", "28px", "34px", "42px");
    let radius_pill = ("9999px", "9999px", "9999px", "9999px", "9999px");
    
    let radius_setting = settings.get("radius").copied().unwrap_or("soft");
    let radius = match radius_setting {
        "none" => radius_none,
        "sharp" => radius_sharp,
        "soft" => radius_soft,
        "round" => radius_round,
        "pill" => radius_pill,
        _ => radius_soft,
    };
    
    let density_compact = ("0.75rem", "1rem", "1.5rem");
    let density_comfortable = ("1rem", "1.5rem", "2.25rem");
    let density_spacious = ("1.25rem", "2rem", "3rem");
    
    let density_setting = settings.get("density").copied().unwrap_or("comfortable");
    let density = match density_setting {
        "compact" => density_compact,
        "comfortable" => density_comfortable,
        "spacious" => density_spacious,
        _ => density_comfortable,
    };
    
    let surface_glass = settings.get("surface").map(|&v| v == "glass").unwrap_or(false);
    
    let canvas_fallback = if dark { "#020617" } else { "#f8fafc" };
    let canvas = safe_css_literal(
        settings.get("canvas").or(settings.get("background")).or(settings.get("bg")).copied().unwrap_or(""),
        canvas_fallback
    );
    
    let base_fallback = if dark { "#0f172a" } else { "#ffffff" };
    let base = safe_css_literal(
        settings.get("base").or(settings.get("surface_base")).copied().unwrap_or(""),
        base_fallback
    );
    
    let muted_fallback = if dark { "#111827" } else { "#f8fafc" };
    let muted = safe_css_literal(
        settings.get("muted_surface").or(settings.get("surface_muted")).copied().unwrap_or(""),
        muted_fallback
    );
    
    let elevated_fallback = if surface_glass {
        if dark { "rgba(15,23,42,0.74)" } else { "rgba(255,255,255,0.72)" }
    } else {
        if dark { "#1f2937" } else { "#ffffff" }
    };
    let elevated = safe_css_literal(
        settings.get("elevated").or(settings.get("surface_elevated")).copied().unwrap_or(""),
        elevated_fallback
    );
    
    let text_fallback = if dark { "#f8fafc" } else { "#0f172a" };
    let text = safe_css_literal(
        settings.get("text").or(settings.get("ink")).copied().unwrap_or(""),
        text_fallback
    );
    
    let text_muted_fallback = if dark { "#cbd5e1" } else { "#475569" };
    let text_muted = safe_css_literal(
        settings.get("muted").or(settings.get("subtle")).copied().unwrap_or(""),
        text_muted_fallback
    );
    
    let border_fallback = if dark { "rgba(148,163,184,0.22)" } else { "rgba(15,23,42,0.10)" };
    let border = safe_css_literal(
        settings.get("border").copied().unwrap_or(""),
        border_fallback
    );
    
    let glass_bg_fallback = if dark { "rgba(15,23,42,0.66)" } else { "rgba(255,255,255,0.58)" };
    let glass_bg = safe_css_literal(
        settings.get("glass").or(settings.get("glass_bg")).copied().unwrap_or(""),
        glass_bg_fallback
    );
    
    let glass_border_fallback = if dark { "rgba(148,163,184,0.20)" } else { "rgba(255,255,255,0.38)" };
    let glass_border = safe_css_literal(
        settings.get("glass_border").copied().unwrap_or(""),
        glass_border_fallback
    );
    
    let gradient_primary_fallback = format!("linear-gradient(135deg, {}, {})", primary.0, accent.0);
    let gradient_primary = safe_css_literal(
        settings.get("gradient_primary").or(settings.get("gradient")).copied().unwrap_or(""),
        &gradient_primary_fallback
    );
    
    let gradient_accent_fallback = format!("linear-gradient(135deg, {}, {})", accent.0, primary.0);
    let gradient_accent = safe_css_literal(
        settings.get("gradient_accent").copied().unwrap_or(""),
        &gradient_accent_fallback
    );
    
    let gradient_hero_fallback = format!("radial-gradient(circle at top right, {}, transparent 30%), linear-gradient(135deg, {}, {})", primary.1, if dark { "#0f172a" } else { "#ffffff" }, accent.1);
    let gradient_hero = safe_css_literal(
        settings.get("gradient_hero").copied().unwrap_or(""),
        &gradient_hero_fallback
    );
    
    let radius_2xl_fallback = radius.4.to_string();
    let radius_2xl = safe_css_literal(
        settings.get("radius_2xl").copied().unwrap_or(""),
        &radius_2xl_fallback
    );

    let font_provider = settings.get("font_provider").copied().unwrap_or("system");
    let has_google_fonts = font_provider == "google";
    let body_family = safe_font_name(
        settings
            .get("font_family")
            .or_else(|| settings.get("body_font"))
            .or_else(|| settings.get("font"))
            .copied()
            .unwrap_or("Plus Jakarta Sans"),
        "Plus Jakarta Sans",
    );
    let heading_family = safe_font_name(
        settings
            .get("heading_font_family")
            .or_else(|| settings.get("heading_font"))
            .or_else(|| settings.get("display_font"))
            .copied()
            .unwrap_or("Outfit"),
        "Outfit",
    );
    let arabic_family = safe_font_name(
        settings
            .get("arabic_font_family")
            .or_else(|| settings.get("arabic_font"))
            .copied()
            .unwrap_or("IBM Plex Sans Arabic"),
        "IBM Plex Sans Arabic",
    );
    let system_fallbacks = "-apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif";
    let body_stack = font_stack(
        &body_family,
        &format!("'{}', 'Tajawal', {}", arabic_family, system_fallbacks),
    );
    let heading_stack = font_stack(
        &heading_family,
        &format!(
            "'{}', '{}', 'Tajawal', {}",
            body_family, arabic_family, system_fallbacks
        ),
    );
    
    let font_import = if has_google_fonts {
        let font_query = google_font_query(&[
            heading_family.clone(),
            body_family.clone(),
            arabic_family.clone(),
            "Tajawal".to_string(),
        ]);
        format!(
            "@import url('https://fonts.googleapis.com/css2?{}&display=swap');\n",
            font_query
        )
    } else {
        String::new()
    };

    let body_font = format!("400 1rem/1.6 {}", body_stack);

    let heading_font = format!("700 1.75rem/1.2 {}", heading_stack);

    let gradient_mesh_fallback = format!("radial-gradient(circle at 10% 20%, {}, transparent 34%), radial-gradient(circle at 80% 0%, {}, transparent 38%), {}", primary.1, accent.1, base);
    let gradient_mesh = safe_css_literal(settings.get("gradient_mesh").copied().unwrap_or(""), &gradient_mesh_fallback);
    
    let gradient_aurora_fallback = format!("radial-gradient(circle at 15% 20%, {}, transparent 30%), radial-gradient(circle at 80% 20%, {}, transparent 35%), {}", primary.1, accent.1, canvas);
    let gradient_aurora = safe_css_literal(settings.get("gradient_aurora").copied().unwrap_or(""), &gradient_aurora_fallback);

    let gradient_spotlight_fallback = format!("radial-gradient(circle at 50% 0%, {}, transparent 48%), {}", primary.1, base);
    let gradient_spotlight = safe_css_literal(settings.get("gradient_spotlight").copied().unwrap_or(""), &gradient_spotlight_fallback);
    
    let success_color = safe_css_literal(
        settings.get("success").copied().unwrap_or(""),
        "#16a34a"
    );
    let warning_color = safe_css_literal(
        settings.get("warning").copied().unwrap_or(""),
        "#ca8a04"
    );
    let danger_color = safe_css_literal(
        settings.get("danger").copied().unwrap_or(""),
        "#dc2626"
    );

    let glow_primary = format!("0 0 0 4px {}, 0 18px 40px -24px {}", primary.1, primary.0);
    let glow_accent = format!("0 0 0 4px {}, 0 18px 40px -24px {}", accent.1, accent.0);

    format!(
        r#"{}
    :root {{
      --color-primary: {};
      --color-primary-soft: {};
      --color-accent: {};
      --amana-direction: {};
      --amana-start: {};
      --amana-end: {};
      --bg-primary: {};
      --bg-secondary: {};
      --text-primary: {};
      --text-secondary: {};
      --surface-base: {};
      --surface-muted: {};
      --surface-elevated: {};
      --border-subtle: {};
      --glass-bg: {};
      --glass-border: {};
      --glass-blur: blur(16px);
      --radius-sm: {};
      --radius-md: {};
      --radius-lg: {};
      --radius-xl: {};
      --radius-2xl: {};
      --radius-small: var(--radius-sm);
      --radius-medium: var(--radius-md);
      --radius-large: var(--radius-lg);
      --radius-soft: var(--radius-md);
      --space-xs: 0.25rem;
      --space-sm: 0.5rem;
      --space-md: {};
      --space-lg: {};
      --space-xl: {};
      --space-2xl: 3rem;
      --space-3xl: 4.5rem;
      --space-4xl: 6rem;
      --text-xs: 0.75rem;
      --text-sm: 0.875rem;
      --text-md: 1rem;
      --text-lg: 1.125rem;
      --text-xl: 1.35rem;
      --text-2xl: 1.75rem;
      --text-3xl: 2.4rem;
      --shadow-sm: 0 1px 3px rgba(15,23,42,0.08);
      --shadow-md: 0 4px 6px -1px rgba(15,23,42,0.10);
      --shadow-lg: 0 10px 24px -8px rgba(15,23,42,0.18);
      --shadow-xl: 0 20px 40px -12px rgba(15,23,42,0.28);
      --transition-fast: all 0.12s ease-in-out;
      --color-success: {};
      --color-warning: {};
      --color-danger: {};
      --border-color: var(--border-subtle);
      --content-width: 1120px;
      --wide-width: 1360px;
      --readable-width: 72ch;
      --gradient-primary: {};
      --gradient-accent: {};
      --gradient-hero: {};
      --gradient-mesh: {};
      --gradient-aurora: {};
      --gradient-spotlight: {};
      --shadow-soft: 0 10px 24px -18px rgba(15,23,42,0.35);
      --shadow-floating: 0 24px 55px -28px rgba(15,23,42,0.45);
      --shadow-strong: 0 30px 70px -30px rgba(2,6,23,0.62);
      --elevation-1: 0 1px 2px rgba(15,23,42,0.08);
      --elevation-2: 0 8px 18px -14px rgba(15,23,42,0.35);
      --elevation-3: 0 18px 36px -22px rgba(15,23,42,0.45);
      --elevation-4: 0 28px 55px -30px rgba(15,23,42,0.55);
      --elevation-5: 0 35px 80px -35px rgba(15,23,42,0.68);
      --layer-base: 0;
      --layer-raised: 10;
      --layer-sticky: 30;
      --layer-dropdown: 60;
      --layer-overlay: 80;
      --layer-modal: 100;
      --layer-toast: 120;
      --glow-primary: {};
      --glow-accent: {};
      --font-body: {};
      --font-heading: {};
    }}
    html {{ direction: {}; }}
    body {{ direction: {}; color-scheme: {}; background: var(--bg-secondary); color: var(--text-primary); font: var(--font-body); }}
    :where(h1, h2, h3, h4, h5, h6) {{ font: var(--font-heading); }}
"#,
        font_import,
        primary.0,
        primary.1,
        accent.0,
        direction,
        start,
        end,
        base,
        canvas,
        text,
        text_muted,
        base,
        muted,
        elevated,
        border,
        glass_bg,
        glass_border,
        radius.0,
        radius.1,
        radius.2,
        radius.3,
        radius_2xl,
        density.0,
        density.1,
        density.2,
        &success_color,
        &warning_color,
        &danger_color,
        gradient_primary,
        gradient_accent,
        gradient_hero,
        gradient_mesh,
        gradient_aurora,
        gradient_spotlight,
        glow_primary,
        glow_accent,
        body_font,
        heading_font,
        direction,
        direction,
        if dark { "dark" } else { "light" }
    )
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
        ejs_body = format!("<div x-data=\"{}\">\n{}\n</div>", x_data_str, ejs_body);
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
  <script defer src="https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js"></script>
  <script defer src="https://code.iconify.design/iconify-icon/2.1.0/iconify-icon.min.js"></script>
  <style>
    {}
    {}
    {}
    :where(.amana-runtime-shell, .amana-page, .page) {{ width: 100%; max-width: 100%; overflow-x: hidden; }}
    :where(.amana-runtime-shell, .amana-page, .page) :where(section, header, main, footer, div, article, aside, form) {{ min-width: 0; }}
    :where(.amana-runtime-shell, .amana-page, .page) :where(h1, h2, h3, p, a, button, span, strong, label, input, textarea, pre) {{ max-width: 100%; overflow-wrap: anywhere; }}
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
        BASE_CSS_CLASSES,
        page_styles,
        body_attrs,
        ejs_body
    )
}

fn compile_default_login_ejs(
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
        html_lang,
        html_dir,
        bootstrap_css
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

const BASE_CSS_CLASSES: &str = r#"
    *, *::before, *::after { box-sizing: border-box; }
    html { width: 100%; max-width: 100%; overflow-x: hidden; scroll-behavior: smooth; }
    body { width: 100%; max-width: 100%; min-width: 0; margin: 0; overflow-x: hidden; background-color: var(--bg-secondary); color: var(--text-primary); font: var(--font-body); text-rendering: geometricPrecision; }
    body.amana-page { display: block; padding: 0 !important; gap: normal !important; }
    :where(main, section, article, aside, header, footer, nav, div, form) { min-width: 0; }
    :where(h1, h2, h3, h4, h5, h6, p, span, strong, a, button, label, input, textarea, pre) { max-width: 100%; overflow-wrap: anywhere; word-break: normal; letter-spacing: 0; }
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
    .amana-section h2, .amana-section-head h2 { margin: 0; font-size: clamp(2rem, 5vw, 4.2rem); line-height: 1.05; letter-spacing: 0; font-weight: 900; }
    .amana-section-copy { color: var(--text-secondary); max-width: 68ch; font-size: clamp(1rem, 2vw, 1.2rem); line-height: 1.8; }
    .amana-grid { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, var(--dg-columns, repeat(auto-fit, minmax(var(--grid-min, 16rem), 1fr))))); gap: var(--component-gap, var(--custom-gap, var(--dg-gap, var(--space-lg)))); }
    .amana-grid > *, .amana-split > *, .dg-layout-split-diagonal > *, .dg-layout-asymmetric > *, .dg-layout-editorial > *, .dg-layout-dashboard-shell > *, .dg-layout-command-center > *, .dg-layout-showcase-rail > * { min-width: 0; }
    .amana-stack { display: flex; flex-direction: column; gap: var(--space-md); }
    .amana-stack-gap-xs { gap: var(--space-xs); }
    .amana-stack-gap-sm { gap: var(--space-sm); }
    .amana-stack-gap-lg { gap: var(--space-lg); }
    .amana-stack-gap-xl { gap: var(--space-xl); }
    .amana-navbar { width: var(--component-width, min(100% - 2rem, var(--wide-width))); max-width: var(--component-max-width, none); margin-inline: auto; display: flex; align-items: center; justify-content: space-between; gap: var(--component-gap, var(--space-lg)); padding: var(--component-padding, 0.85rem 0); min-height: var(--component-min-height, 4.25rem); }
    .amana-brand { display: inline-flex; align-items: center; gap: 0.65rem; color: var(--text-primary); font-weight: 900; text-decoration: none; letter-spacing: 0; }
    .amana-brand::before { content: ""; width: 0.72rem; height: 0.72rem; border-radius: 999px; background: var(--gradient-primary); box-shadow: var(--glow-accent); }
    .amana-navlinks { display: flex; align-items: center; justify-content: flex-end; gap: var(--space-sm); flex-wrap: wrap; }
    .amana-hero { position: relative; isolation: isolate; display: grid; grid-template-columns: var(--component-columns, var(--dg-template, var(--dg-columns, minmax(0, 1fr)))); gap: var(--component-gap, var(--custom-gap, var(--dg-gap, clamp(1.5rem, 4vw, 3.5rem)))); align-items: center; width: var(--component-width, auto); max-width: var(--component-max-width, none); min-width: var(--component-min-width, 0); min-height: var(--component-min-height, auto); height: var(--component-height, auto); padding: var(--component-padding, clamp(1.5rem, 4vw, 4rem)); background: var(--custom-bg, var(--custom-gradient, var(--gradient-hero))); border: 1px solid var(--custom-border, var(--border-subtle)); border-radius: var(--custom-radius, var(--radius-2xl)); overflow: hidden; box-shadow: var(--custom-shadow, var(--shadow-floating)); opacity: var(--component-opacity, 1); transform: var(--component-transform, none); transition: var(--component-transition, transform 180ms ease, box-shadow 180ms ease, border-color 180ms ease); }
    .amana-hero::before { content: ""; position: absolute; inset: -20%; background: radial-gradient(circle at 15% 20%, rgba(34,211,238,0.18), transparent 28%), radial-gradient(circle at 85% 20%, rgba(99,102,241,0.22), transparent 32%); z-index: -1; }
    .amana-hero-content { display: grid; gap: var(--component-gap, var(--space-md)); max-width: var(--component-copy-width, 780px); min-width: 0; }
    .amana-hero h1 { margin: 0; font-size: var(--component-title-size, clamp(2.25rem, 5.4vw, 5.4rem)); line-height: var(--component-title-leading, 1.02); max-width: var(--component-title-width, min(100%, 16ch)); letter-spacing: 0; font-weight: var(--dg-type-weight, 900); }
    .amana-hero-copy { margin: 0; max-width: var(--component-copy-width, 66ch); color: var(--custom-muted, var(--text-secondary)); font-size: var(--component-copy-size, clamp(1rem, 1.8vw, 1.2rem)); line-height: 1.8; }
    .amana-hero-actions { display: flex; gap: var(--component-gap, var(--space-md)); flex-wrap: wrap; margin-top: var(--space-md); align-items: center; }
    .amana-hero-proof { color: var(--text-secondary); font-weight: 800; }
    .amana-hero-media { min-height: clamp(16rem, 34vw, 28rem); border-radius: var(--radius-2xl); background-size: cover; background-position: center; border: 1px solid var(--border-subtle); box-shadow: var(--shadow-floating); }
    .amana-eyebrow { color: var(--color-accent); font-weight: 900; text-transform: uppercase; letter-spacing: 0.1em; font-size: var(--text-sm); }
    .amana-card { position: relative; display: grid; gap: var(--component-gap, var(--custom-gap, var(--space-md))); width: var(--component-width, auto); max-width: var(--component-max-width, none); min-width: var(--component-min-width, 0); min-height: var(--component-min-height, 100%); height: var(--component-height, auto); background: var(--custom-bg, var(--custom-gradient, linear-gradient(180deg, color-mix(in srgb, var(--surface-elevated) 92%, transparent), color-mix(in srgb, var(--surface-muted) 82%, transparent)))); border: 1px solid var(--custom-border, var(--border-subtle)); border-radius: var(--custom-radius, var(--radius-2xl)); padding: var(--component-padding, var(--custom-padding, clamp(1.1rem, 2.6vw, 1.8rem))); box-shadow: var(--custom-shadow, var(--shadow-soft)); opacity: var(--component-opacity, 1); transform: var(--component-transform, none); overflow: hidden; transition: var(--component-transition, transform 180ms ease, box-shadow 180ms ease, border-color 180ms ease); }
    .amana-card::before { content: ""; position: absolute; inset: 0; pointer-events: none; background: linear-gradient(135deg, rgba(255,255,255,0.08), transparent 38%); opacity: 0.75; }
    .amana-card:hover { transform: translateY(-4px); box-shadow: var(--shadow-floating); border-color: color-mix(in srgb, var(--color-accent) 32%, var(--border-subtle)); }
    .amana-card > * { position: relative; }
    .amana-card h3 { margin: 0; font-size: clamp(1.25rem, 2vw, 1.75rem); line-height: 1.15; font-weight: 900; }
    .amana-feature-card { min-height: 13rem; }
    .amana-pricing-card { display: flex; flex-direction: column; gap: var(--space-md); }
    .amana-price { font-size: clamp(2rem, 5vw, 3.5rem); line-height: 1; font-weight: 950; }
    .amana-muted { color: var(--text-secondary); line-height: 1.75; }
    .amana-btn { position: relative; display: inline-flex; align-items: center; justify-content: center; gap: var(--component-gap, 0.65rem); width: var(--component-width, auto); max-width: var(--component-max-width, 100%); min-width: var(--component-min-width, 0); min-height: var(--component-min-height, 3rem); height: var(--component-height, auto); padding: var(--component-padding, 0.78rem 1.12rem); border-radius: var(--custom-radius, 999px); font-weight: 900; text-decoration: none; border: 1px solid var(--custom-border, transparent); transition: var(--component-transition, transform 160ms ease, box-shadow 160ms ease, border-color 160ms ease, background 160ms ease); white-space: nowrap; line-height: 1.15; overflow: hidden; opacity: var(--component-opacity, 1); transform: var(--component-transform, none); }
    .amana-btn:hover { transform: translateY(-2px); }
    .amana-btn:active { transform: translateY(0) scale(0.99); }
    .amana-btn:focus-visible { outline: 3px solid color-mix(in srgb, var(--color-accent) 60%, transparent); outline-offset: 3px; }
    .amana-btn-primary { background: var(--gradient-primary); color: white; box-shadow: var(--glow-primary); }
    .amana-btn-primary:hover { box-shadow: var(--shadow-floating), var(--glow-primary); }
    .amana-btn-secondary { color: var(--text-primary); background: color-mix(in srgb, var(--surface-elevated) 82%, transparent); border-color: var(--border-subtle); box-shadow: var(--shadow-soft); }
    .amana-btn-ghost { color: var(--text-primary); background: transparent; border-color: var(--border-subtle); }
    .amana-btn-sm { min-height: 2.35rem; padding: 0.52rem 0.82rem; font-size: var(--text-sm); }
    .amana-btn-lg { min-height: 3.45rem; padding: 0.96rem 1.35rem; font-size: var(--text-lg); }
    .amana-icon, .amana-btn-icon, iconify-icon { display: inline-grid; place-items: center; width: 1.25em; min-width: 1.25em; height: 1.25em; line-height: 1; vertical-align: -0.18em; transition: transform 160ms ease; }
    .amana-btn:hover .amana-btn-icon { transform: translateX(-2px); }
    .amana-btn-intent-danger { background: var(--color-danger); color: white; }
    .amana-btn-intent-success { background: var(--color-success); color: white; }
    .amana-field { display: flex; flex-direction: column; gap: 0.45rem; margin-bottom: var(--space-md); }
    .amana-field span { color: var(--text-primary); font-weight: 800; }
    .amana-field input, .amana-form-control { width: 100%; border: 1px solid var(--border-subtle); border-radius: var(--radius-soft); min-height: 3rem; padding: 0.78rem 0.92rem; background: color-mix(in srgb, var(--surface-base) 84%, transparent); color: var(--text-primary); box-shadow: inset 0 1px 0 rgba(255,255,255,0.04); }
    .amana-field input:focus, .amana-form-control:focus { outline: 3px solid color-mix(in srgb, var(--color-accent) 34%, transparent); border-color: var(--color-accent); }
    .amana-form-card { background: color-mix(in srgb, var(--surface-elevated) 88%, transparent); border: 1px solid var(--border-subtle); border-radius: var(--radius-2xl); padding: clamp(1.25rem, 3vw, 2rem); box-shadow: var(--shadow-floating); }
    .amana-help { color: var(--text-secondary); font-size: var(--text-sm); }
    .amana-alert { border-radius: var(--radius-soft); border: 1px solid var(--border-subtle); padding: var(--space-md); background: var(--surface-muted); }
    .amana-alert-success { border-color: rgba(22,163,74,0.35); }
    .amana-alert-danger { border-color: rgba(220,38,38,0.35); }
    .amana-footer { width: min(100% - 2rem, var(--wide-width)); margin: var(--space-3xl) auto 0; padding-block: var(--space-xl); color: var(--text-secondary); border-top: 1px solid var(--border-subtle); }
    .amana-modal { position: fixed; inset: 0; display: grid; place-items: center; background: rgba(2,6,23,0.55); padding: var(--space-lg); backdrop-filter: blur(10px); }
    .amana-modal-panel { width: min(100%, 36rem); background: var(--surface-elevated); border-radius: var(--radius-2xl); padding: var(--space-lg); box-shadow: var(--shadow-strong); }
    .amana-tabs { display: flex; gap: var(--space-sm); flex-wrap: wrap; border-bottom: 1px solid var(--border-subtle); padding-bottom: var(--space-sm); }
    .amana-card-top { display: flex; align-items: center; justify-content: space-between; gap: var(--space-sm); margin-bottom: var(--space-xs); }
    .amana-card-meta { color: var(--text-secondary); font-size: var(--text-sm); }
    .amana-card-action { color: var(--color-accent); font-weight: 900; text-decoration: none; margin-top: auto; }
    .amana-card-density-compact { padding: var(--space-md); }
    .amana-card-density-spacious { padding: var(--space-xl); }
    .amana-badge { display: inline-flex; align-items: center; width: fit-content; gap: 0.35rem; border: 1px solid var(--border-subtle); border-radius: 999px; padding: 0.38rem 0.78rem; font-size: var(--text-sm); font-weight: 900; background: color-mix(in srgb, var(--surface-muted) 78%, transparent); color: var(--text-primary); box-shadow: var(--shadow-soft); }
    .amana-badge-success { border-color: rgba(22,163,74,0.35); color: var(--color-success); }
    .amana-badge-warning { border-color: rgba(202,138,4,0.35); color: var(--color-warning); }
    .amana-badge-danger { border-color: rgba(220,38,38,0.35); color: var(--color-danger); }
    .amana-kpi { display: grid; gap: 0.35rem; padding: clamp(1.25rem, 3vw, 2rem); border: 1px solid var(--border-subtle); border-radius: var(--radius-2xl); background: linear-gradient(180deg, var(--surface-elevated), var(--surface-muted)); box-shadow: var(--shadow-soft); }
    .amana-kpi-label { color: var(--text-secondary); font-size: var(--text-sm); }
    .amana-kpi-value { font-size: clamp(2rem, 5vw, 4rem); line-height: 1; }
    .amana-kpi-trend { color: var(--color-success); font-weight: 700; }
    .amana-logo-cloud { display: grid; gap: var(--space-md); padding-block: var(--space-lg); }
    .amana-logo-row { display: flex; flex-wrap: wrap; gap: var(--space-md); align-items: center; color: var(--text-secondary); }
    .amana-testimonial { margin: 0; display: grid; gap: var(--space-md); border: 1px solid var(--border-subtle); border-radius: var(--radius-xl); padding: var(--space-lg); background: var(--surface-elevated); box-shadow: var(--shadow-soft); }
    .amana-testimonial blockquote { margin: 0; font-size: var(--text-lg); color: var(--text-primary); }
    .amana-testimonial figcaption { display: grid; gap: 0.1rem; color: var(--text-secondary); }
    .amana-timeline { display: grid; gap: var(--space-md); list-style: none; padding: 0; margin: 0; }
    .amana-timeline-item { position: relative; padding: var(--space-lg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xl); background: var(--surface-elevated); }
    .amana-empty-state { display: grid; place-items: center; text-align: center; gap: var(--space-md); min-height: 18rem; border: 1px dashed var(--border-subtle); border-radius: var(--radius-xl); padding: var(--space-xl); background: var(--surface-muted); }
    .amana-split { display: grid; grid-template-columns: minmax(0, 1fr) minmax(16rem, 0.85fr); gap: var(--space-xl); align-items: center; }
    .amana-cluster { display: flex; flex-wrap: wrap; gap: var(--space-md); align-items: center; }
    .amana-sidebar { border: 1px solid var(--border-subtle); border-radius: var(--radius-xl); background: var(--surface-elevated); padding: var(--space-lg); box-shadow: var(--shadow-soft); }
    .amana-navbar-sticky { position: sticky; top: 0; z-index: 20; background: color-mix(in srgb, var(--surface-base) 78%, transparent); backdrop-filter: blur(12px); border-bottom: 1px solid var(--border-subtle); }
    .amana-page { min-height: 100vh; background: var(--bg-secondary); color: var(--text-primary); overflow-x: hidden; }
    .amana-runtime-shell { display: block; width: 100%; max-width: 100%; min-height: 100vh; margin: 0; padding: 0; overflow-x: hidden; }
    .amana-runtime-shell > :not(script):not(style) { max-width: 100%; }
    .dg-canvas-width-full .amana-container { width: 100%; max-width: none; }
    .dg-canvas-width-wide .amana-container { width: min(100% - 2rem, var(--wide-width)); }
    .dg-canvas-width-readable .amana-container { width: min(100% - 2rem, var(--readable-width)); }
    .dg-layout-split-diagonal,
    .dg-layout-asymmetric { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, minmax(0, 1.1fr) minmax(16rem, 0.85fr))); align-items: center; gap: var(--component-gap, var(--dg-gap, clamp(1.5rem, 5vw, 4rem))); }
    :where(.dg-layout-split-diagonal, .dg-layout-asymmetric, .dg-layout-editorial, .dg-layout-dashboard-shell, .dg-layout-magazine, .dg-layout-bento, .dg-layout-command-center, .dg-layout-showcase-rail) > .amana-container { grid-column: 1 / -1; width: min(100% - 2rem, var(--content-width)); }
    .dg-layout-centered { text-align: center; justify-items: center; }
    .dg-layout-editorial { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, minmax(14rem, 0.55fr) minmax(0, 1fr))); gap: var(--component-gap, var(--dg-gap, clamp(2rem, 6vw, 5rem))); align-items: start; }
    .dg-layout-dashboard-shell,
    :where(.dg-layout-dashboard-shell) .amana-runtime-shell > :not(script):not(style) { display: grid; grid-template-columns: var(--component-columns, var(--dg-template, minmax(14rem, 18rem) minmax(0, 1fr))); gap: var(--component-gap, var(--dg-gap, var(--space-lg))); align-items: start; }
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
      .amana-navbar { align-items: flex-start; flex-direction: column; }
      .amana-hero { padding-inline: var(--space-lg); }
      .amana-hero h1 { font-size: 2.6rem; }
      .amana-split,
      .dg-flow-dashboard,
      .dg-layout-split-diagonal,
      .dg-layout-asymmetric,
      .dg-layout-editorial,
      .dg-layout-dashboard-shell,
      :where(.dg-layout-dashboard-shell) .amana-runtime-shell > :not(script):not(style),
      .dg-layout-magazine,
      .dg-layout-bento,
      .dg-layout-command-center,
      .dg-layout-showcase-rail { grid-template-columns: 1fr; }
      .dg-layout-magazine > *,
      .dg-layout-bento > *,
      .dg-layout-bento > *:first-child { grid-column: auto; grid-row: auto; }
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
