// src/parser/css.rs

pub(crate) fn normalize_css_value(value: &str) -> String {
    let mut value_str = value.trim();
    if value_str.starts_with('"') && value_str.ends_with('"') && value_str.len() >= 2 {
        value_str = &value_str[1..value_str.len() - 1];
    }
    let units = ["px", "rem", "em", "vh", "vw", "fr", "ms", "s"];
    let chars: Vec<char> = value_str.chars().collect();
    let mut normalized = String::new();

    let mut idx = 0;
    while idx < chars.len() {
        let ch = chars[idx];
        if ch == ' ' {
            let prev = idx.checked_sub(1).and_then(|i| chars.get(i)).copied();
            let next_is_unit = if prev.is_some_and(|c| c.is_ascii_digit() || c == '.') {
                let mut found_unit = false;
                for &unit in &units {
                    let unit_len = unit.chars().count();
                    if idx + 1 + unit_len <= chars.len() {
                        let matches_unit = chars[idx + 1..idx + 1 + unit_len]
                            .iter()
                            .zip(unit.chars())
                            .all(|(&c1, c2)| c1 == c2);
                        if matches_unit {
                            let after_idx = idx + 1 + unit_len;
                            let is_word_boundary = after_idx >= chars.len()
                                || !chars[after_idx].is_ascii_alphanumeric();
                            if is_word_boundary {
                                found_unit = true;
                                break;
                            }
                        }
                    }
                }
                found_unit
            } else {
                false
            };

            if next_is_unit {
                idx += 1;
                continue;
            }

            let next_char = chars.get(idx + 1).copied();
            if next_char == Some('%') {
                idx += 1;
                continue;
            }
        }
        normalized.push(ch);
        idx += 1;
    }

    normalized = normalized
        .replace("( ", "(")
        .replace(" )", ")")
        .replace(" ,", ",");

    let mut final_res = String::new();
    let final_chars: Vec<char> = normalized.chars().collect();
    let mut i = 0;
    while i < final_chars.len() {
        let c = final_chars[i];
        if c == ' ' {
            final_res.push(' ');
            while i + 1 < final_chars.len() && final_chars[i + 1] == ' ' {
                i += 1;
            }
        } else {
            final_res.push(c);
        }
        i += 1;
    }

    final_res
}

pub(crate) fn ensure_safe_css_value(value: &str) -> Result<(), String> {
    let lower = value.to_lowercase();
    let blocked = ["javascript:", "expression(", "<script", "</", "behavior:"];
    if blocked.iter().any(|needle| lower.contains(needle)) {
        return Err(format!("Unsafe CSS value rejected: '{}'", value));
    }
    Ok(())
}

pub(crate) fn spacing_token(value: &str) -> Option<&'static str> {
    match value {
        "none" | "0" => Some("0"),
        "xs" => Some("var(--space-xs)"),
        "sm" => Some("var(--space-sm)"),
        "small" => Some("var(--padding-small)"),
        "md" => Some("var(--space-md)"),
        "medium" => Some("var(--padding-medium)"),
        "lg" => Some("var(--space-lg)"),
        "large" => Some("var(--padding-large)"),
        "xl" => Some("var(--space-xl)"),
        "2xl" | "xxl" => Some("var(--space-2xl)"),
        "3xl" => Some("var(--space-3xl)"),
        "4xl" => Some("var(--space-4xl)"),
        _ => None,
    }
}

pub(crate) fn size_token(value: &str) -> Option<&'static str> {
    match value {
        "full" => Some("100%"),
        "screen" => Some("100vh"),
        "fit" => Some("fit-content"),
        "min" => Some("min-content"),
        "max" => Some("max-content"),
        "content" => Some("var(--content-width)"),
        "readable" => Some("var(--readable-width)"),
        "wide" => Some("var(--wide-width)"),
        "fluid-xs" => Some("clamp(0.75rem, 1.4vw, 0.9rem)"),
        "fluid-sm" => Some("clamp(0.875rem, 1.6vw, 1rem)"),
        "fluid-md" => Some("clamp(1rem, 1.8vw, 1.15rem)"),
        "fluid-lg" => Some("clamp(1.125rem, 2.2vw, 1.35rem)"),
        "fluid-xl" => Some("clamp(1.5rem, 4vw, 2.4rem)"),
        "fluid-2xl" => Some("clamp(2rem, 6vw, 4rem)"),
        "fluid-3xl" => Some("clamp(2.6rem, 8vw, 6rem)"),
        _ => None,
    }
}

