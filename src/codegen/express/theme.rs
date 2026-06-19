// src/codegen/express/theme.rs
use crate::ast::TokenConfigBlock;
use crate::semantic::ir::ThemeIR;
use std::collections::HashMap;

pub(super) fn theme_direction(theme: Option<&ThemeIR>) -> &str {
    if let Some(t) = theme {
        for (key, val) in &t.settings {
            if key == "direction" {
                return val;
            }
        }
    }
    "ltr"
}

pub(super) fn theme_language(theme: Option<&ThemeIR>) -> &str {
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
    if !text
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || ".,#%()+-/*".contains(c))
    {
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
        families.push(format!("family={}:wght@400;500;600;700;800;900", encoded));
    }
    families.join("&")
}

fn theme_color(value: &str, fallback_name: &str) -> (String, String, String) {
    let fallback = named_color_scale(fallback_name).unwrap_or(("#4f46e5", "#eef2ff", "#312e81"));
    if let Some(named) = named_color_scale(value) {
        return (
            named.0.to_string(),
            named.1.to_string(),
            named.2.to_string(),
        );
    }
    let base = safe_css_literal(value, fallback.0);
    let mix1 = format!("color-mix(in srgb, {} 16%, transparent)", base);
    let mix2 = format!("color-mix(in srgb, {} 58%, #020617)", base);
    (base, mix1, mix2)
}

fn css_token_name(category: &str, name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        return String::new();
    }
    format!("--{}-{}", category, out)
}

fn token_css(tokens: Option<&TokenConfigBlock>) -> String {
    let Some(tokens) = tokens else {
        return String::new();
    };

    let mut lines = Vec::new();
    for (name, value) in &tokens.colors {
        let key = css_token_name("color", name);
        let value = safe_css_literal(value, "");
        if !key.is_empty() && !value.is_empty() {
            lines.push(format!("      {}: {};", key, value));
        }
    }
    for (name, value) in &tokens.spacing {
        let key = css_token_name("space", name);
        let value = safe_css_literal(value, "");
        if !key.is_empty() && !value.is_empty() {
            lines.push(format!("      {}: {};", key, value));
        }
    }
    for (name, value) in &tokens.radius {
        let key = css_token_name("radius", name);
        let value = safe_css_literal(value, "");
        if !key.is_empty() && !value.is_empty() {
            lines.push(format!("      {}: {};", key, value));
        }
    }
    for (name, value) in &tokens.shadows {
        let key = css_token_name("shadow", name);
        if !key.is_empty() {
            lines.push(format!(
                "      {}: {};",
                key,
                safe_css_literal(value, "none")
            ));
        }
    }

    if lines.is_empty() {
        String::new()
    } else {
        format!("    :root {{\n{}\n    }}\n", lines.join("\n"))
    }
}

