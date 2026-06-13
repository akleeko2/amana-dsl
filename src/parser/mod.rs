// src/parser/mod.rs
use crate::ast::*;
use crate::lexer::{Token, TokenKind};

/// Parser structure responsible for converting a sequence of Lexer tokens into Amana AST nodes.
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

fn normalize_css_value(value: &str) -> String {
    let units = ["px", "rem", "em", "vh", "vw", "fr", "ms", "s"];
    let chars: Vec<char> = value.trim().chars().collect();
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
                        let matches_unit = chars[idx + 1 .. idx + 1 + unit_len]
                            .iter()
                            .zip(unit.chars())
                            .all(|(&c1, c2)| c1 == c2);
                        if matches_unit {
                            let after_idx = idx + 1 + unit_len;
                            let is_word_boundary = after_idx >= chars.len() || !chars[after_idx].is_ascii_alphanumeric();
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

fn ensure_safe_css_value(value: &str) -> Result<(), String> {
    let lower = value.to_lowercase();
    let blocked = ["javascript:", "expression(", "<script", "</", "behavior:"];
    if blocked.iter().any(|needle| lower.contains(needle)) {
        return Err(format!("Unsafe CSS value rejected: '{}'", value));
    }
    Ok(())
}

fn spacing_token(value: &str) -> Option<&'static str> {
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

fn size_token(value: &str) -> Option<&'static str> {
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

fn color_token(value: &str) -> Option<&'static str> {
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

fn is_pascal_case_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
}

fn compile_responsive_columns(value: &str) -> String {
    let mut parts = value.split_whitespace();
    let _ = parts.next();
    let min = parts.next().unwrap_or("16rem");
    format!(
        "display: grid; grid-template-columns: repeat(auto-fit, minmax({}, 1fr));",
        min
    )
}

fn compile_hover_rule(selector: &str, value: &str) -> Option<String> {
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

fn compile_position_token(value: &str) -> String {
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

fn compile_layer_token(value: &str) -> String {
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

fn compile_direction_token(value: &str) -> String {
    match value {
        "rtl" => "direction: rtl; text-align: start;".to_string(),
        "ltr" => "direction: ltr; text-align: start;".to_string(),
        "auto" => "direction: auto;".to_string(),
        _ => format!("direction: {};", value),
    }
}

fn compile_named_transition(value: &str) -> String {
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

fn map_css_value(prop: &str, value: &str) -> String {
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

fn compile_css_decl(prop: &str, raw_value: &str) -> Result<String, String> {
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

impl Parser {
    /// Constructs a new Parser instance utilizing the provided token stream.
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    /// Parses the entire token stream, returning a vector of top-level AmanaNode declarations.
    pub fn parse(&mut self) -> Result<Vec<AmanaNode>, String> {
        let mut nodes = Vec::new();
        while self.position < self.tokens.len() {
            match self.peek_kind() {
                Some(TokenKind::App) => nodes.push(AmanaNode::App(self.parse_app()?)),
                Some(TokenKind::Identifier(ref id)) if id == "theme" => {
                    nodes.push(AmanaNode::Theme(self.parse_theme()?))
                }
                Some(TokenKind::Identifier(ref id)) if id == "seed" => {
                    nodes.push(AmanaNode::Seed(self.parse_seed()?))
                }
                Some(TokenKind::Model) => nodes.push(AmanaNode::Model(self.parse_model()?)),
                Some(TokenKind::Route) => nodes.push(AmanaNode::Route(self.parse_route()?)),
                Some(TokenKind::View) => nodes.push(AmanaNode::View(self.parse_view()?)),
                Some(TokenKind::Component) => {
                    nodes.push(AmanaNode::Component(self.parse_component()?))
                }
                Some(TokenKind::Variant) => {
                    nodes.push(AmanaNode::Variant(self.parse_variant_node()?))
                }
                Some(TokenKind::Tokens) => {
                    nodes.push(AmanaNode::Tokens(self.parse_tokens_decl()?))
                }
                Some(TokenKind::NewLine) => {
                    self.advance();
                }
                Some(_) => {
                    return Err(format!(
                        "Unexpected token {:?} at line {}",
                        self.peek_kind(),
                        self.peek_line()
                    ));
                }
                None => break,
            }
        }
        Ok(nodes)
    }

    fn parse_theme(&mut self) -> Result<ThemeDecl, String> {
        self.expect_identifier_with_value("theme")?;
        self.expect(TokenKind::Colon)?;
        self.consume_newlines();
        self.expect(TokenKind::Indent)?;
        let mut settings = Vec::new();
        while !self.check(TokenKind::Dedent) {
            let key = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            let value = self.parse_simple_setting_value()?;
            settings.push((key, value));
            self.consume_newlines();
        }
        self.expect(TokenKind::Dedent)?;
        Ok(ThemeDecl { settings })
    }

    fn parse_seed(&mut self) -> Result<SeedDecl, String> {
        self.expect_identifier_with_value("seed")?;
        let model_name = self.expect_identifier()?;
        self.expect(TokenKind::Colon)?;
        self.consume_newlines();
        self.expect(TokenKind::Indent)?;

        let mut rows: Vec<Vec<(String, Expression)>> = Vec::new();
        let mut single_row = Vec::new();
        while !self.check(TokenKind::Dedent) {
            let key = self.expect_identifier()?;
            if key == "row" {
                self.expect(TokenKind::Colon)?;
                self.consume_newlines();
                self.expect(TokenKind::Indent)?;
                let mut row = Vec::new();
                while !self.check(TokenKind::Dedent) {
                    let field = self.expect_identifier()?;
                    self.expect(TokenKind::Colon)?;
                    let value = self.parse_expression(1)?;
                    row.push((field, value));
                    self.consume_newlines();
                }
                self.expect(TokenKind::Dedent)?;
                rows.push(row);
            } else {
                self.expect(TokenKind::Colon)?;
                let value = self.parse_expression(1)?;
                single_row.push((key, value));
            }
            self.consume_newlines();
        }
        self.expect(TokenKind::Dedent)?;
        if !single_row.is_empty() {
            rows.insert(0, single_row);
        }
        Ok(SeedDecl { model_name, rows })
    }

    fn parse_simple_setting_value(&mut self) -> Result<String, String> {
        let value = match self.peek_kind() {
            Some(TokenKind::Identifier(value)) => {
                self.advance();
                value
            }
            Some(TokenKind::Str) => {
                self.advance();
                "str".to_string()
            }
            Some(TokenKind::Int) => {
                self.advance();
                "int".to_string()
            }
            Some(TokenKind::Float) => {
                self.advance();
                "float".to_string()
            }
            Some(TokenKind::Bool) => {
                self.advance();
                "bool".to_string()
            }
            Some(TokenKind::Email) => {
                self.advance();
                "email".to_string()
            }
            Some(TokenKind::Password) => {
                self.advance();
                "password".to_string()
            }
            Some(TokenKind::DateTime) => {
                self.advance();
                "datetime".to_string()
            }
            Some(TokenKind::Money) => {
                self.advance();
                "money".to_string()
            }
            Some(TokenKind::StringLiteral(value)) => {
                self.advance();
                value
            }
            Some(TokenKind::Number(value)) => {
                self.advance();
                value.to_string()
            }
            Some(TokenKind::Boolean(value)) => {
                self.advance();
                value.to_string()
            }
            other => {
                return Err(format!(
                    "Expected theme setting value at line {}:{}, found {:?}",
                    self.peek_line(),
                    self.peek_column(),
                    other
                ));
            }
        };
        Ok(value)
    }

    fn parse_app(&mut self) -> Result<AppConfig, String> {
        self.expect(TokenKind::App)?;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::Colon)?;
        self.consume_newlines();
        self.expect(TokenKind::Indent)?;

        let mut title = name.clone();
        let mut auth_model = "User".to_string();
        let mut db_path = "app.db".to_string();
        let mut capabilities = Vec::new();

        while !self.check(TokenKind::Dedent) {
            let key = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            match key.as_str() {
                "title" => {
                    if let Some(TokenKind::StringLiteral(s)) = self.peek_kind() {
                        self.advance();
                        title = s;
                    } else {
                        return Err(format!(
                            "Expected string literal for title at line {}",
                            self.peek_line()
                        ));
                    }
                }
                "auth_model" => {
                    auth_model = self.expect_identifier()?;
                }
                "db_path" => {
                    if let Some(TokenKind::StringLiteral(s)) = self.peek_kind() {
                        self.advance();
                        db_path = s;
                    } else {
                        return Err(format!(
                            "Expected string literal for db_path at line {}",
                            self.peek_line()
                        ));
                    }
                }
                "capabilities" => {
                    // دعم صيغتين: القائمة بأقواس ["time", "auth"] أو القائمة بالشرطة (- time)
                    if self.check(TokenKind::LBracket) {
                        // الصيغة الأولى: capabilities: ["time", "auth"]
                        self.expect(TokenKind::LBracket)?;
                        if !self.check(TokenKind::RBracket) {
                            loop {
                                if let Some(TokenKind::StringLiteral(s)) = self.peek_kind() {
                                    self.advance();
                                    capabilities.push(s);
                                } else {
                                    return Err(format!(
                                        "Expected string literal inside capabilities list at line {}",
                                        self.peek_line()
                                    ));
                                }
                                if self.check(TokenKind::Comma) {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                        }
                        self.expect(TokenKind::RBracket)?;
                    } else {
                        // الصيغة الثانية: capabilities متبوعة بكتلة إزاحة مع شرطات
                        // capabilities:
                        //     - time
                        //     - auth
                        //     - network.outbound
                        self.consume_newlines();
                        self.expect(TokenKind::Indent)?;
                        while !self.check(TokenKind::Dedent) {
                            // تخطي الشرطة (Minus token)
                            if self.check(TokenKind::Minus) {
                                self.advance();
                            }
                            // قراءة اسم القدرة (قد يحتوي على نقاط مثل network.outbound)
                            let mut cap_name = self.expect_identifier()?;
                            // دعم النقاط في أسماء القدرات (مثل network.outbound)
                            while self.check(TokenKind::Dot) {
                                self.advance();
                                cap_name.push('.');
                                cap_name.push_str(&self.expect_identifier()?);
                            }
                            capabilities.push(cap_name);
                            self.consume_newlines();
                        }
                        self.expect(TokenKind::Dedent)?;
                    }
                }
                _ => {
                    self.advance(); // Skip unknown settings
                }
            }
            self.consume_newlines();
        }
        self.expect(TokenKind::Dedent)?;
        Ok(AppConfig {
            name,
            title,
            db_path,
            auth_model,
            capabilities,
        })
    }

    fn parse_model(&mut self) -> Result<ModelDecl, String> {
        self.expect(TokenKind::Model)?;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::Colon)?;
        self.consume_newlines();
        self.expect(TokenKind::Indent)?;

        let mut fields = Vec::new();
        let mut permissions = Vec::new();

        while !self.check(TokenKind::Dedent) {
            if self.check(TokenKind::Permit) {
                permissions.push(self.parse_permission()?);
            } else {
                fields.push(self.parse_field()?);
            }
            self.consume_newlines();
        }
        self.expect(TokenKind::Dedent)?;
        Ok(ModelDecl {
            name,
            fields,
            permissions,
        })
    }

    fn parse_field(&mut self) -> Result<ModelField, String> {
        let name = self.expect_identifier()?;
        self.expect(TokenKind::Colon)?;

        let data_type_token = self
            .peek_kind()
            .ok_or_else(|| format!("Expected data type at line {}", self.peek_line()))?;
        let data_type = match data_type_token {
            TokenKind::Str => {
                self.advance();
                DataType::Str
            }
            TokenKind::Int => {
                self.advance();
                DataType::Int
            }
            TokenKind::Float => {
                self.advance();
                DataType::Float
            }
            TokenKind::Bool => {
                self.advance();
                DataType::Bool
            }
            TokenKind::Email => {
                self.advance();
                DataType::Email
            }
            TokenKind::Password => {
                self.advance();
                DataType::Password
            }
            TokenKind::DateTime => {
                self.advance();
                DataType::DateTime
            }
            TokenKind::Money => {
                self.advance();
                DataType::Money
            }
            TokenKind::Identifier(id) => {
                self.advance();
                DataType::Custom(id)
            }
            _ => {
                return Err(format!(
                    "Invalid data type {:?} at line {}",
                    data_type_token,
                    self.peek_line()
                ));
            }
        };

        let mut is_primary_key = false;
        let mut is_unique = false;
        let mut is_required = false;
        let mut min_value = None;
        let mut max_value = None;
        let mut default_value = None;
        let mut foreign_key = None;
        let mut on_delete = None;

        // دعم modifiers بدون أقواس مثل: email: email unique
        // قراءة modifiers كـ identifiers منفصلة قبل السطر الجديد
        while !self.check(TokenKind::NewLine)
            && !self.check(TokenKind::Dedent)
            && self.peek_kind().is_some()
        {
            if let Some(TokenKind::Identifier(opt)) = self.peek_kind() {
                self.advance();
                match opt.as_str() {
                    "primary_key" => is_primary_key = true,
                    "unique" => is_unique = true,
                    "required" => is_required = true,
                    "min" => {
                        let n = self.expect_number_value("min")?;
                        min_value = Some(n);
                    }
                    "max" => {
                        let n = self.expect_number_value("max")?;
                        max_value = Some(n);
                    }
                    "default" => {
                        // default يمكن أن يكون متبوعاً بقيمة مباشرة أو بعد مسافة
                        let def_val = self.peek_kind().ok_or_else(|| {
                            format!("Expected default value at line {}", self.peek_line())
                        })?;
                        match def_val {
                            TokenKind::StringLiteral(s) => {
                                self.advance();
                                default_value = Some(s);
                            }
                            TokenKind::Number(n) => {
                                self.advance();
                                default_value = Some(n.to_string());
                            }
                            TokenKind::Boolean(b) => {
                                self.advance();
                                default_value = Some(b.to_string());
                            }
                            _ => {
                                return Err(format!(
                                    "Invalid default value type at line {}",
                                    self.peek_line()
                                ));
                            }
                        }
                    }
                    "foreign_key" => {
                        let target_model = self.expect_identifier()?;
                        self.expect(TokenKind::LParen)?;
                        let target_field = self.expect_identifier()?;
                        self.expect(TokenKind::RParen)?;
                        foreign_key = Some((target_model, target_field));
                    }
                    "on_delete" => {
                        let action = self.expect_identifier()?;
                        // دعم SET NULL كحالة خاصة
                        if action == "SET" {
                            if let Some(TokenKind::Identifier(ref next)) = self.peek_kind() {
                                if next == "NULL" {
                                    self.advance();
                                    on_delete = Some("SET NULL".to_string());
                                } else {
                                    on_delete = Some(action);
                                }
                            } else {
                                on_delete = Some(action);
                            }
                        } else {
                            on_delete = Some(action);
                        }
                    }
                    _ => {
                        // Unknown modifier, ignore or error
                    }
                }
            } else {
                break;
            }
        }

        // دعم القديم أيضاً: modifiers داخل أقواس [unique, default: "value"]
        if self.check(TokenKind::LBracket) {
            self.advance();
            while !self.check(TokenKind::RBracket) {
                let opt = self.expect_identifier()?;
                match opt.as_str() {
                    "primary_key" => is_primary_key = true,
                    "unique" => is_unique = true,
                    "required" => is_required = true,
                    "min" => {
                        self.expect(TokenKind::Colon)?;
                        if let Some(TokenKind::Number(n)) = self.peek_kind() {
                            self.advance();
                            min_value = Some(n);
                        } else {
                            return Err(format!(
                                "Expected number for min option at line {}",
                                self.peek_line()
                            ));
                        }
                    }
                    "max" => {
                        self.expect(TokenKind::Colon)?;
                        if let Some(TokenKind::Number(n)) = self.peek_kind() {
                            self.advance();
                            max_value = Some(n);
                        } else {
                            return Err(format!(
                                "Expected number for max option at line {}",
                                self.peek_line()
                            ));
                        }
                    }
                    "default" => {
                        self.expect(TokenKind::Colon)?;
                        let def_val = self.peek_kind().ok_or_else(|| {
                            format!("Expected default value at line {}", self.peek_line())
                        })?;
                        match def_val {
                            TokenKind::StringLiteral(s) => {
                                self.advance();
                                default_value = Some(s);
                            }
                            TokenKind::Number(n) => {
                                self.advance();
                                default_value = Some(n.to_string());
                            }
                            TokenKind::Boolean(b) => {
                                self.advance();
                                default_value = Some(b.to_string());
                            }
                            _ => {
                                return Err(format!(
                                    "Invalid default value type at line {}",
                                    self.peek_line()
                                ));
                            }
                        }
                    }
                    "foreign_key" => {
                        self.expect(TokenKind::Colon)?;
                        let target_model = self.expect_identifier()?;
                        self.expect(TokenKind::Dot)?;
                        let target_field = self.expect_identifier()?;
                        foreign_key = Some((target_model, target_field));
                    }
                    "on_delete" => {
                        self.expect(TokenKind::Colon)?;
                        let action = self.expect_identifier()?;
                        // دعم SET NULL كحالة خاصة
                        if action == "SET" {
                            if let Some(TokenKind::Identifier(ref next)) = self.peek_kind() {
                                if next == "NULL" {
                                    self.advance();
                                    on_delete = Some("SET NULL".to_string());
                                } else {
                                    on_delete = Some(action);
                                }
                            } else {
                                on_delete = Some(action);
                            }
                        } else {
                            on_delete = Some(action);
                        }
                    }
                    _ => {
                        return Err(format!(
                            "Unknown option '{}' at line {}",
                            opt,
                            self.peek_line()
                        ));
                    }
                }
                if self.check(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBracket)?;
        }

        Ok(ModelField {
            name,
            data_type,
            is_primary_key,
            is_unique,
            is_required,
            min_value,
            max_value,
            default_value,
            foreign_key,
            on_delete,
        })
    }

    fn expect_number_value(&mut self, option: &str) -> Result<f64, String> {
        if self.check(TokenKind::Colon) {
            self.advance();
        }
        if let Some(TokenKind::Number(n)) = self.peek_kind() {
            self.advance();
            Ok(n)
        } else {
            Err(format!(
                "Expected number for {} option at line {}",
                option,
                self.peek_line()
            ))
        }
    }

    fn parse_permission(&mut self) -> Result<PermissionRule, String> {
        self.expect(TokenKind::Permit)?;
        let role = self.expect_identifier()?;
        let action = self.expect_identifier()?;
        let resource = self.expect_identifier()?;
        Ok(PermissionRule {
            role,
            action,
            resource,
        })
    }

    fn parse_route(&mut self) -> Result<RouteDecl, String> {
        self.expect(TokenKind::Route)?;
        let path = self.parse_path()?;

        // دعم صيغتين:
        // 1. الصيغة البسيطة: route /path -> view ViewName
        // 2. الصيغة الموسعة: route /path: [كتلة مع guard و fetch و view]

        if self.check(TokenKind::Arrow) {
            // الصيغة البسيطة
            self.advance();
            self.expect(TokenKind::View)?;
            let view_name = self.expect_identifier()?;
            Ok(RouteDecl {
                path,
                view_name,
                guards: Vec::new(),
                fetches: Vec::new(),
            })
        } else if self.check(TokenKind::Colon) {
            // الصيغة الموسعة
            self.advance();
            self.consume_newlines();
            self.expect(TokenKind::Indent)?;

            let mut guards = Vec::new();
            let mut fetches = Vec::new();
            let mut view_name = String::new();

            while !self.check(TokenKind::Dedent) {
                let keyword = self.peek_kind().ok_or_else(|| {
                    format!(
                        "Expected keyword in route block at line {}",
                        self.peek_line()
                    )
                })?;

                match keyword {
                    TokenKind::Identifier(ref id) if id == "guard" => {
                        guards.push(self.parse_guard_stmt()?);
                    }
                    TokenKind::Fetch => {
                        fetches.push(self.parse_fetch_stmt()?);
                    }
                    TokenKind::View => {
                        self.advance();
                        view_name = self.expect_identifier()?;
                    }
                    _ => {
                        return Err(format!(
                            "Unexpected keyword {:?} in route block at line {}",
                            keyword,
                            self.peek_line()
                        ));
                    }
                }
                self.consume_newlines();
            }

            self.expect(TokenKind::Dedent)?;
            Ok(RouteDecl {
                path,
                view_name,
                guards,
                fetches,
            })
        } else {
            Err(format!(
                "Expected -> or : after route path at line {}",
                self.peek_line()
            ))
        }
    }

    fn parse_guard_stmt(&mut self) -> Result<GuardStmt, String> {
        self.expect_identifier_with_value("guard")?;
        let condition = self.parse_expression(1)?;
        self.expect(TokenKind::Else)?;
        self.expect_identifier_with_value("redirect")?;
        let redirect_path = self.parse_path()?;
        Ok(GuardStmt {
            condition,
            else_action: format!("redirect {}", redirect_path),
        })
    }

    fn expect_identifier_with_value(&mut self, expected: &str) -> Result<(), String> {
        if let Some(TokenKind::Identifier(id)) = self.peek_kind()
            && id == expected
        {
            self.advance();
            return Ok(());
        }
        Err(format!(
            "Expected identifier '{}' at line {}",
            expected,
            self.peek_line()
        ))
    }

    fn parse_path(&mut self) -> Result<String, String> {
        let mut path = String::new();
        if self.check(TokenKind::Slash) {
            self.advance();
            path.push('/');
            while let Some(tk) = self.peek_kind() {
                match tk {
                    TokenKind::Identifier(id) => {
                        self.advance();
                        path.push_str(&id);
                    }
                    TokenKind::Number(n) => {
                        self.advance();
                        path.push_str(&n.to_string());
                    }
                    TokenKind::Minus => {
                        self.advance();
                        path.push('-');
                    }
                    TokenKind::Slash => {
                        self.advance();
                        path.push('/');
                    }
                    TokenKind::LBracket => {
                        self.advance();
                        path.push('[');
                    }
                    TokenKind::RBracket => {
                        self.advance();
                        path.push(']');
                    }
                    _ => break,
                }
            }
            Ok(path)
        } else {
            Err(format!(
                "Expected route path starting with '/' at line {}",
                self.peek_line()
            ))
        }
    }

    fn parse_view(&mut self) -> Result<ViewDecl, String> {
        self.expect(TokenKind::View)?;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::Colon)?;
        self.consume_newlines();
        self.expect(TokenKind::Indent)?;

        let mut protected = None;
        let mut server_fetches = Vec::new();
        let mut client_states = Vec::new();
        let mut render_body = None;
        let mut styles = None;
        let mut canvas = None;

        while !self.check(TokenKind::Dedent) {
            let block_token = self
                .peek_kind()
                .ok_or_else(|| format!("Expected block name at line {}", self.peek_line()))?;
            self.advance();
            self.expect(TokenKind::Colon)?;
            self.consume_newlines();
            self.expect(TokenKind::Indent)?;

            match block_token {
                TokenKind::Protected => {
                    protected = Some(self.parse_protected_block()?);
                }
                TokenKind::Server => {
                    while !self.check(TokenKind::Dedent) {
                        server_fetches.push(self.parse_fetch_stmt()?);
                        self.consume_newlines();
                    }
                }
                TokenKind::Client => {
                    while !self.check(TokenKind::Dedent) {
                        if self.check(TokenKind::State) {
                            client_states.push(self.parse_state_decl()?);
                        }
                        self.consume_newlines();
                    }
                }
                TokenKind::Render => {
                    render_body = Some(self.parse_view_element()?);
                }
                TokenKind::Style => {
                    styles = Some(self.parse_styles()?);
                }
                TokenKind::Identifier(ref id) if id == "canvas" => {
                    canvas = Some(self.parse_design_block_body("canvas".to_string())?);
                }
                _ => {
                    return Err(format!(
                        "Unknown view block {:?} at line {}",
                        block_token,
                        self.peek_line()
                    ));
                }
            }
            self.expect(TokenKind::Dedent)?;
            self.consume_newlines();
        }
        self.expect(TokenKind::Dedent)?;
        Ok(ViewDecl {
            name,
            protected,
            server_fetches,
            client_states,
            render_body,
            styles,
            canvas,
        })
    }

    fn parse_component(&mut self) -> Result<ComponentDecl, String> {
        self.expect(TokenKind::Component)?;
        let name = self.expect_identifier()?;
        
        let mut params = Vec::new();
        if self.check(TokenKind::LParen) {
            params = self.parse_component_params()?;
        }
        
        self.expect(TokenKind::Colon)?;
        self.consume_newlines();
        self.expect(TokenKind::Indent)?;

        let mut render_body = None;
        let mut styles = None;
        let mut variants = Vec::new();

        while !self.check(TokenKind::Dedent) {
            let block_token = self.peek_kind().ok_or_else(|| {
                format!(
                    "Expected block name in component at line {}",
                    self.peek_line()
                )
            })?;
            self.advance();
            self.expect(TokenKind::Colon)?;
            self.consume_newlines();
            self.expect(TokenKind::Indent)?;

            match block_token {
                TokenKind::Render => {
                    render_body = Some(self.parse_view_element()?);
                }
                TokenKind::Style => {
                    styles = Some(self.parse_styles()?);
                }
                TokenKind::Identifier(ref id) if id == "variants" => {
                    while !self.check(TokenKind::Dedent) {
                        let var_name = self.expect_identifier()?;
                        self.expect(TokenKind::Colon)?;
                        self.consume_newlines();
                        self.expect(TokenKind::Indent)?;
                        
                        let mut base_rules = Vec::new();
                        let mut hover_rules = Vec::new();
                        let mut slot_rules = Vec::new();
                        let mut responsive_rules = Vec::new();
                        
                        while !self.check(TokenKind::Dedent) {
                            let sec_name = self.expect_identifier()?;
                            self.expect(TokenKind::Colon)?;
                            self.consume_newlines();
                            self.expect(TokenKind::Indent)?;
                            match sec_name.as_str() {
                                "base" => {
                                    base_rules = self.parse_style_rules_inline()?;
                                }
                                "hover" => {
                                    hover_rules = self.parse_style_rules_inline()?;
                                }
                                "slots" => {
                                    slot_rules = self.parse_slots_rules()?;
                                }
                                "responsive" => {
                                    responsive_rules = self.parse_responsive_rules()?;
                                }
                                _ => return Err(format!("Unknown variant section '{}' inside component variant at line {}", sec_name, self.peek_line())),
                            }
                            self.expect(TokenKind::Dedent)?;
                            self.consume_newlines();
                        }
                        self.expect(TokenKind::Dedent)?;
                        self.consume_newlines();
                        
                        variants.push(VariantDecl {
                            target: name.clone(),
                            name: var_name,
                            base_rules,
                            hover_rules,
                            slot_rules,
                            responsive_rules,
                        });
                    }
                }
                _ => {
                    return Err(format!(
                        "Unknown component block {:?} at line {}",
                        block_token,
                        self.peek_line()
                    ));
                }
            }
            self.expect(TokenKind::Dedent)?;
            self.consume_newlines();
        }
        self.expect(TokenKind::Dedent)?;
        Ok(ComponentDecl {
            name,
            params,
            render_body,
            styles,
            variants,
        })
    }

    fn parse_protected_block(&mut self) -> Result<ProtectedBlock, String> {
        let mut allow_expr = Expression::Null;
        let mut deny_path = String::new();
        let mut unauth_path = String::new();

        while !self.check(TokenKind::Dedent) {
            let key = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            match key.as_str() {
                "allow" => {
                    allow_expr = self.parse_expression(1)?;
                }
                "deny" => {
                    self.expect(TokenKind::Arrow)?;
                    deny_path = self.parse_path()?;
                }
                "unauthenticated" => {
                    self.expect(TokenKind::Arrow)?;
                    unauth_path = self.parse_path()?;
                }
                _ => {
                    return Err(format!(
                        "Unknown protected setting '{}' at line {}",
                        key,
                        self.peek_line()
                    ));
                }
            }
            self.consume_newlines();
        }
        Ok(ProtectedBlock {
            allow_expr,
            deny_path,
            unauth_path,
        })
    }

    fn parse_fetch_stmt(&mut self) -> Result<FetchStmt, String> {
        self.expect(TokenKind::Fetch)?;
        let var_name = self.expect_identifier()?;
        self.expect(TokenKind::Assign)?;
        let model_name = self.expect_identifier()?;
        self.expect(TokenKind::Dot)?;

        let query_method = self.expect_identifier()?;
        self.expect(TokenKind::LParen)?;

        let mut query_args = Vec::new();
        if !self.check(TokenKind::RParen) {
            loop {
                let mut key = None;
                if let Some(TokenKind::Identifier(id)) = self.peek_kind()
                    && self.position + 1 < self.tokens.len()
                    && self.tokens[self.position + 1].kind == TokenKind::Colon
                {
                    self.advance(); // identifier
                    self.advance(); // colon
                    key = Some(id);
                }
                let expr = self.parse_expression(1)?;
                query_args.push((key, expr));
                if self.check(TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(FetchStmt {
            var_name,
            model_name,
            query_method,
            query_args,
        })
    }

    fn parse_state_decl(&mut self) -> Result<StateDecl, String> {
        self.expect(TokenKind::State)?;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::Assign)?;
        let initial_value = self.parse_expression(1)?;

        let mut persist = "memory".to_string();
        if self.check(TokenKind::LBracket) {
            self.advance();
            while !self.check(TokenKind::RBracket) {
                let key = self.expect_identifier()?;
                if key == "persist" {
                    self.expect(TokenKind::Colon)?;
                    persist = self.expect_identifier()?;
                }
                if self.check(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBracket)?;
        }
        Ok(StateDecl {
            name,
            initial_value,
            persist,
        })
    }

    fn is_design_block_name(name: &str) -> bool {
        matches!(
            name,
            "compose"
                | "visual"
                | "type"
                | "motion"
                | "creative"
                | "brand"
                | "art"
                | "responsive"
                | "interaction"
                | "a11y"
                | "component"
                | "tokens"
                | "states"
        )
    }

    fn parse_design_block_body(&mut self, kind: String) -> Result<DesignBlock, String> {
        let settings = self.parse_design_settings(None)?;
        Ok(DesignBlock { kind, settings })
    }

    fn parse_design_settings(
        &mut self,
        prefix: Option<String>,
    ) -> Result<Vec<(String, String)>, String> {
        let mut settings = Vec::new();
        while !self.check(TokenKind::Dedent) {
            self.consume_newlines();
            if self.check(TokenKind::Dedent) {
                break;
            }
            let raw_key = self.expect_identifier()?;
            let key = if let Some(parent) = &prefix {
                format!("{}.{}", parent, raw_key)
            } else {
                raw_key
            };
            self.expect(TokenKind::Colon)?;
            if self.check(TokenKind::NewLine) && self.check_has_block_children() {
                self.consume_newlines();
                self.expect(TokenKind::Indent)?;
                settings.extend(self.parse_design_settings(Some(key))?);
                self.expect(TokenKind::Dedent)?;
            } else {
                let value = self.parse_design_setting_value()?;
                settings.push((key, value));
            }
            self.consume_newlines();
        }
        Ok(settings)
    }

    fn parse_design_setting_value(&mut self) -> Result<String, String> {
        let mut value = String::new();
        let mut previous_was_word = false;
        while !self.check(TokenKind::NewLine)
            && !self.check(TokenKind::Dedent)
            && self.peek_kind().is_some()
        {
            let token = self.peek_kind().unwrap();
            let (piece, is_word) = match token {
                TokenKind::Identifier(text) => {
                    self.advance();
                    (text, true)
                }
                TokenKind::StringLiteral(text) => {
                    self.advance();
                    (text, true)
                }
                TokenKind::Number(n) => {
                    self.advance();
                    (n.to_string(), true)
                }
                TokenKind::Boolean(value) => {
                    self.advance();
                    (value.to_string(), true)
                }
                TokenKind::Str
                | TokenKind::Int
                | TokenKind::Float
                | TokenKind::Bool
                | TokenKind::Email
                | TokenKind::Password
                | TokenKind::DateTime
                | TokenKind::Money
                | TokenKind::State
                | TokenKind::App
                | TokenKind::Model
                | TokenKind::Route
                | TokenKind::View
                | TokenKind::Component
                | TokenKind::Protected
                | TokenKind::Server
                | TokenKind::Client
                | TokenKind::Render
                | TokenKind::Form
                | TokenKind::If
                | TokenKind::Else
                | TokenKind::For
                | TokenKind::In
                | TokenKind::Permit
                | TokenKind::Fetch
                | TokenKind::Style
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::Not => (self.expect_identifier()?, true),
                TokenKind::Dot => {
                    self.advance();
                    (".".to_string(), false)
                }
                TokenKind::Minus => {
                    self.advance();
                    ("-".to_string(), false)
                }
                TokenKind::Slash => {
                    self.advance();
                    ("/".to_string(), false)
                }
                TokenKind::Percent => {
                    self.advance();
                    ("%".to_string(), false)
                }
                TokenKind::Comma => {
                    self.advance();
                    (",".to_string(), false)
                }
                TokenKind::Plus => {
                    self.advance();
                    ("+".to_string(), false)
                }
                TokenKind::Star => {
                    self.advance();
                    ("*".to_string(), false)
                }
                TokenKind::LParen => {
                    self.advance();
                    ("(".to_string(), false)
                }
                TokenKind::RParen => {
                    self.advance();
                    (")".to_string(), false)
                }
                other => {
                    return Err(format!(
                        "Unsupported design grammar value token {:?} at line {}:{}",
                        other,
                        self.peek_line(),
                        self.peek_column()
                    ));
                }
            };
            if !piece.is_empty() {
                if (previous_was_word || value.ends_with(')')) && is_word {
                    value.push(' ');
                }
                value.push_str(&piece);
                previous_was_word = is_word;
            }
        }
        let trimmed = Self::normalize_design_setting_value(value.trim());
        if trimmed.is_empty() {
            Err(format!(
                "Expected design grammar setting value at line {}:{}",
                self.peek_line(),
                self.peek_column()
            ))
        } else {
            Ok(trimmed)
        }
    }

    fn normalize_design_setting_value(value: &str) -> String {
        let chars: Vec<char> = value.chars().collect();
        let mut out = String::new();
        for (idx, ch) in chars.iter().enumerate() {
            if *ch == ' ' {
                let prev = idx.checked_sub(1).and_then(|i| chars.get(i)).copied();
                let next = chars.get(idx + 1).copied();
                if prev.is_some_and(|c| c.is_ascii_digit() || c == '.')
                    && next.is_some_and(|c| c.is_ascii_alphabetic() || c == '%')
                {
                    continue;
                }
            }
            out.push(*ch);
        }
        out
    }

    fn parse_view_element(&mut self) -> Result<ViewElement, String> {
        self.consume_newlines();
        let token = self
            .peek_kind()
            .ok_or_else(|| format!("Expected view element at line {}", self.peek_line()))?;
        match token {
            TokenKind::Component => {
                self.advance();
                self.expect(TokenKind::Colon)?;
                self.consume_newlines();
                self.expect(TokenKind::Indent)?;
                let block = self.parse_design_block_body("component".to_string())?;
                self.expect(TokenKind::Dedent)?;
                Ok(ViewElement::DesignBlock(block))
            }
            TokenKind::Slot => {
                self.advance();
                if self.check(TokenKind::Colon) {
                    self.advance();
                    Ok(ViewElement::SlotDecl { name: "default".to_string(), optional: false })
                } else {
                    let name = self.expect_identifier()?;
                    let mut optional = false;
                    if self.check(TokenKind::Optional) {
                        self.advance();
                        optional = true;
                    } else if self.check_identifier() {
                        let peek_ident = self.peek_token_kind();
                        if let Some(TokenKind::Identifier(ref s)) = peek_ident {
                            if s == "optional" {
                                self.advance();
                                optional = true;
                            }
                        }
                    }
                    Ok(ViewElement::SlotDecl { name, optional })
                }
            }
            TokenKind::Identifier(ref name) if Self::is_design_block_name(name) => {
                let kind = name.clone();
                self.advance();
                self.expect(TokenKind::Colon)?;
                self.consume_newlines();
                self.expect(TokenKind::Indent)?;
                let block = self.parse_design_block_body(kind)?;
                self.expect(TokenKind::Dedent)?;
                Ok(ViewElement::DesignBlock(block))
            }
            TokenKind::If => {
                self.advance();
                let condition = self.parse_expression(1)?;
                self.expect(TokenKind::Colon)?;
                self.consume_newlines();
                self.expect(TokenKind::Indent)?;
                let mut then_branch = Vec::new();
                while !self.check(TokenKind::Dedent) {
                    then_branch.push(self.parse_view_element()?);
                    self.consume_newlines();
                }
                self.expect(TokenKind::Dedent)?;

                let mut else_branch = None;
                self.consume_newlines();
                if self.check(TokenKind::Else) {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    self.consume_newlines();
                    self.expect(TokenKind::Indent)?;
                    let mut eb = Vec::new();
                    while !self.check(TokenKind::Dedent) {
                        eb.push(self.parse_view_element()?);
                        self.consume_newlines();
                    }
                    self.expect(TokenKind::Dedent)?;
                    else_branch = Some(eb);
                }
                Ok(ViewElement::IfBlock {
                    condition,
                    then_branch,
                    else_branch,
                })
            }
            TokenKind::For => {
                self.advance();
                let item_var = self.expect_identifier()?;
                self.expect(TokenKind::In)?;
                let list_var = self.expect_identifier()?;
                self.expect(TokenKind::Colon)?;
                self.consume_newlines();
                self.expect(TokenKind::Indent)?;

                let mut body = Vec::new();
                while !self.check(TokenKind::Dedent) {
                    body.push(self.parse_view_element()?);
                    self.consume_newlines();
                }
                self.expect(TokenKind::Dedent)?;

                Ok(ViewElement::ForEach {
                    item_var,
                    list_expr: Expression::Identifier(list_var),
                    body,
                })
            }
            TokenKind::Form => {
                let next_kind = self.tokens.get(self.position + 1).map(|t| t.kind.clone());
                if next_kind == Some(TokenKind::LBracket) {
                    self.advance();
                    self.expect(TokenKind::LBracket)?;
                    let mut fields = Vec::new();
                    if !self.check(TokenKind::RBracket) {
                        fields.push(self.expect_identifier()?);
                        while self.check(TokenKind::Comma) {
                            self.advance();
                            fields.push(self.expect_identifier()?);
                        }
                    }
                    self.expect(TokenKind::RBracket)?;
                    self.expect(TokenKind::Colon)?;
                    self.consume_newlines();
                    self.expect(TokenKind::Indent)?;

                    let mut connect_action = String::new();
                    let mut redirect_success = String::new();
                    let mut defaults = Vec::new();
                    let mut constraints = Vec::new();
                    let mut ui = None;
                    let mut submit_label = None;
                    let mut field_options = Vec::new();

                    while !self.check(TokenKind::Dedent) {
                        let key = self.expect_identifier()?;
                        match key.as_str() {
                            "connect" => {
                                let m = self.expect_identifier()?;
                                self.expect(TokenKind::Dot)?;
                                let act = self.expect_identifier()?;
                                connect_action = format!("{}.{}", m, act);
                            }
                            "redirect" => {
                                let status = self.expect_identifier()?;
                                self.expect(TokenKind::Arrow)?;
                                let path = self.parse_path()?;
                                if status == "success" {
                                    redirect_success = path;
                                }
                            }
                            "default" => {
                                let field = self.expect_identifier()?;
                                self.expect(TokenKind::Assign)?;
                                let value = self.parse_expression(1)?;
                                defaults.push((field, value));
                            }
                            "where" => {
                                let field = self.expect_identifier()?;
                                self.expect(TokenKind::Assign)?;
                                let value = self.parse_expression(1)?;
                                constraints.push((field, value));
                            }
                            "ui" => {
                                self.expect(TokenKind::Colon)?;
                                ui = Some(self.parse_simple_setting_value()?);
                            }
                            "submit" => {
                                self.expect(TokenKind::Colon)?;
                                submit_label = Some(self.parse_simple_setting_value()?);
                            }
                            "field" => {
                                let name = self.expect_identifier()?;
                                self.expect(TokenKind::Colon)?;
                                self.consume_newlines();
                                self.expect(TokenKind::Indent)?;
                                let mut label = None;
                                let mut placeholder = None;
                                let mut input_type = None;
                                let mut help = None;
                                let mut required = None;
                                while !self.check(TokenKind::Dedent) {
                                    let opt = self.expect_identifier()?;
                                    self.expect(TokenKind::Colon)?;
                                    match opt.as_str() {
                                        "label" => label = Some(self.parse_simple_setting_value()?),
                                        "placeholder" => {
                                            placeholder = Some(self.parse_simple_setting_value()?)
                                        }
                                        "type" => {
                                            input_type = Some(self.parse_simple_setting_value()?)
                                        }
                                        "help" => help = Some(self.parse_simple_setting_value()?),
                                        "required" => {
                                            let value = self.parse_simple_setting_value()?;
                                            required = Some(value == "true" || value == "yes");
                                        }
                                        _ => {
                                            return Err(format!(
                                                "Unknown form field option '{}' at line {}.",
                                                opt,
                                                self.peek_line()
                                            ));
                                        }
                                    }
                                    self.consume_newlines();
                                }
                                self.expect(TokenKind::Dedent)?;
                                field_options.push(FormFieldOptions {
                                    name,
                                    label,
                                    placeholder,
                                    input_type,
                                    help,
                                    required,
                                });
                            }
                            _ => {
                                return Err(format!(
                                    "Unknown form setting '{}' at line {}. Expected connect, redirect, default, where, ui, submit, or field.",
                                    key,
                                    self.peek_line()
                                ));
                            }
                        }
                        self.consume_newlines();
                    }
                    self.expect(TokenKind::Dedent)?;

                    Ok(ViewElement::FormBlock {
                        fields,
                        connect_action,
                        redirect_success,
                        defaults,
                        constraints,
                        ui,
                        submit_label,
                        field_options,
                    })
                } else {
                    self.advance();
                    let tag = "form".to_string();
                    let mut classes = Vec::new();
                    while self.check(TokenKind::Dot) {
                        self.advance();
                        let mut class_name = self.expect_identifier()?;
                        while self.check(TokenKind::Minus) {
                            self.advance();
                            let part = self.expect_identifier()?;
                            class_name = format!("{}-{}", class_name, part);
                        }
                        classes.push(class_name);
                    }

                    let mut attributes = Vec::new();
                    if self.check(TokenKind::LParen) {
                        self.advance();
                        while !self.check(TokenKind::RParen) {
                            let attr_name = self.expect_identifier()?;
                            self.expect(TokenKind::Colon)?;
                            let attr_val = self.parse_expression(1)?;
                            attributes.push((attr_name, attr_val));
                            if self.check(TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::RParen)?;
                    }

                    self.expect(TokenKind::Colon)?;
                    let has_newline = self.check(TokenKind::NewLine);
                    let has_block = if has_newline {
                        self.check_has_block_children()
                    } else {
                        false
                    };

                    if has_newline && !has_block {
                        self.consume_newlines();
                        Ok(ViewElement::Element {
                            tag,
                            classes,
                            attributes,
                            children: vec![],
                        })
                    } else {
                        self.consume_newlines();
                        if self.check(TokenKind::Indent) {
                            let children = self.parse_indented_block(|p| p.parse_view_element())?;
                            Ok(ViewElement::Element {
                                tag,
                                classes,
                                attributes,
                                children,
                            })
                        } else {
                            if self.check(TokenKind::NewLine)
                                || self.check(TokenKind::Dedent)
                                || self.peek_kind().is_none()
                            {
                                Ok(ViewElement::Element {
                                    tag,
                                    classes,
                                    attributes,
                                    children: vec![],
                                })
                            } else {
                                let next_token = self.peek_kind().ok_or_else(|| {
                                    format!("Expected text at line {}", self.peek_line())
                                })?;
                                match next_token {
                                    TokenKind::StringLiteral(s) => {
                                        self.advance();
                                        Ok(ViewElement::Element {
                                            tag,
                                            classes,
                                            attributes,
                                            children: vec![ViewElement::Text(s)],
                                        })
                                    }
                                    _ => {
                                        let expr = self.parse_expression(1)?;
                                        Ok(ViewElement::Element {
                                            tag,
                                            classes,
                                            attributes,
                                            children: vec![ViewElement::FormattedText(vec![expr])],
                                        })
                                    }
                                }
                            }
                        }
                    }
                }
            }
            TokenKind::Identifier(ref orig_tag) => {
                let mut tag = orig_tag.clone();
                let element_line = self.peek_line();
                let element_column = self.peek_column();
                if tag == "ResourceGrid" || tag == "ResourceTable" {
                    let is_grid = tag == "ResourceGrid";
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    let resource_expr = self.parse_expression(1)?;
                    self.expect(TokenKind::RParen)?;
                    self.expect(TokenKind::Colon)?;
                    self.consume_newlines();
                    self.expect(TokenKind::Indent)?;
                    
                    let mut item_component = String::new();
                    let mut item_arg_name = String::new();
                    let mut empty_element = None;
                    let mut loading_element = None;
                    let mut error_element = None;
                    let mut filter_fields = Vec::new();
                    let mut sort_fields = Vec::new();
                    
                    while !self.check(TokenKind::Dedent) {
                        let key = self.expect_identifier()?;
                        match key.as_str() {
                            "item" => {
                                item_component = self.expect_identifier()?;
                                self.expect(TokenKind::LParen)?;
                                item_arg_name = self.expect_identifier()?;
                                self.expect(TokenKind::RParen)?;
                            }
                            "empty" => {
                                self.expect(TokenKind::Colon)?;
                                self.consume_newlines();
                                self.expect(TokenKind::Indent)?;
                                let mut empty_body = Vec::new();
                                while !self.check(TokenKind::Dedent) {
                                    empty_body.push(self.parse_view_element()?);
                                    self.consume_newlines();
                                }
                                self.expect(TokenKind::Dedent)?;
                                empty_element = Some(empty_body);
                            }
                            "loading" => {
                                self.expect(TokenKind::Colon)?;
                                self.consume_newlines();
                                self.expect(TokenKind::Indent)?;
                                let mut loading_body = Vec::new();
                                while !self.check(TokenKind::Dedent) {
                                    loading_body.push(self.parse_view_element()?);
                                    self.consume_newlines();
                                }
                                self.expect(TokenKind::Dedent)?;
                                loading_element = Some(loading_body);
                            }
                            "error" => {
                                self.expect(TokenKind::Colon)?;
                                self.consume_newlines();
                                self.expect(TokenKind::Indent)?;
                                let mut error_body = Vec::new();
                                while !self.check(TokenKind::Dedent) {
                                    error_body.push(self.parse_view_element()?);
                                    self.consume_newlines();
                                }
                                self.expect(TokenKind::Dedent)?;
                                error_element = Some(error_body);
                            }
                            "filters" => {
                                self.expect(TokenKind::Colon)?;
                                self.consume_newlines();
                                self.expect(TokenKind::Indent)?;
                                while !self.check(TokenKind::Dedent) {
                                    self.expect(TokenKind::Minus)?;
                                    filter_fields.push(self.expect_identifier()?);
                                    self.consume_newlines();
                                }
                                self.expect(TokenKind::Dedent)?;
                            }
                            "sort" => {
                                self.expect(TokenKind::Colon)?;
                                self.consume_newlines();
                                self.expect(TokenKind::Indent)?;
                                while !self.check(TokenKind::Dedent) {
                                    self.expect(TokenKind::Minus)?;
                                    sort_fields.push(self.expect_identifier()?);
                                    self.consume_newlines();
                                }
                                self.expect(TokenKind::Dedent)?;
                            }
                            _ => return Err(format!("Unknown resource element option '{}' at line {}", key, self.peek_line())),
                        }
                        self.consume_newlines();
                    }
                    self.expect(TokenKind::Dedent)?;
                    
                    if is_grid {
                        return Ok(ViewElement::ResourceGrid {
                            resource_expr,
                            item_component,
                            item_arg_name,
                            empty_element,
                            loading_element,
                            error_element,
                            filter_fields,
                            sort_fields,
                        });
                    } else {
                        return Ok(ViewElement::ResourceTable {
                            resource_expr,
                            item_component,
                            item_arg_name,
                            empty_element,
                            loading_element,
                            error_element,
                            filter_fields,
                            sort_fields,
                        });
                    }
                } else {
                    self.advance();
                    while self.check(TokenKind::Minus) {
                        self.advance();
                        let part = self.expect_identifier()?;
                        tag = format!("{}-{}", tag, part);
                    }
                }
                let mut classes = Vec::new();
                while self.check(TokenKind::Dot) {
                    self.advance();
                    let mut class_name = self.expect_identifier()?;
                    while self.check(TokenKind::Minus) {
                        self.advance();
                        let part = self.expect_identifier()?;
                        class_name = format!("{}-{}", class_name, part);
                    }
                    classes.push(class_name);
                }

                let mut attributes = Vec::new();
                let mut has_call_parens = false;
                if self.check(TokenKind::LParen) {
                    has_call_parens = true;
                    self.advance();
                    while !self.check(TokenKind::RParen) {
                        let attr_name = self.expect_identifier()?;
                        self.expect(TokenKind::Colon)?;
                        let attr_val = self.parse_expression(1)?;
                        attributes.push((attr_name, attr_val));
                        if self.check(TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                }

                if !self.check(TokenKind::Colon) {
                    if self.check(TokenKind::NewLine)
                        || self.check(TokenKind::Dedent)
                        || self.peek_kind().is_none()
                    {
                        return Ok(ViewElement::Element {
                            tag,
                            classes,
                            attributes,
                            children: vec![],
                        });
                    }
                    return Err(format!(
                        "Expected ':' after element '{}' at line {}:{}, found {:?}",
                        tag,
                        self.peek_line(),
                        self.peek_column(),
                        self.peek_kind()
                    ));
                }
                self.expect(TokenKind::Colon)?;
                let has_newline = self.check(TokenKind::NewLine);
                let has_block = if has_newline {
                    self.check_has_block_children()
                } else {
                    false
                };

                if has_newline && !has_block {
                    if is_pascal_case_name(&tag) && has_call_parens {
                        return Err(format!(
                            "Component calls without children can be written as {}(). at line {}:{}",
                            tag, element_line, element_column
                        ));
                    }
                    self.consume_newlines();
                    Ok(ViewElement::Element {
                        tag,
                        classes,
                        attributes,
                        children: vec![],
                    })
                } else {
                    self.consume_newlines();
                    if self.check(TokenKind::Indent) {
                        let children = self.parse_indented_block(|p| p.parse_view_element())?;
                        Ok(ViewElement::Element {
                            tag,
                            classes,
                            attributes,
                            children,
                        })
                    } else {
                        if self.check(TokenKind::NewLine)
                            || self.check(TokenKind::Dedent)
                            || self.peek_kind().is_none()
                        {
                            Ok(ViewElement::Element {
                                tag,
                                classes,
                                attributes,
                                children: vec![],
                            })
                        } else {
                            let next_token = self.peek_kind().ok_or_else(|| {
                                format!("Expected text at line {}", self.peek_line())
                            })?;
                            match next_token {
                                TokenKind::StringLiteral(s) => {
                                    self.advance();
                                    Ok(ViewElement::Element {
                                        tag,
                                        classes,
                                        attributes,
                                        children: vec![ViewElement::Text(s)],
                                    })
                                }
                                _ => {
                                    let expr = self.parse_expression(1)?;
                                    Ok(ViewElement::Element {
                                        tag,
                                        classes,
                                        attributes,
                                        children: vec![ViewElement::FormattedText(vec![expr])],
                                    })
                                }
                            }
                        }
                    }
                }
            }
            TokenKind::StringLiteral(s) => {
                self.advance();
                Ok(ViewElement::Text(s))
            }
            _ => {
                let expr = self.parse_expression(1)?;
                Ok(ViewElement::FormattedText(vec![expr]))
            }
        }
    }

    fn parse_styles(&mut self) -> Result<String, String> {
        let mut styles_str = String::new();
        self.consume_newlines();
        while !self.check(TokenKind::Dedent) {
            self.consume_newlines();
            if self.check(TokenKind::Dedent) {
                break;
            }

            // 1. Parse Selector. Collect until terminal Colon.
            let mut selector = String::new();
            let mut prev_was_ident = false;
            while self.peek_kind().is_some() {
                let tk = self.peek_kind().unwrap();
                if tk == TokenKind::Colon {
                    let is_terminal = match self.tokens.get(self.position + 1).map(|t| &t.kind) {
                        Some(TokenKind::NewLine) | Some(TokenKind::Indent) | Some(TokenKind::Dedent) | None => true,
                        _ => false,
                    };
                    if is_terminal {
                        break;
                    }
                }
                if tk == TokenKind::NewLine || tk == TokenKind::Indent || tk == TokenKind::Dedent {
                    break;
                }

                if Self::is_identifier_like_token(&tk) {
                    if prev_was_ident {
                        selector.push(' ');
                    }
                    selector.push_str(&self.expect_identifier()?);
                    prev_was_ident = true;
                    continue;
                }

                self.advance();
                match tk {
                    TokenKind::Dot => {
                        selector.push('.');
                        prev_was_ident = false;
                    }
                    TokenKind::Star => {
                        if prev_was_ident {
                            selector.push(' ');
                        }
                        selector.push('*');
                        prev_was_ident = true;
                    }
                    TokenKind::Minus => {
                        selector.push('-');
                        prev_was_ident = false;
                    }
                    TokenKind::Colon => {
                        selector.push(':');
                        prev_was_ident = false;
                    }
                    TokenKind::Comma => {
                        selector.push_str(", ");
                        if self.check(TokenKind::NewLine) {
                            self.advance();
                        }
                        prev_was_ident = false;
                    }
                    TokenKind::HashColor(ref color) => {
                        if prev_was_ident {
                            selector.push(' ');
                        }
                        selector.push_str(&format!("#{}", color));
                        prev_was_ident = true;
                    }
                    _ => {
                        prev_was_ident = false;
                    }
                }
            }
            self.expect(TokenKind::Colon)?;
            self.consume_newlines();

            styles_str.push_str(&selector);
            styles_str.push_str(" {\n");

            self.expect(TokenKind::Indent)?;
            let mut extra_rules = Vec::new();

            // Parse declarations inside this selector's block.
            while !self.check(TokenKind::Dedent) {
                self.consume_newlines();
                if self.check(TokenKind::Dedent) {
                    break;
                }

                let mut prop = String::new();
                while !self.check(TokenKind::Colon) && self.peek_kind().is_some() {
                    let tk = self.peek_kind().unwrap();
                    if Self::is_identifier_like_token(&tk) {
                        prop.push_str(&self.expect_identifier()?);
                        continue;
                    }
                    match tk {
                        TokenKind::Minus => {
                            self.advance();
                            prop.push('-');
                        }
                        _ => break,
                    }
                }
                if self.check(TokenKind::Colon) {
                    self.advance();
                }

                let mut val = String::new();
                let mut prev_was_word = false;
                while !self.check(TokenKind::NewLine)
                    && !self.check(TokenKind::Dedent)
                    && self.peek_kind().is_some()
                {
                    let tk = self.peek_kind().unwrap();
                    let (token_str, is_word) = match tk {
                        TokenKind::HashColor(ref color) => {
                            self.advance();
                            (format!("#{}", color), true)
                        }
                        _ if Self::is_identifier_like_token(&tk) => {
                            (self.expect_identifier()?, true)
                        }
                        TokenKind::Number(n) => {
                            self.advance();
                            (n.to_string(), true)
                        }
                        TokenKind::StringLiteral(s) => {
                            self.advance();
                            (format!("\"{}\"", s), true)
                        }
                        TokenKind::Dot => {
                            self.advance();
                            (".".to_string(), false)
                        }
                        TokenKind::Colon => {
                            self.advance();
                            (":".to_string(), false)
                        }
                        TokenKind::Comma => {
                            self.advance();
                            (",".to_string(), false)
                        }
                        TokenKind::Percent => {
                            self.advance();
                            ("%".to_string(), false)
                        }
                        TokenKind::LParen => {
                            self.advance();
                            ("(".to_string(), false)
                        }
                        TokenKind::RParen => {
                            self.advance();
                            (")".to_string(), false)
                        }
                        TokenKind::Minus => {
                            self.advance();
                            ("-".to_string(), false)
                        }
                        TokenKind::Slash => {
                            self.advance();
                            ("/".to_string(), false)
                        }
                        TokenKind::Plus => {
                            self.advance();
                            ("+".to_string(), false)
                        }
                        TokenKind::Star => {
                            self.advance();
                            ("*".to_string(), false)
                        }
                        TokenKind::Assign => {
                            self.advance();
                            ("=".to_string(), false)
                        }
                        _ => {
                            self.advance();
                            (String::new(), false)
                        }
                    };

                    if !token_str.is_empty() {
                        if (prev_was_word || val.ends_with(')')) && is_word {
                            val.push(' ');
                        }
                        val.push_str(&token_str);
                        prev_was_word = is_word;
                    }
                }

                let css_decl = compile_css_decl(&prop, &val)?;
                if let Some(rule) = compile_hover_rule(&selector, &val) {
                    extra_rules.push(rule);
                }

                styles_str.push_str("  ");
                styles_str.push_str(&css_decl);
                styles_str.push('\n');

                self.consume_newlines();
            }

            self.expect(TokenKind::Dedent)?;
            styles_str.push_str("}\n");
            for rule in extra_rules {
                styles_str.push_str(&rule);
            }
            self.consume_newlines();
        }

        Ok(styles_str)
    }

    fn is_identifier_like_token(tk: &TokenKind) -> bool {
        matches!(
            tk,
            TokenKind::Identifier(_)
                | TokenKind::Email
                | TokenKind::Password
                | TokenKind::Str
                | TokenKind::Int
                | TokenKind::Float
                | TokenKind::Bool
                | TokenKind::DateTime
                | TokenKind::Money
                | TokenKind::State
                | TokenKind::App
                | TokenKind::Model
                | TokenKind::Route
                | TokenKind::View
                | TokenKind::Component
                | TokenKind::Protected
                | TokenKind::Server
                | TokenKind::Client
                | TokenKind::Render
                | TokenKind::Form
                | TokenKind::If
                | TokenKind::Else
                | TokenKind::For
                | TokenKind::In
                | TokenKind::Permit
                | TokenKind::Fetch
                | TokenKind::Style
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::Not
                | TokenKind::Variant
                | TokenKind::Slot
                | TokenKind::Optional
                | TokenKind::Tokens
        )
    }

    /// Parses expressions utilizing Pratt Parsing algorithm for operator precedence.
    pub fn parse_expression(&mut self, precedence: u8) -> Result<Expression, String> {
        let mut left = self.parse_primary()?;

        while let Some(op) = self.peek_kind() {
            let op_precedence = self.get_precedence(&op);
            if op_precedence < precedence {
                break;
            }

            self.advance();
            left = match op {
                TokenKind::Dot => {
                    let property = self.expect_identifier()?;
                    Expression::MemberAccess {
                        object: Box::new(left),
                        property,
                    }
                }
                TokenKind::LParen => {
                    let args = self.parse_call_args()?;
                    Expression::Call {
                        callee: Box::new(left),
                        args,
                    }
                }
                TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::EqEq
                | TokenKind::Neq
                | TokenKind::Gt
                | TokenKind::Lt
                | TokenKind::Gte
                | TokenKind::Lte
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::Assign => {
                    let right = if op == TokenKind::Assign {
                        self.parse_expression(op_precedence)?
                    } else {
                        self.parse_expression(op_precedence + 1)?
                    };
                    Expression::Binary {
                        left: Box::new(left),
                        op: self.op_to_str(&op),
                        right: Box::new(right),
                    }
                }
                _ => return Err("Unexpected operator in expression".to_string()),
            };
        }
        Ok(left)
    }

    fn peek_kind(&self) -> Option<TokenKind> {
        self.tokens.get(self.position).map(|t| t.kind.clone())
    }

    fn peek_line(&self) -> usize {
        self.tokens.get(self.position).map(|t| t.line).unwrap_or(0)
    }

    fn peek_column(&self) -> usize {
        self.tokens
            .get(self.position)
            .map(|t| t.column)
            .unwrap_or(1)
    }

    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), String> {
        if self.peek_kind() == Some(kind.clone()) {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "Expected token {:?} at line {}:{}, found {:?}",
                kind,
                self.peek_line(),
                self.peek_column(),
                self.peek_kind()
            ))
        }
    }

    fn expect_identifier(&mut self) -> Result<String, String> {
        if let Some(tk) = self.peek_kind() {
            match tk {
                TokenKind::Identifier(id) => {
                    self.advance();
                    Ok(id)
                }
                TokenKind::Email => {
                    self.advance();
                    Ok("email".to_string())
                }
                TokenKind::Password => {
                    self.advance();
                    Ok("password".to_string())
                }
                TokenKind::Str => {
                    self.advance();
                    Ok("str".to_string())
                }
                TokenKind::Int => {
                    self.advance();
                    Ok("int".to_string())
                }
                TokenKind::Float => {
                    self.advance();
                    Ok("float".to_string())
                }
                TokenKind::Bool => {
                    self.advance();
                    Ok("bool".to_string())
                }
                TokenKind::DateTime => {
                    self.advance();
                    Ok("datetime".to_string())
                }
                TokenKind::Money => {
                    self.advance();
                    Ok("money".to_string())
                }
                TokenKind::State => {
                    self.advance();
                    Ok("state".to_string())
                }
                TokenKind::App => {
                    self.advance();
                    Ok("app".to_string())
                }
                TokenKind::Model => {
                    self.advance();
                    Ok("model".to_string())
                }
                TokenKind::Route => {
                    self.advance();
                    Ok("route".to_string())
                }
                TokenKind::View => {
                    self.advance();
                    Ok("view".to_string())
                }
                TokenKind::Component => {
                    self.advance();
                    Ok("component".to_string())
                }
                TokenKind::Protected => {
                    self.advance();
                    Ok("protected".to_string())
                }
                TokenKind::Server => {
                    self.advance();
                    Ok("server".to_string())
                }
                TokenKind::Client => {
                    self.advance();
                    Ok("client".to_string())
                }
                TokenKind::Render => {
                    self.advance();
                    Ok("render".to_string())
                }
                TokenKind::Form => {
                    self.advance();
                    Ok("form".to_string())
                }
                TokenKind::If => {
                    self.advance();
                    Ok("if".to_string())
                }
                TokenKind::Else => {
                    self.advance();
                    Ok("else".to_string())
                }
                TokenKind::For => {
                    self.advance();
                    Ok("for".to_string())
                }
                TokenKind::In => {
                    self.advance();
                    Ok("in".to_string())
                }
                TokenKind::Permit => {
                    self.advance();
                    Ok("permit".to_string())
                }
                TokenKind::Fetch => {
                    self.advance();
                    Ok("fetch".to_string())
                }
                TokenKind::Style => {
                    self.advance();
                    Ok("style".to_string())
                }
                TokenKind::And => {
                    self.advance();
                    Ok("and".to_string())
                }
                TokenKind::Or => {
                    self.advance();
                    Ok("or".to_string())
                }
                TokenKind::Not => {
                    self.advance();
                    Ok("not".to_string())
                }
                TokenKind::Variant => {
                    self.advance();
                    Ok("variant".to_string())
                }
                TokenKind::Slot => {
                    self.advance();
                    Ok("slot".to_string())
                }
                TokenKind::Optional => {
                    self.advance();
                    Ok("optional".to_string())
                }
                TokenKind::Tokens => {
                    self.advance();
                    Ok("tokens".to_string())
                }
                _ => Err(format!("Expected identifier at line {}", self.peek_line())),
            }
        } else {
            Err(format!("Expected identifier at line {}", self.peek_line()))
        }
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek_kind() == Some(kind)
    }

    fn check_has_block_children(&self) -> bool {
        let mut pos = self.position;
        while pos < self.tokens.len() && self.tokens[pos].kind == TokenKind::NewLine {
            pos += 1;
        }
        if pos < self.tokens.len() {
            self.tokens[pos].kind == TokenKind::Indent
        } else {
            false
        }
    }

    fn consume_newlines(&mut self) {
        while self.check(TokenKind::NewLine) {
            self.advance();
        }
    }

    fn parse_indented_block<F, T>(&mut self, parse_fn: F) -> Result<Vec<T>, String>
    where
        F: Fn(&mut Self) -> Result<T, String>,
    {
        self.expect(TokenKind::Indent)?;
        let mut items = Vec::new();
        while !self.check(TokenKind::Dedent) {
            items.push(parse_fn(self)?);
            self.consume_newlines();
        }
        self.expect(TokenKind::Dedent)?;
        Ok(items)
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        match self.peek_kind() {
            Some(TokenKind::Not) => {
                self.advance();
                let expr = self.parse_expression(9)?;
                Ok(Expression::Unary {
                    op: "not".to_string(),
                    expr: Box::new(expr),
                })
            }
            Some(TokenKind::Minus) => {
                self.advance();
                let expr = self.parse_expression(9)?;
                Ok(Expression::Unary {
                    op: "-".to_string(),
                    expr: Box::new(expr),
                })
            }
            Some(TokenKind::Identifier(id)) => {
                self.advance();
                Ok(Expression::Identifier(id))
            }
            Some(TokenKind::Number(n)) => {
                self.advance();
                Ok(Expression::Number(n))
            }
            Some(TokenKind::StringLiteral(s)) => {
                self.advance();
                Ok(Expression::StringLiteral(s))
            }
            Some(TokenKind::Boolean(b)) => {
                self.advance();
                Ok(Expression::Boolean(b))
            }
            Some(TokenKind::Null) => {
                self.advance();
                Ok(Expression::Null)
            }
            Some(TokenKind::LParen) => {
                self.advance();
                let expr = self.parse_expression(1)?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            _ => Err(format!(
                "Expected primary expression at line {}",
                self.peek_line()
            )),
        }
    }

    fn get_precedence(&self, op: &TokenKind) -> u8 {
        match op {
            TokenKind::Dot | TokenKind::LParen => 10,
            TokenKind::Star | TokenKind::Slash => 8,
            TokenKind::Plus | TokenKind::Minus => 7,
            TokenKind::EqEq
            | TokenKind::Neq
            | TokenKind::Gt
            | TokenKind::Lt
            | TokenKind::Gte
            | TokenKind::Lte => 6,
            TokenKind::And | TokenKind::Or => 5,
            TokenKind::Assign => 2,
            _ => 0,
        }
    }

    fn op_to_str(&self, op: &TokenKind) -> String {
        match op {
            TokenKind::Plus => "+".to_string(),
            TokenKind::Minus => "-".to_string(),
            TokenKind::Star => "*".to_string(),
            TokenKind::Slash => "/".to_string(),
            TokenKind::EqEq => "==".to_string(),
            TokenKind::Neq => "!=".to_string(),
            TokenKind::Gt => ">".to_string(),
            TokenKind::Lt => "<".to_string(),
            TokenKind::Gte => ">=".to_string(),
            TokenKind::Lte => "<=".to_string(),
            TokenKind::And => "and".to_string(),
            TokenKind::Or => "or".to_string(),
            TokenKind::Assign => "=".to_string(),
            _ => "".to_string(),
        }
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expression>, String> {
        let mut args = Vec::new();
        if !self.check(TokenKind::RParen) {
            args.push(self.parse_expression(1)?);
            while self.check(TokenKind::Comma) {
                self.advance();
                args.push(self.parse_expression(1)?);
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(args)
    }

    fn check_identifier(&self) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Identifier(_)))
    }

    fn peek_token_kind(&self) -> Option<TokenKind> {
        self.tokens.get(self.position).map(|t| t.kind.clone())
    }

    fn parse_component_params(&mut self) -> Result<Vec<ComponentParam>, String> {
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while !self.check(TokenKind::RParen) {
            let name = self.expect_identifier()?;
            let mut ty = None;
            if self.check(TokenKind::Colon) {
                self.advance();
                let next_kind = self.peek_kind().ok_or_else(|| "Unexpected EOF".to_string())?;
                let type_str = match next_kind {
                    TokenKind::Identifier(t) => { self.advance(); t }
                    TokenKind::Str => { self.advance(); "str".to_string() }
                    TokenKind::Int => { self.advance(); "int".to_string() }
                    TokenKind::Float => { self.advance(); "float".to_string() }
                    TokenKind::Bool => { self.advance(); "bool".to_string() }
                    TokenKind::Email => { self.advance(); "email".to_string() }
                    TokenKind::Password => { self.advance(); "password".to_string() }
                    TokenKind::DateTime => { self.advance(); "datetime".to_string() }
                    TokenKind::Money => { self.advance(); "money".to_string() }
                    _ => return Err(format!("Expected type name in component parameters, found {:?}", next_kind)),
                };
                ty = Some(type_str);
            }
            let mut default_value = None;
            let mut required = true;
            if self.check(TokenKind::Assign) {
                self.advance();
                default_value = Some(self.parse_expression(1)?);
                required = false;
            }
            params.push(ComponentParam {
                name,
                ty,
                default_value,
                required,
            });
            if self.check(TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(params)
    }

    fn parse_css_decls_block(&mut self) -> Result<Vec<CssDecl>, String> {
        let mut decls = Vec::new();
        while !self.check(TokenKind::Dedent) {
            self.consume_newlines();
            if self.check(TokenKind::Dedent) {
                break;
            }
            
            let mut prop = String::new();
            while !self.check(TokenKind::Colon) && self.peek_kind().is_some() {
                let tk = self.peek_kind().unwrap();
                if Self::is_identifier_like_token(&tk) {
                    prop.push_str(&self.expect_identifier()?);
                    continue;
                }
                match tk {
                    TokenKind::Minus => {
                        self.advance();
                        prop.push('-');
                    }
                    _ => break,
                }
            }
            
            if self.check(TokenKind::Colon) {
                self.advance();
            } else {
                return Err(format!("Expected ':' in CSS declaration, found {:?}", self.peek_kind()));
            }
            
            let mut val = String::new();
            let mut prev_was_word = false;
            while !self.check(TokenKind::NewLine)
                && !self.check(TokenKind::Dedent)
                && self.peek_kind().is_some()
            {
                let tk = self.peek_kind().unwrap();
                let (token_str, is_word) = match tk {
                    _ if Self::is_identifier_like_token(&tk) => {
                        (self.expect_identifier()?, true)
                    }
                    TokenKind::Number(n) => {
                        self.advance();
                        (n.to_string(), true)
                    }
                    TokenKind::StringLiteral(s) => {
                        self.advance();
                        (format!("\"{}\"", s), true)
                    }
                    TokenKind::Dot => {
                        self.advance();
                        (".".to_string(), false)
                    }
                    TokenKind::Colon => {
                        self.advance();
                        (":".to_string(), false)
                    }
                    TokenKind::Comma => {
                        self.advance();
                        (",".to_string(), false)
                    }
                    TokenKind::Percent => {
                        self.advance();
                        ("%".to_string(), false)
                    }
                    TokenKind::LParen => {
                        self.advance();
                        ("(".to_string(), false)
                    }
                    TokenKind::RParen => {
                        self.advance();
                        (")".to_string(), false)
                    }
                    TokenKind::Minus => {
                        self.advance();
                        ("-".to_string(), false)
                    }
                    TokenKind::Slash => {
                        self.advance();
                        ("/".to_string(), false)
                    }
                    TokenKind::Plus => {
                        self.advance();
                        ("+".to_string(), false)
                    }
                    TokenKind::Star => {
                        self.advance();
                        ("*".to_string(), false)
                    }
                    TokenKind::Assign => {
                        self.advance();
                        ("=".to_string(), false)
                    }
                    _ => {
                        self.advance();
                        (String::new(), false)
                    }
                };
                if !token_str.is_empty() {
                    if (prev_was_word || val.ends_with(')')) && is_word {
                        val.push(' ');
                    }
                    val.push_str(&token_str);
                    prev_was_word = is_word;
                }
            }
            decls.push(CssDecl { property: prop, value: val });
            self.consume_newlines();
        }
        Ok(decls)
    }

    fn parse_style_rules_inline(&mut self) -> Result<Vec<StyleRule>, String> {
        let decls = self.parse_css_decls_block()?;
        Ok(vec![StyleRule {
            selector: "&".to_string(),
            declarations: decls,
        }])
    }

    fn parse_slots_rules(&mut self) -> Result<Vec<(String, Vec<StyleRule>)>, String> {
        let mut slot_rules = Vec::new();
        while !self.check(TokenKind::Dedent) {
            let slot_name = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            self.consume_newlines();
            self.expect(TokenKind::Indent)?;
            let decls = self.parse_css_decls_block()?;
            self.expect(TokenKind::Dedent)?;
            self.consume_newlines();
            let style_rule = StyleRule {
                selector: "&".to_string(),
                declarations: decls,
            };
            slot_rules.push((slot_name, vec![style_rule]));
        }
        Ok(slot_rules)
    }

    fn parse_responsive_rules(&mut self) -> Result<Vec<ResponsiveRule>, String> {
        let mut rules = Vec::new();
        while !self.check(TokenKind::Dedent) {
            let breakpoint = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            self.consume_newlines();
            self.expect(TokenKind::Indent)?;
            let decls = self.parse_css_decls_block()?;
            self.expect(TokenKind::Dedent)?;
            self.consume_newlines();
            let style_rule = StyleRule {
                selector: "&".to_string(),
                declarations: decls,
            };
            rules.push(ResponsiveRule {
                breakpoint,
                rules: vec![style_rule],
            });
        }
        Ok(rules)
    }

    fn parse_variant_node(&mut self) -> Result<VariantDecl, String> {
        self.expect(TokenKind::Variant)?;
        let target = self.expect_identifier()?;
        self.expect(TokenKind::Dot)?;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::Colon)?;
        self.consume_newlines();
        self.expect(TokenKind::Indent)?;
        
        let mut base_rules = Vec::new();
        let mut hover_rules = Vec::new();
        let mut slot_rules = Vec::new();
        let mut responsive_rules = Vec::new();
        
        while !self.check(TokenKind::Dedent) {
            let sec_name = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            self.consume_newlines();
            self.expect(TokenKind::Indent)?;
            match sec_name.as_str() {
                "base" => {
                    base_rules = self.parse_style_rules_inline()?;
                }
                "hover" => {
                    hover_rules = self.parse_style_rules_inline()?;
                }
                "slots" => {
                    slot_rules = self.parse_slots_rules()?;
                }
                "responsive" => {
                    responsive_rules = self.parse_responsive_rules()?;
                }
                _ => return Err(format!("Unknown variant section '{}' in variant declaration at line {}", sec_name, self.peek_line())),
            }
            self.expect(TokenKind::Dedent)?;
            self.consume_newlines();
        }
        self.expect(TokenKind::Dedent)?;
        Ok(VariantDecl {
            target,
            name,
            base_rules,
            hover_rules,
            slot_rules,
            responsive_rules,
        })
    }

    fn parse_tokens_decl(&mut self) -> Result<TokenConfigBlock, String> {
        self.expect(TokenKind::Tokens)?;
        self.expect(TokenKind::Colon)?;
        self.consume_newlines();
        self.expect(TokenKind::Indent)?;
        
        let mut colors = Vec::new();
        let mut spacing = Vec::new();
        let mut radius = Vec::new();
        let mut shadows = Vec::new();
        
        while !self.check(TokenKind::Dedent) {
            let category = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            self.consume_newlines();
            self.expect(TokenKind::Indent)?;
            
            while !self.check(TokenKind::Dedent) {
                let name = self.expect_identifier()?;
                self.expect(TokenKind::Colon)?;
                let val = self.parse_simple_setting_value()?;
                self.consume_newlines();
                match category.as_str() {
                    "color" | "colors" => colors.push((name, val)),
                    "space" | "spacing" => spacing.push((name, val)),
                    "radius" => radius.push((name, val)),
                    "shadow" | "shadows" => shadows.push((name, val)),
                    _ => {}
                }
            }
            self.expect(TokenKind::Dedent)?;
            self.consume_newlines();
        }
        self.expect(TokenKind::Dedent)?;
        Ok(TokenConfigBlock {
            colors,
            spacing,
            radius,
            shadows,
        })
    }
}