pub(crate) fn color_token(value: &str) -> Option<&'static str> {
    match value {
        "primary" => Some("var(--color-primary)"),
        "primary-soft" => Some("var(--color-primary-soft)"),
        "accent" => Some("var(--color-accent)"),
        "success" => Some("var(--color-success)"),
        "warning" => Some("var(--color-warning)"),
        "danger" => Some("var(--color-danger)"),
        "surface" => Some("var(--bg-primary)"),
        "surface-muted" => Some("var(--surface-muted)"),
        "surface-elevated" => Some("var(--surface-elevated)"),
        "ink" => Some("var(--text-primary)"),
        "subtle" => Some("var(--text-secondary)"),
        "canvas-soft" => Some("var(--canvas-soft)"),
        "custom-primary" => Some("var(--custom-primary, var(--color-primary))"),
        "custom-accent" => Some("var(--custom-accent, var(--color-accent))"),
        "custom-bg" => Some("var(--custom-bg, var(--bg-secondary))"),
        "custom-text" => Some("var(--custom-text, var(--text-primary))"),
        "canvas" => Some("var(--bg-secondary)"),
        "text" => Some("var(--text-primary)"),
        "muted" | "secondary" => Some("var(--text-secondary)"),
        "border" => Some("var(--border-color)"),
        "indigo" => Some("#4f46e5"),
        "cyan" => Some("#06b6d4"),
        "violet" => Some("#7c3aed"),
        "emerald" => Some("#059669"),
        "rose" => Some("#e11d48"),
        "slate" => Some("#334155"),
        _ => None,
    }
}

pub(crate) fn is_pascal_case_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
}

pub(crate) fn compile_responsive_columns(value: &str) -> String {
    let mut parts = value.split_whitespace();
    let _ = parts.next();
    let min = parts.next().unwrap_or("16rem");
    format!(
        "display: grid; grid-template-columns: repeat(auto-fit, minmax({}, 1fr));",
        min
    )
}

pub(crate) fn compile_hover_rule(selector: &str, value: &str) -> Option<String> {
    let val = normalize_css_value(value);
    let body = match val.as_str() {
        "lift" => "transform: translateY(-4px); box-shadow: var(--shadow-floating);",
        "glow" => "box-shadow: var(--glow-primary);",
        "scale" => "transform: scale(1.02);",
        "lift-glow" => {
            "transform: translateY(-5px); box-shadow: var(--shadow-floating), var(--glow-primary);"
        }
        _ => return None,
    };
    Some(format!("{}:hover {{\n  {}\n}}\n", selector, body))
}

pub(crate) fn compile_position_token(value: &str) -> String {
    match value {
        "sticky" => {
            "position: sticky; top: var(--sticky-top, 0); z-index: var(--layer-sticky);".to_string()
        }
        "fixed" => "position: fixed; z-index: var(--layer-overlay);".to_string(),
        "absolute" => "position: absolute;".to_string(),
        "relative" => "position: relative;".to_string(),
        "static" => "position: static;".to_string(),
        _ => format!("position: {};", value),
    }
}

pub(crate) fn compile_layer_token(value: &str) -> String {
    let z_index = match value {
        "base" => "var(--layer-base)",
        "raised" | "surface" => "var(--layer-raised)",
        "sticky" | "nav" | "navbar" => "var(--layer-sticky)",
        "dropdown" | "popover" => "var(--layer-dropdown)",
        "overlay" | "scrim" => "var(--layer-overlay)",
        "modal" | "dialog" => "var(--layer-modal)",
        "toast" | "top" => "var(--layer-toast)",
        _ => value,
    };
    format!("z-index: {};", z_index)
}