pub(super) fn theme_css(theme: Option<&ThemeIR>, tokens: Option<&TokenConfigBlock>) -> String {
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

    let dark = settings
        .get("mode")
        .map(|&v| v == "dark" || v == "night")
        .unwrap_or(false);
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

    let surface_glass = settings
        .get("surface")
        .map(|&v| v == "glass")
        .unwrap_or(false);

    let canvas_fallback = if dark { "#020617" } else { "#f8fafc" };
    let canvas = safe_css_literal(
        settings
            .get("canvas")
            .or(settings.get("background"))
            .or(settings.get("bg"))
            .copied()
            .unwrap_or(""),
        canvas_fallback,
    );

    let base_fallback = if dark { "#0f172a" } else { "#ffffff" };
    let base = safe_css_literal(
        settings
            .get("base")
            .or(settings.get("surface_base"))
            .copied()
            .unwrap_or(""),
        base_fallback,
    );

    let muted_fallback = if dark { "#111827" } else { "#f8fafc" };
    let muted = safe_css_literal(
        settings
            .get("muted_surface")
            .or(settings.get("surface_muted"))
            .copied()
            .unwrap_or(""),
        muted_fallback,
    );

    let elevated_fallback = if surface_glass {
        if dark {
            "rgba(15,23,42,0.74)"
        } else {
            "rgba(255,255,255,0.72)"
        }
    } else {
        if dark { "#1f2937" } else { "#ffffff" }
    };
    let elevated = safe_css_literal(
        settings
            .get("elevated")
            .or(settings.get("surface_elevated"))
            .copied()
            .unwrap_or(""),
        elevated_fallback,
    );

    let text_fallback = if dark { "#f8fafc" } else { "#0f172a" };
    let text = safe_css_literal(
        settings
            .get("text")
            .or(settings.get("ink"))
            .copied()
            .unwrap_or(""),
        text_fallback,
    );

    let text_muted_fallback = if dark { "#cbd5e1" } else { "#475569" };
    let text_muted = safe_css_literal(
        settings
            .get("muted")
            .or(settings.get("subtle"))
            .copied()
            .unwrap_or(""),
        text_muted_fallback,
    );

    let border_fallback = if dark {
        "rgba(148,163,184,0.22)"
    } else {
        "rgba(15,23,42,0.10)"
    };
    let border = safe_css_literal(
        settings.get("border").copied().unwrap_or(""),
        border_fallback,
    );

    let glass_bg_fallback = if dark {
        "rgba(15,23,42,0.66)"
    } else {
        "rgba(255,255,255,0.58)"
    };
    let glass_bg = safe_css_literal(
        settings
            .get("glass")
            .or(settings.get("glass_bg"))
            .copied()
            .unwrap_or(""),
        glass_bg_fallback,
    );

    let glass_border_fallback = if dark {
        "rgba(148,163,184,0.20)"
    } else {
        "rgba(255,255,255,0.38)"
    };
    let glass_border = safe_css_literal(
        settings.get("glass_border").copied().unwrap_or(""),
        glass_border_fallback,
    );

    let gradient_primary_fallback = format!("linear-gradient(135deg, {}, {})", primary.0, accent.0);
    let gradient_primary = safe_css_literal(
        settings
            .get("gradient_primary")
            .or(settings.get("gradient"))
            .copied()
            .unwrap_or(""),
        &gradient_primary_fallback,
    );

    let gradient_accent_fallback = format!("linear-gradient(135deg, {}, {})", accent.0, primary.0);
    let gradient_accent = safe_css_literal(
        settings.get("gradient_accent").copied().unwrap_or(""),
        &gradient_accent_fallback,
    );

    let gradient_hero_fallback = format!(
        "radial-gradient(circle at top right, {}, transparent 30%), linear-gradient(135deg, {}, {})",
        primary.1,
        if dark { "#0f172a" } else { "#ffffff" },
        accent.1
    );
    let gradient_hero = safe_css_literal(
        settings.get("gradient_hero").copied().unwrap_or(""),
        &gradient_hero_fallback,
    );

    let radius_2xl_fallback = radius.4.to_string();
    let radius_2xl = safe_css_literal(
        settings.get("radius_2xl").copied().unwrap_or(""),
        &radius_2xl_fallback,
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

    let gradient_mesh_fallback = format!(
        "radial-gradient(circle at 10% 20%, {}, transparent 34%), radial-gradient(circle at 80% 0%, {}, transparent 38%), {}",
        primary.1, accent.1, base
    );
    let gradient_mesh = safe_css_literal(
        settings.get("gradient_mesh").copied().unwrap_or(""),
        &gradient_mesh_fallback,
    );

    let gradient_aurora_fallback = format!(
        "radial-gradient(circle at 15% 20%, {}, transparent 30%), radial-gradient(circle at 80% 20%, {}, transparent 35%), {}",
        primary.1, accent.1, canvas
    );
    let gradient_aurora = safe_css_literal(
        settings.get("gradient_aurora").copied().unwrap_or(""),
        &gradient_aurora_fallback,
    );

    let gradient_spotlight_fallback = format!(
        "radial-gradient(circle at 50% 0%, {}, transparent 48%), {}",
        primary.1, base
    );
    let gradient_spotlight = safe_css_literal(
        settings.get("gradient_spotlight").copied().unwrap_or(""),
        &gradient_spotlight_fallback,
    );

    let success_color = safe_css_literal(settings.get("success").copied().unwrap_or(""), "#16a34a");
    let warning_color = safe_css_literal(settings.get("warning").copied().unwrap_or(""), "#ca8a04");
    let danger_color = safe_css_literal(settings.get("danger").copied().unwrap_or(""), "#dc2626");

    let glow_primary = format!("0 0 0 4px {}, 0 18px 40px -24px {}", primary.1, primary.0);
    let glow_accent = format!("0 0 0 4px {}, 0 18px 40px -24px {}", accent.1, accent.0);

    let mut css = format!(
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
    );
    css.push_str(&token_css(tokens));
    css
}
