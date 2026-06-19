// src/codegen/express/static_files/engine.rs

pub(crate) fn engine_js() -> &'static str {
    r#"const express = require('express');
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

function compileExpressionToJs(expr, authModel = 'User') {
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
    return id === `${authModel}.current` || id === 'User.current' ? 'currentUser' : id;
  }
  if (expr.Binary !== undefined) {
    const { left, op, right } = expr.Binary;
    const l = compileExpressionToJs(left, authModel);
    const r = compileExpressionToJs(right, authModel);
    const jsOp = op === 'and' ? '&&' : (op === 'or' ? '||' : op);
    return `(${l} ${jsOp} ${r})`;
  }
  if (expr.Unary !== undefined) {
    const { op, expr: innerExpr } = expr.Unary;
    const e = compileExpressionToJs(innerExpr, authModel);
    const jsOp = op === 'not' ? '!' : op;
    return `(${jsOp}${e})`;
  }
  if (expr.Ternary !== undefined) {
    const { cond, then_branch, else_branch } = expr.Ternary;
    const c = compileExpressionToJs(cond, authModel);
    const t = compileExpressionToJs(then_branch, authModel);
    const el = compileExpressionToJs(else_branch, authModel);
    return `(${c} ? ${t} : ${el})`;
  }
  if (expr.MemberAccess !== undefined) {
    const { object, property } = expr.MemberAccess;
    const obj = compileExpressionToJs(object, authModel);
    if ((obj === authModel || obj === 'User') && property === 'current') return 'currentUser';
    return `${obj}.${property}`;
  }
  if (expr.Call !== undefined) {
    const { callee, args } = expr.Call;
    if (callee.Identifier === 'env') {
      if (args.length === 1) {
        return `(process.env[${compileExpressionToJs(args[0], authModel)}] || "")`;
      } else if (args.length === 2) {
        return `(process.env[${compileExpressionToJs(args[0], authModel)}] || ${compileExpressionToJs(args[1], authModel)})`;
      }
    }
    const c = compileExpressionToJs(callee, authModel);
    const formattedArgs = args.map(arg => compileExpressionToJs(arg, authModel));
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
    if (attrName === 'columns') {
      const val = /^\d+$/.test(String(cleanValue).trim())
        ? (String(cleanValue).trim() === '1' ? 'minmax(0, 1fr)' : `repeat(${cleanValue}, minmax(0, 1fr))`)
        : cleanValue;
      styles.push(`${cssVar}:${val}`);
      styles.push(`--dg-columns:${val}`);
    } else {
      styles.push(`${cssVar}:${cleanValue}`);
      if (cssProp) styles.push(`${cssProp}:${cleanValue}`);
    }
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
      if (canSizeComponent && ['columns', 'layout.columns'].includes(key)) { const val = /^\d+$/.test(String(cleanValue).trim()) ? (String(cleanValue).trim() === '1' ? 'minmax(0, 1fr)' : `repeat(${cleanValue}, minmax(0, 1fr))`) : cleanValue; styles.push(`--component-columns:${val};--dg-columns:${val}`); }
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
      if (kind === 'compose' && key === 'columns') { const val = /^\d+$/.test(String(cleanValue).trim()) ? (String(cleanValue).trim() === '1' ? 'minmax(0, 1fr)' : `repeat(${cleanValue}, minmax(0, 1fr))`) : cleanValue; styles.push(`--dg-columns:${val}`); }
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
      if (kind === 'responsive' && key === 'columns') { const val = /^\d+$/.test(String(cleanValue).trim()) ? (String(cleanValue).trim() === '1' ? 'minmax(0, 1fr)' : `repeat(${cleanValue}, minmax(0, 1fr))`) : cleanValue; styles.push(`--dg-responsive-columns:${val}`); }
      if (kind === 'responsive' && key === 'desktop.columns') { const val = /^\d+$/.test(String(cleanValue).trim()) ? (String(cleanValue).trim() === '1' ? 'minmax(0, 1fr)' : `repeat(${cleanValue}, minmax(0, 1fr))`) : cleanValue; styles.push(`--bp-desktop-columns:${val}`); }
      if (kind === 'responsive' && key === 'laptop.columns') { const val = /^\d+$/.test(String(cleanValue).trim()) ? (String(cleanValue).trim() === '1' ? 'minmax(0, 1fr)' : `repeat(${cleanValue}, minmax(0, 1fr))`) : cleanValue; styles.push(`--bp-laptop-columns:${val}`); }
      if (kind === 'responsive' && key === 'tablet.columns') { const val = /^\d+$/.test(String(cleanValue).trim()) ? (String(cleanValue).trim() === '1' ? 'minmax(0, 1fr)' : `repeat(${cleanValue}, minmax(0, 1fr))`) : cleanValue; styles.push(`--bp-tablet-columns:${val}`); }
      if (kind === 'responsive' && key === 'mobile.columns') { const val = /^\d+$/.test(String(cleanValue).trim()) ? (String(cleanValue).trim() === '1' ? 'minmax(0, 1fr)' : `repeat(${cleanValue}, minmax(0, 1fr))`) : cleanValue; styles.push(`--bp-mobile-columns:${val}`); }
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
    let colVal = columns;
    if (columns) {
      colVal = /^\d+$/.test(String(columns).trim())
        ? (String(columns).trim() === '1' ? 'minmax(0, 1fr)' : `repeat(${columns}, minmax(0, 1fr))`)
        : columns;
    }
    const rawAttrs = attrsFor('amana-grid');
    const gridVars = `--grid-min:${escapeAttr(min)};${columns ? `--dg-columns:${escapeAttr(colVal)};` : ''}`;
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
    const variant = getAttr(attributes, 'variant', 'default');
    const variantClass = (variant && variant !== 'default') ? ` amana-navbar-${variant}` : '';
    return `<nav${attrsFor(`amana-navbar${sticky ? ' amana-navbar-sticky' : ''}${variantClass}`)}><a class="amana-brand" href="/">${brand}</a><div class="amana-navlinks">${inner}</div></nav>`;
  }
  if (tag === 'Slides') {
    const autoplay = getAttr(attributes, 'autoplay', 'false') === 'true';
    const height = getAttr(attributes, 'height', '400px');
    const effect = getAttr(attributes, 'effect', 'slide');
    const childCount = renderChildren.length;
    
    const slidesHtml = renderChildren.map((child, i) => {
      const childRendered = generateEjs(child, clientStates, dataVar);
      const transitionAttrs = effect === 'fade'
        ? `x-transition:enter="transition ease-out duration-500" x-transition:enter-start="opacity-0" x-transition:enter-end="opacity-100" x-transition:leave="transition ease-in duration-500" x-transition:leave-start="opacity-100" x-transition:leave-end="opacity-0"`
        : `x-transition:enter="transition ease-out duration-500" x-transition:enter-start="opacity-0 transform translate-x-4" x-transition:enter-end="opacity-100 transform translate-x-0" x-transition:leave="transition ease-in duration-300" x-transition:leave-start="opacity-100 transform translate-x-0" x-transition:leave-end="opacity-0 transform -translate-x-4"`;
      return `<div x-show="activeSlide === ${i}" ${transitionAttrs} style="display: none;">${childRendered}</div>`;
    }).join('');
    
    let dotsHtml = '';
    for (let i = 0; i < childCount; i++) {
      dotsHtml += `<span class="amana-slides-dot" :class="{ 'active': activeSlide === ${i} }" @click="activeSlide = ${i}"></span>`;
    }
    
    return `<div${attrsFor('amana-slides')} x-data="{ activeSlide: 0, slidesCount: ${childCount}, autoplay: ${autoplay}, init() { if (this.autoplay) { setInterval(() => { this.activeSlide = (this.activeSlide + 1) % this.slidesCount; }, 5000); } } }" style="height: ${escapeAttr(height)}; min-height: ${escapeAttr(height)};"><div class="amana-slides-inner">${slidesHtml}</div><button class="amana-slides-arrow prev" @click="activeSlide = (activeSlide - 1 + slidesCount) % slidesCount">&larr;</button><button class="amana-slides-arrow next" @click="activeSlide = (activeSlide + 1) % slidesCount">&rarr;</button><div class="amana-slides-dots">${dotsHtml}</div></div>`;
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
    const placeholderAttr = placeholder ? ` placeholder="${escapeAttr(placeholder)}"` : '';
    const inputHtml = type === 'textarea'
      ? `<textarea class="amana-form-control" name="${name}" id="${name}"${placeholderAttr} rows="4"></textarea>`
      : `<input class="amana-form-control" type="${escapeAttr(type)}" name="${name}" id="${name}"${placeholderAttr}>`;
    return `<label${attrsFor('amana-field')}><span>${label}</span>${inputHtml}</label>`;
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
    return `<div class="chart-container mb-4" style="position: relative; width:100%; max-width:100%; height:clamp(18rem, 48vw, 26rem)">\n  <canvas id="chart_${data_expr}" style="width:100%; height:100%"></canvas>\n</div>\n\
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
        attrs += ` x-on:${key}="${escapeAttr(compileExpressionToJs(expr))}"`;
      } else if (key === 'show') {
        attrs += ` x-show="${escapeAttr(compileExpressionToJs(expr))}"`;
      } else if (key === 'text') {
        attrs += ` x-text="${escapeAttr(compileExpressionToJs(expr))}"`;
      } else if (key === 'init') {
        const code = expr.StringLiteral !== undefined ? expr.StringLiteral : compileExpressionToJs(expr);
        attrs += ` x-init="${escapeAttr(code)}"`;
      } else if (['disabled', 'checked', 'selected', 'readonly'].includes(key)) {
        attrs += ` :${key}="${escapeAttr(compileExpressionToJs(expr))}"`;
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
  const authModel = scope.authModel || 'User';
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
    if (object && object.Identifier === authModel && property === 'current') return currentUser;
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
      case '=': return l == r;
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

  getAuthModelName() {
    return this.ir.app.auth_model || 'User';
  }

  getCurrentPrincipal(req) {
    return req && req.session ? req.session.user || null : null;
  }

  findModelByName(name) {
    const key = String(name || '').toLowerCase();
    return this.ir.models.find(m => m.name.toLowerCase() === key || m.table_name === key) || null;
  }

  modelHasPolicies(model) {
    return Boolean(model && Array.isArray(model.permissions) && model.permissions.length > 0);
  }

  authScope(req, currentUser, extra = {}) {
    return {
      authModel: this.getAuthModelName(),
      currentUser,
      params: (req && req.params) || {},
      query: (req && req.query) || {},
      body: (req && req.body) || {},
      ...extra
    };
  }

  principalRoles(principal) {
    const roles = new Set(['public']);
    if (!principal) {
      roles.add('guest');
      return roles;
    }
    roles.add('authenticated');
    roles.add('user');
    const addRole = role => {
      if (role !== null && role !== undefined && String(role).trim()) {
        roles.add(String(role).trim().toLowerCase());
      }
    };
    addRole(principal.role);
    addRole(principal.kind);
    addRole(principal.type);
    if (Array.isArray(principal.roles)) {
      for (const role of principal.roles) addRole(role);
    }
    return roles;
  }

  permissionActionMatches(ruleAction, requestedAction) {
    const rule = String(ruleAction || '').toLowerCase();
    const requested = String(requestedAction || '').toLowerCase();
    if (rule === '*' || rule === 'manage' || rule === requested) return true;
    if (requested === 'read' && ['list', 'find', 'count', 'view'].includes(rule)) return true;
    if (requested === 'create' && ['write', 'insert'].includes(rule)) return true;
    if (requested === 'register' && ['create', 'write', 'insert'].includes(rule)) return true;
    if (requested === 'update' && ['write', 'edit'].includes(rule)) return true;
    if (requested === 'delete' && ['write', 'remove', 'destroy'].includes(rule)) return true;
    return false;
  }

  permissionResourceMatches(ruleResource, model) {
    const resource = String(ruleResource || '').toLowerCase();
    return resource === '*' || resource === model.name.toLowerCase() || resource === model.table_name.toLowerCase();
  }

  candidatePermissionRules(model, action, currentUser) {
    if (!this.modelHasPolicies(model)) return null;
    const roles = this.principalRoles(currentUser);
    return model.permissions.filter(rule => {
      const role = String(rule.role || '').toLowerCase();
      return (role === '*' || roles.has(role))
        && this.permissionActionMatches(rule.action, action)
        && this.permissionResourceMatches(rule.resource, model);
    });
  }

  matchingPermissionRules(model, action, req, currentUser, row = null, submitted = {}) {
    if (!this.modelHasPolicies(model)) return null;
    const roles = this.principalRoles(currentUser);
    const rowScope = row && typeof row === 'object' ? row : {};
    const submittedScope = submitted && typeof submitted === 'object' ? submitted : {};
    const scope = this.authScope(req, currentUser, {
      ...submittedScope,
      ...rowScope,
      submitted: submittedScope,
      record: rowScope,
      row: rowScope,
      resource: rowScope
    });

    return model.permissions.filter(rule => {
      const role = String(rule.role || '').toLowerCase();
      if (role !== '*' && !roles.has(role)) return false;
      if (!this.permissionActionMatches(rule.action, action)) return false;
      if (!this.permissionResourceMatches(rule.resource, model)) return false;
      if (rule.where_expr !== null && rule.where_expr !== undefined) {
        try {
          if (!Boolean(evalAmanaExpression(rule.where_expr, req, currentUser, scope))) return false;
        } catch (_) {
          return false;
        }
      }
      return true;
    });
  }

  canPerform(model, action, req, currentUser, row = null, submitted = {}) {
    const rules = this.matchingPermissionRules(model, action, req, currentUser, row, submitted);
    if (rules === null) return true;
    return rules.length > 0;
  }

  ensurePermission(model, action, req, currentUser, row = null, submitted = {}) {
    if (!this.canPerform(model, action, req, currentUser, row, submitted)) {
      const err = new Error(`Permission denied for ${action} on ${model.name}.`);
      err.statusCode = currentUser ? 403 : 401;
      throw err;
    }
  }

  ensureFieldsAllowed(model, action, fields, req, currentUser, row = null, submitted = {}) {
    const rules = this.matchingPermissionRules(model, action, req, currentUser, row, submitted);
    if (rules === null) return;
    if (rules.length === 0) {
      this.ensurePermission(model, action, req, currentUser, row, submitted);
    }
    if (rules.some(rule => !Array.isArray(rule.fields) || rule.fields.length === 0)) return;
    const allowed = new Set();
    for (const rule of rules) {
      for (const field of rule.fields || []) {
        allowed.add(String(field).toLowerCase());
      }
    }
    const denied = fields
      .map(field => String(field).toLowerCase())
      .filter(field => field !== 'id' && !allowed.has(field));
    if (denied.length > 0) {
      const err = new Error(`Fields are not permitted for ${action}: ${denied.join(', ')}.`);
      err.statusCode = 403;
      throw err;
    }
  }

  readableRow(model, row, req, currentUser) {
    const rules = this.matchingPermissionRules(model, 'read', req, currentUser, row);
    if (rules === null) return row;
    if (rules.length === 0) return null;
    if (rules.some(rule => !Array.isArray(rule.fields) || rule.fields.length === 0)) return row;
    const allowed = new Set(['id']);
    for (const rule of rules) {
      for (const field of rule.fields || []) {
        allowed.add(String(field).toLowerCase());
      }
    }
    const filtered = {};
    for (const [key, value] of Object.entries(row || {})) {
      if (allowed.has(String(key).toLowerCase())) filtered[key] = value;
    }
    return filtered;
  }

  readableRows(model, rows, req, currentUser) {
    if (!this.modelHasPolicies(model)) return rows;
    return rows
      .map(row => this.readableRow(model, row, req, currentUser))
      .filter(row => row !== null);
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
    const authModel = this.findModelByName(this.getAuthModelName());
    if (authModel) {
      const adminEmail = process.env.AMANA_ADMIN_EMAIL;
      const adminPassword = process.env.AMANA_ADMIN_PASSWORD;
      if (!adminEmail || !adminPassword) {
        throw new Error('AMANA_ADMIN_EMAIL and AMANA_ADMIN_PASSWORD are required when AMANA_SEED_ADMIN=true.');
      }
      const userTable = quoteSqlIdentifier(authModel.table_name);
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

    const requireRestAccess = (req, res, model, action) => {
      const currentUser = this.getCurrentPrincipal(req);
      if (this.modelHasPolicies(model)) {
        const candidates = this.candidatePermissionRules(model, action, currentUser);
        if (!candidates || candidates.length === 0) {
          res.status(currentUser ? 403 : 401).json({ ok: false, error: 'Permission denied.' });
          return false;
        }
        return true;
      }
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
        if (!requireRestAccess(req, res, model, 'read')) return;
        try {
          const currentUser = this.getCurrentPrincipal(req);
          const requestedLimit = Number(req.query.limit || DEFAULT_QUERY_LIMIT);
          const requestedPage = Number(req.query.page || 1);
          const requestedOffset = req.query.offset !== undefined ? Number(req.query.offset) : undefined;
          const limit = Number.isFinite(requestedLimit) ? Math.max(1, Math.min(requestedLimit, 1000)) : DEFAULT_QUERY_LIMIT;
          const page = Number.isFinite(requestedPage) ? Math.max(1, requestedPage) : 1;
          const offset = Number.isFinite(requestedOffset) ? Math.max(0, requestedOffset) : (page - 1) * limit;
          const rows = await this.dbAll(`SELECT * FROM ${tableSql} LIMIT ? OFFSET ?`, [limit, offset]);
          const readableRows = this.readableRows(model, rows, req, currentUser);
          res.json({ data: readableRows, page, limit, offset });
        } catch (err) {
          console.error('[Amana API Error]', err);
          res.status(500).json({ error: 'Failed to load records.' });
        }
      });

      router.get(`${base}/:id`, async (req, res) => {
        if (!requireRestAccess(req, res, model, 'read')) return;
        try {
          const currentUser = this.getCurrentPrincipal(req);
          const row = await this.dbGet(`SELECT * FROM ${tableSql} WHERE "id" = ? LIMIT 1`, [req.params.id]);
          if (!row) return res.status(404).json({ error: 'Record not found.' });
          const readable = this.readableRow(model, row, req, currentUser);
          if (!readable) return res.status(currentUser ? 403 : 401).json({ error: 'Permission denied.' });
          res.json({ data: readable });
        } catch (err) {
          console.error('[Amana API Error]', err);
          res.status(500).json({ error: 'Failed to load record.' });
        }
      });

      router.post(base, async (req, res) => {
        if (!requireRestAccess(req, res, model, 'create')) return;
        try {
          const currentUser = this.getCurrentPrincipal(req);
          const insertFields = fields.filter(f => Object.prototype.hasOwnProperty.call(req.body, f));
          if (insertFields.length === 0) return res.status(400).json({ error: 'No accepted fields submitted.' });
          this.ensureFieldsAllowed(model, 'create', insertFields, req, currentUser, null, req.body);
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
          res.status(err.statusCode || (String(err.message || '').startsWith('Field ') ? 400 : 500)).json({ error: err.message || 'Failed to create record.' });
        }
      });

      router.put(`${base}/:id`, async (req, res) => {
        if (!requireRestAccess(req, res, model, 'update')) return;
        try {
          const currentUser = this.getCurrentPrincipal(req);
          const existing = await this.dbGet(`SELECT * FROM ${tableSql} WHERE "id" = ? LIMIT 1`, [req.params.id]);
          if (!existing) return res.status(404).json({ error: 'Record not found.' });
          const updateFields = fields.filter(f => Object.prototype.hasOwnProperty.call(req.body, f));
          if (updateFields.length === 0) return res.status(400).json({ error: 'No accepted fields submitted.' });
          this.ensureFieldsAllowed(model, 'update', updateFields, req, currentUser, existing, req.body);
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
          res.status(err.statusCode || (String(err.message || '').startsWith('Field ') ? 400 : 500)).json({ error: err.message || 'Failed to update record.' });
        }
      });

      router.delete(`${base}/:id`, async (req, res) => {
        if (!requireRestAccess(req, res, model, 'delete')) return;
        try {
          const currentUser = this.getCurrentPrincipal(req);
          const existing = await this.dbGet(`SELECT * FROM ${tableSql} WHERE "id" = ? LIMIT 1`, [req.params.id]);
          if (!existing) return res.status(404).json({ error: 'Record not found.' });
          this.ensurePermission(model, 'delete', req, currentUser, existing);
          await this.dbRun(`DELETE FROM ${tableSql} WHERE "id" = ?`, [req.params.id]);
          res.json({ ok: true });
        } catch (err) {
          console.error('[Amana API Error]', err);
          res.status(err.statusCode || 500).json({ error: err.message || 'Failed to delete record.' });
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
          scriptSrc: ["'self'", "'unsafe-inline'", "'unsafe-eval'", "cdn.jsdelivr.net", "code.iconify.design"],
          styleSrc: ["'self'", "'unsafe-inline'", "cdn.jsdelivr.net", "fonts.googleapis.com"],
          fontSrc: ["'self'", "fonts.gstatic.com"],
          connectSrc: ["'self'", "cdn.jsdelivr.net", "api.iconify.design"],
          imgSrc: ["'self'", "data:", "cdn.jsdelivr.net", "images.unsplash.com", "logoipsum.com"]
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
        ejs_template = `<div class="amana-state-scope" x-data="${escapeAttr(xDataStr)}">\n${ejs_template}\n</div>`;
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
      :where(.dg-layout-dashboard-shell) {
        height: auto !important;
        min-height: 100vh !important;
        overflow: auto !important;
      }
      :where(.dg-layout-dashboard-shell) .amana-runtime-shell {
        height: auto !important;
        min-height: 100vh !important;
        overflow: auto !important;
      }
      :where(.dg-layout-dashboard-shell) .amana-state-scope {
        height: auto !important;
        min-height: 100vh !important;
        overflow: visible !important;
      }
      .app-shell {
        display: flex !important;
        flex-direction: column !important;
        height: auto !important;
        min-height: 100vh !important;
        overflow: visible !important;
        padding: 0.5rem !important;
        gap: 1rem !important;
        width: 100% !important;
        max-width: 100% !important;
      }
      .side-rail {
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
      }
      .side-rail > [class*="brand"],
      .side-brand {
        width: auto !important;
        border-bottom: none !important;
        padding: 0 !important;
        flex-shrink: 0 !important;
      }
      .side-rail > [class*="nav"],
      .side-nav {
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
      }
      .side-rail > [class*="nav"]::-webkit-scrollbar,
      .side-nav::-webkit-scrollbar {
        display: none !important;
      }
      .side-rail > [class*="nav"] a,
      .side-rail a[class*="link"],
      .side-nav a {
        flex: 0 0 auto !important;
        font-size: 0.8rem !important;
        padding: 0.35rem 0.65rem !important;
        border-radius: 8px !important;
        white-space: nowrap !important;
      }
      .side-rail > [class*="footer"],
      .side-footer {
        display: none !important;
      }
      .dashboard-main {
        flex: 1 1 auto !important;
        height: auto !important;
        min-height: 0 !important;
        overflow: visible !important;
        width: 100% !important;
        max-width: 100% !important;
      }
      .workspace {
        padding: 0 !important;
      }
      :where(.dg-layout-dashboard-shell) .dashboard-main > :not(script):not(style) {
        padding-left: 1rem !important;
        padding-right: 1rem !important;
      }
      .dash-header {
        padding-left: 1rem !important;
        padding-right: 1rem !important;
        flex-wrap: wrap !important;
        gap: 1rem !important;
      }
      .dashboard-grid,
      .settings-layout,
      .ticket-detail-grid,
      .reports-grid,
      .reports-secondary {
        grid-template-columns: 1fr !important;
        padding-left: 1rem !important;
        padding-right: 1rem !important;
        gap: 1.25rem !important;
      }
      .reports-kpis {
        grid-template-columns: repeat(2, 1fr) !important;
        gap: 0.75rem !important;
      }
      .settings-nav {
        position: static !important;
        flex-direction: row !important;
        overflow-x: auto !important;
        flex-wrap: nowrap !important;
        scrollbar-width: none !important;
        width: 100% !important;
        padding: 0.25rem !important;
      }
      .settings-nav::-webkit-scrollbar {
        display: none !important;
      }
      .settings-nav-item {
        white-space: nowrap !important;
      }
      .inbox-split-pane {
        grid-template-columns: 1fr !important;
        min-height: auto !important;
      }
      .inbox-list-pane {
        border-right: none !important;
        padding-right: 0 !important;
        border-bottom: 1px solid rgba(17, 24, 39, 0.06) !important;
        padding-bottom: 1.25rem !important;
      }
      .inbox-detail-pane {
        padding-left: 0 !important;
      }
      .table-row {
        grid-template-columns: 1fr !important;
        gap: 0.5rem !important;
        align-items: flex-start !important;
      }
      .dg-rsp-mobile-stacked, .dg-responsive-mobile-stacked {
        grid-template-columns: minmax(0, 1fr) !important;
      }
      .dg-rsp-mobile-stacked > *, .dg-responsive-mobile-stacked > * {
        grid-column: span 1 / auto !important;
      }
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
          const authModel = this.findModelByName(this.getAuthModelName());
          if (!authModel) {
            throw new Error(`Auth model '${this.getAuthModelName()}' is not defined.`);
          }
          const user = await this.dbGet(`SELECT * FROM ${quoteSqlIdentifier(authModel.table_name)} WHERE "email" = ?`, [email]);
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
        const currentUser = this.getCurrentPrincipal(req);
        const routeScope = this.authScope(req, currentUser);
        if (r.guard) {
          if (!currentUser) {
            return res.redirect(r.guard.unauth_path);
          }
          const allowed = Boolean(evalAmanaExpression(r.guard.cond_expr, req, currentUser, routeScope));
          if (!allowed) {
            return res.redirect(r.guard.deny_path);
          }
        }

        try {
          const viewIr = this.ir.views.find(v => v.name.toLowerCase() === r.view_name.toLowerCase());
          const styles = viewIr ? viewIr.styles || '' : '';
          const renderVars = {
            ...this.authScope(req, currentUser),
            csrfToken: req.session.csrfToken,
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
              const fetchModel = this.findModelByName(fetch.model_name);
              if (!fetchModel) {
                throw new Error(`Fetch references unknown model '${fetch.model_name}'.`);
              }
              let effectiveMethod = fetch.query_method;
              if (fetch.query_method === 'count' && this.modelHasPolicies(fetchModel)) {
                effectiveMethod = fetch.query_args && fetch.query_args.length > 0 ? 'filter' : 'all';
              }
              const { sql, paramsJs } = generateSafeQuery(this.ir.models, fetch.model_name, effectiveMethod, fetch.query_args);
              const queryParams = [];
              for (const paramExprJs of paramsJs) {
                queryParams.push(evalAmanaExpression(paramExprJs, req, currentUser, renderVars));
              }

              let result;
              if (fetch.query_method === 'find') {
                const row = await this.dbGet(sql, queryParams);
                result = row ? this.readableRow(fetchModel, row, req, currentUser) : null;
              } else if (fetch.query_method === 'count') {
                if (this.modelHasPolicies(fetchModel)) {
                  const rows = await this.dbAll(sql, queryParams);
                  result = this.readableRows(fetchModel, rows, req, currentUser).length;
                } else {
                  const row = await this.dbGet(sql, queryParams);
                  result = row ? row.count : 0;
                }
              } else {
                const rows = await this.dbAll(sql, queryParams);
                result = this.readableRows(fetchModel, rows, req, currentUser);
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
            const currentUser = this.getCurrentPrincipal(req);
            const formScope = this.authScope(req, currentUser);
            const modelLowercase = form.model_name.toLowerCase();
            const action = form.action.toLowerCase();

            if (r.guard) {
              if (!currentUser) {
                return res.redirect(r.guard.unauth_path);
              }
              const allowed = Boolean(evalAmanaExpression(r.guard.cond_expr, req, currentUser, formScope));
              if (!allowed) {
                return res.redirect(r.guard.deny_path);
              }
            }

            const model = this.findModelByName(form.model_name);
            if (!model) {
              throw new Error(`Form action references unknown model '${form.model_name}'.`);
            }
            const modelTableSql = quoteSqlIdentifier(model.table_name);
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
              const value = evalAmanaExpression(expr, req, currentUser, formScope);
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
              const value = evalAmanaExpression(expr, req, currentUser, formScope);
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
              const existing = await this.dbGet(`SELECT * FROM ${modelTableSql} WHERE "id" = ? LIMIT 1`, [req.body.id]);
              if (!existing) {
                return res.status(404).send('Record not found.');
              }
              const submitted = {};
              for (const f of updateFields) {
                if (f !== 'id') submitted[f] = readFormValue(f);
              }
              this.ensureFieldsAllowed(model, 'update', updateFields.filter(f => f !== 'id'), req, currentUser, existing, submitted);
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
              const existing = await this.dbGet(`SELECT * FROM ${modelTableSql} WHERE "id" = ? LIMIT 1`, [req.body.id]);
              if (!existing) {
                return res.status(404).send('Record not found.');
              }
              this.ensurePermission(model, 'delete', req, currentUser, existing);
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
              const submitted = {};
              for (const f of insertFields) {
                submitted[f] = readFormValue(f);
              }
              this.ensureFieldsAllowed(model, action === 'register' ? 'register' : 'create', insertFields, req, currentUser, null, submitted);
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
            return res.status(err.statusCode || (String(err.message || '').startsWith('Field ') ? 400 : 500)).send(err.message || 'Form submission failed.');
          }
        });
      }
    }
  }
}

module.exports = AmanaEngine;
"#
}