pub(crate) fn compile_direction_token(value: &str) -> String {
    match value {
        "rtl" => "direction: rtl; text-align: start;".to_string(),
        "ltr" => "direction: ltr; text-align: start;".to_string(),
        "auto" => "direction: auto;".to_string(),
        _ => format!("direction: {};", value),
    }
}

pub(crate) fn compile_named_transition(value: &str) -> String {
    let mut parts = value.split_whitespace();
    let name = parts.next().unwrap_or(value);
    let duration = parts.next().unwrap_or("180ms");
    let easing = parts.next().unwrap_or("ease");
    let properties = match name {
        "fade" => "opacity",
        "move" | "slide" | "lift" | "scale" => "transform",
        "surface" => "background, border-color, box-shadow, color",
        "layout" => "grid-template-columns, gap, padding",
        "colors" | "color" => "background, border-color, color",
        "all" => "all",
        _ => return format!("transition: {};", value),
    };
    format!("transition: {} {} {};", properties, duration, easing)
}

pub(crate) fn map_css_value(prop: &str, value: &str) -> String {
    if prop.starts_with("padding")
        || prop.starts_with("margin")
        || matches!(
            prop,
            "gap"
                | "row-gap"
                | "column-gap"
                | "inset"
                | "inset-block"
                | "inset-block-start"
                | "inset-block-end"
                | "inset-inline"
                | "inset-inline-start"
                | "inset-inline-end"
                | "top"
                | "right"
                | "bottom"
                | "left"
        )
    {
        return spacing_token(value).unwrap_or(value).to_string();
    }
    if matches!(
        prop,
        "width"
            | "height"
            | "min-width"
            | "max-width"
            | "min-height"
            | "max-height"
            | "inline-size"
            | "block-size"
            | "min-inline-size"
            | "max-inline-size"
            | "min-block-size"
            | "max-block-size"
    ) {
        return size_token(value).unwrap_or(value).to_string();
    }
    if matches!(
        prop,
        "color" | "background" | "background-color" | "border-color"
    ) {
        return color_token(value).unwrap_or(value).to_string();
    }
    value.to_string()
}

pub(crate) fn compile_css_decl(prop: &str, raw_value: &str) -> Result<String, String> {
    let val = normalize_css_value(raw_value);
    ensure_safe_css_value(&val)?;

    let decl = match prop {
        "layout" => match val.as_str() {
            "row" => "display: flex; flex-direction: row; align-items: center;".to_string(),
            "column" | "stack" => "display: flex; flex-direction: column;".to_string(),
            "grid" => "display: grid;".to_string(),
            "center" => {
                "display: flex; align-items: center; justify-content: center;".to_string()
            }
            "inline" => "display: inline-flex; align-items: center;".to_string(),
            "cluster" => "display: flex; flex-wrap: wrap; align-items: center;".to_string(),
            "split" => "display: grid; grid-template-columns: minmax(0, 1fr) minmax(16rem, 0.85fr); align-items: center;".to_string(),
            _ => format!("display: {};", val),
        },
        "center" => match val.as_str() {
            "both" => "display: flex; align-items: center; justify-content: center;".to_string(),
            "x" => "display: flex; justify-content: center;".to_string(),
            "y" => "display: flex; align-items: center;".to_string(),
            _ => format!("place-items: {};", val),
        },
        "columns" if val.starts_with("responsive") => compile_responsive_columns(&val),
        "columns" => format!("grid-template-columns: repeat({}, minmax(0, 1fr));", val),
        "responsive-columns" => compile_responsive_columns(&val),
        "rows" => format!("grid-template-rows: repeat({}, minmax(0, 1fr));", val),
        "area" => format!("grid-area: {};", val),
        "position" => compile_position_token(&val),
        "layer" | "z" | "z-index" => compile_layer_token(&val),
        "direction" | "dir" => compile_direction_token(&val),
        "radius" | "border-radius" => match val.as_str() {
            "full" => "border-radius: 9999px;".to_string(),
            "pill" => "border-radius: 9999px;".to_string(),
            "xl" => "border-radius: var(--radius-xl);".to_string(),
            "2xl" => "border-radius: var(--radius-2xl);".to_string(),
            "soft" => "border-radius: var(--radius-soft);".to_string(),
            "large" | "lg" => "border-radius: var(--radius-large);".to_string(),
            "medium" | "md" => "border-radius: var(--radius-medium);".to_string(),
            "small" | "sm" => "border-radius: var(--radius-small);".to_string(),
            _ => format!("border-radius: {};", val),
        },
        "shadow" | "box-shadow" => match val.as_str() {
            "sm" => "box-shadow: var(--shadow-sm);".to_string(),
            "md" | "smooth" => "box-shadow: var(--shadow-md);".to_string(),
            "lg" | "large" => "box-shadow: var(--shadow-lg);".to_string(),
            "xl" | "strong" => "box-shadow: var(--shadow-xl);".to_string(),
            "soft" => "box-shadow: var(--shadow-soft);".to_string(),
            "floating" => "box-shadow: var(--shadow-floating);".to_string(),
            "none" => "box-shadow: none;".to_string(),
            _ => format!("box-shadow: {};", val),
        },
        "elevation" => format!("box-shadow: var(--elevation-{});", val),
        "border" => match val.as_str() {
            "smooth" => "border: 1px solid var(--border-color);".to_string(),
            "subtle" => "border: 1px solid var(--border-subtle);".to_string(),
            "none" => "border: none;".to_string(),
            _ => format!("border: {};", val),
        },
        "surface" => match val.as_str() {
            "base" => "background: var(--surface-base); color: var(--text-primary);".to_string(),
            "muted" => "background: var(--surface-muted); color: var(--text-primary);".to_string(),
            "elevated" => "background: var(--surface-elevated); color: var(--text-primary); border: 1px solid var(--border-subtle); box-shadow: var(--shadow-soft);".to_string(),
            "glass" => "background: var(--glass-bg); backdrop-filter: var(--glass-blur); -webkit-backdrop-filter: var(--glass-blur); border: 1px solid var(--glass-border);".to_string(),
            "custom" => "background: var(--custom-bg, var(--surface-elevated)); color: var(--custom-text, var(--text-primary)); border: 1px solid var(--custom-border, var(--border-subtle));".to_string(),
            "outline" => "background: transparent; color: var(--custom-text, var(--text-primary)); border: 1px solid var(--custom-border, var(--border-subtle));".to_string(),
            "flat" => "background: var(--custom-bg, transparent); color: var(--custom-text, var(--text-primary));".to_string(),
            _ => format!("background: {};", map_css_value("background", &val)),
        },
        "background" | "background-color" => match val.as_str() {
            "glass" => "background: var(--glass-bg); backdrop-filter: var(--glass-blur); -webkit-backdrop-filter: var(--glass-blur); border: 1px solid var(--glass-border);".to_string(),
            _ => format!("{}: {};", prop, map_css_value(prop, &val)),
        },
        "gradient" => match val.as_str() {
            "primary" => "background: var(--gradient-primary);".to_string(),
            "accent" => "background: var(--gradient-accent);".to_string(),
            "hero" => "background: var(--gradient-hero);".to_string(),
            "mesh" => "background: var(--gradient-mesh);".to_string(),
            "aurora" => "background: var(--gradient-aurora);".to_string(),
            "spotlight" => "background: var(--gradient-spotlight);".to_string(),
            "custom" => "background: var(--custom-gradient, var(--gradient-primary));".to_string(),
            "brand" => "background: linear-gradient(var(--gradient-angle, 135deg), var(--custom-primary, var(--color-primary)), var(--custom-accent, var(--color-accent)));".to_string(),
            "sunset" => "background: linear-gradient(135deg, #f97316, #db2777);".to_string(),
            "ocean" => "background: linear-gradient(135deg, #0284c7, #06b6d4);".to_string(),
            _ => format!("background: linear-gradient({});", val),
        },
        "glow" => match val.as_str() {
            "primary" => "box-shadow: var(--glow-primary);".to_string(),
            "accent" => "box-shadow: var(--glow-accent);".to_string(),
            "none" => "box-shadow: none;".to_string(),
            _ => format!("box-shadow: 0 0 0 4px {};", map_css_value("color", &val)),
        },
        "hover" => match val.as_str() {
            "lift" => "transition: var(--transition-smooth); will-change: transform, box-shadow;".to_string(),
            "glow" => "transition: var(--transition-smooth); will-change: box-shadow;".to_string(),
            "scale" => "transition: var(--transition-smooth); will-change: transform;".to_string(),
            "lift-glow" => "transition: var(--transition-smooth); will-change: transform, box-shadow;".to_string(),
            _ => format!("transition: {};", val),
        },
        "color" => format!("color: {};", map_css_value(prop, &val)),
        "text" | "ink" => format!("color: {};", map_css_value("color", &val)),
        "fill" => format!("background: {};", map_css_value("background", &val)),
        "stroke" | "outline" => format!("border-color: {};", map_css_value("border-color", &val)),
        "opacity" => format!("opacity: {};", val),
        "blur" => format!("backdrop-filter: blur({}); -webkit-backdrop-filter: blur({});", val, val),
        "blend" => format!("mix-blend-mode: {};", val),
        "shape" => match val.as_str() {
            "circle" => "border-radius: 9999px; aspect-ratio: 1;".to_string(),
            "pill" => "border-radius: 9999px;".to_string(),
            "squircle" => "border-radius: 28% 22% 30% 20%;".to_string(),
            "ticket" => "border-radius: var(--radius-soft); clip-path: polygon(0 0, 100% 0, 100% calc(100% - 1rem), calc(100% - 1rem) 100%, 0 100%);".to_string(),
            "diagonal" => "clip-path: polygon(0 0, 100% 0, 96% 100%, 0 92%);".to_string(),
            _ => format!("clip-path: {};", val),
        },
        "font" => match val.as_str() {
            "body" => "font: var(--font-body);".to_string(),
            "heading" => "font: var(--font-heading);".to_string(),
            "mono" => "font-family: var(--font-mono);".to_string(),
            _ => format!("font: {};", val),
        },
        "size" | "font-size" => match val.as_str() {
            "xs" => "font-size: var(--text-xs);".to_string(),
            "sm" | "small" => "font-size: var(--text-sm);".to_string(),
            "md" | "base" => "font-size: var(--text-md);".to_string(),
            "lg" | "large" => "font-size: var(--text-lg);".to_string(),
            "xl" => "font-size: var(--text-xl);".to_string(),
            "2xl" => "font-size: var(--text-2xl);".to_string(),
            "3xl" => "font-size: var(--text-3xl);".to_string(),
            "fluid-xs" | "fluid-sm" | "fluid-md" | "fluid-lg" | "fluid-xl" | "fluid-2xl" | "fluid-3xl" => {
                format!(
                    "font-size: {};",
                    size_token(&val).unwrap_or(val.as_str())
                )
            }
            _ => format!("font-size: {};", val),
        },
        "weight" => format!("font-weight: {};", val),
        "leading" => format!("line-height: {};", val),
        "tracking" => format!("letter-spacing: {};", val),
        "transition" => match val.as_str() {
            "smooth" => "transition: var(--transition-smooth);".to_string(),
            "fast" => "transition: var(--transition-fast);".to_string(),
            "none" => "transition: none;".to_string(),
            _ => compile_named_transition(&val),
        },
        "scroll" => match val.as_str() {
            "x" => "overflow-x: auto; overflow-y: hidden;".to_string(),
            "y" => "overflow-y: auto; overflow-x: hidden;".to_string(),
            "both" => "overflow: auto;".to_string(),
            _ => format!("overflow: {};", val),
        },
        "hide" => match val.as_str() {
            "mobile" => "@media (max-width: 640px) { display: none; }".to_string(),
            "desktop" => "@media (min-width: 641px) { display: none; }".to_string(),
            _ => format!("display: {};", val),
        },
        _ => format!("{}: {};", prop, map_css_value(prop, &val)),
    };
    Ok(decl)
}
