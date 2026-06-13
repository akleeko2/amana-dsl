// src/codegen/html.rs
use crate::ast::*;

fn is_js_reserved_identifier(name: &str) -> bool {
    matches!(
        name,
        "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "let"
            | "new"
            | "null"
            | "return"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
    )
}

fn is_valid_js_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_' || first == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
        && !is_js_reserved_identifier(name)
}

fn safe_js_identifier(name: &str) -> String {
    if is_valid_js_identifier(name) {
        return name.to_string();
    }

    let mut safe = String::from("__amana_");
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '$' {
            safe.push(c);
        } else {
            safe.push('_');
        }
    }
    if safe == "__amana_" {
        safe.push_str("item");
    }
    safe
}

fn scoped_identifier(id: &str, aliases: &[(String, String)]) -> String {
    aliases
        .iter()
        .rev()
        .find(|(source, _)| source == id)
        .map(|(_, target)| target.clone())
        .unwrap_or_else(|| id.to_string())
}

/// Compiles an Amana AST Expression into its JavaScript equivalent representation.
/// This includes arithmetic/logical expressions, member accesses, and function calls.
/// It dynamically translates env("VAR", "default") calls into Node.js process.env expressions.
pub fn compile_expression_to_js(expr: &Expression) -> String {
    compile_expression_to_js_scoped(expr, &[])
}

fn compile_expression_to_js_scoped(expr: &Expression, aliases: &[(String, String)]) -> String {
    match expr {
        Expression::Number(n) => n.to_string(),
        Expression::StringLiteral(s) => {
            if s.starts_with("f\"") && s.ends_with("\"") {
                let content = &s[2..s.len() - 1];
                let js_template = content.replace("{", "${");
                format!("`{}`", js_template)
            } else {
                format!("\"{}\"", s.replace("\"", "\\\""))
            }
        }
        Expression::Boolean(b) => b.to_string(),
        Expression::Null => "null".to_string(),
        Expression::Identifier(id) => scoped_identifier(id, aliases),
        Expression::Binary { left, op, right } => {
            let l = compile_expression_to_js_scoped(left, aliases);
            let r = compile_expression_to_js_scoped(right, aliases);
            let js_op = match op.as_str() {
                "and" => "&&",
                "or" => "||",
                _ => op,
            };
            format!("({} {} {})", l, js_op, r)
        }
        Expression::Unary { op, expr } => {
            let e = compile_expression_to_js_scoped(expr, aliases);
            let js_op = match op.as_str() {
                "not" => "!",
                _ => op,
            };
            format!("({}{})", js_op, e)
        }
        Expression::Ternary {
            cond,
            then_branch,
            else_branch,
        } => {
            let c = compile_expression_to_js_scoped(cond, aliases);
            let t = compile_expression_to_js_scoped(then_branch, aliases);
            let el = compile_expression_to_js_scoped(else_branch, aliases);
            format!("({} ? {} : {})", c, t, el)
        }
        Expression::MemberAccess { object, property } => {
            let obj = compile_expression_to_js_scoped(object, aliases);
            if obj == "User" && property == "current" {
                "currentUser".to_string()
            } else {
                format!("{}.{}", obj, property)
            }
        }
        Expression::Call { callee, args } => {
            if let Expression::Identifier(name) = &**callee
                && name == "env"
            {
                if args.len() == 1 {
                    return format!(
                        "(process.env[{}] || \"\")",
                        compile_expression_to_js_scoped(&args[0], aliases)
                    );
                } else if args.len() == 2 {
                    return format!(
                        "(process.env[{}] || {})",
                        compile_expression_to_js_scoped(&args[0], aliases),
                        compile_expression_to_js_scoped(&args[1], aliases)
                    );
                }
            }
            let c = compile_expression_to_js_scoped(callee, aliases);
            let formatted_args: Vec<String> = args
                .iter()
                .map(|arg| compile_expression_to_js_scoped(arg, aliases))
                .collect();
            format!("{}({})", c, formatted_args.join(", "))
        }
    }
}

/// Checks if an Expression references any client-side states.
fn references_client_state(expr: &Expression, client_states: &[StateDecl]) -> bool {
    match expr {
        Expression::Identifier(id) => client_states.iter().any(|s| s.name == *id),
        Expression::Binary { left, right, .. } => {
            references_client_state(left, client_states)
                || references_client_state(right, client_states)
        }
        Expression::Unary { expr, .. } => references_client_state(expr, client_states),
        Expression::Ternary {
            cond,
            then_branch,
            else_branch,
        } => {
            references_client_state(cond, client_states)
                || references_client_state(then_branch, client_states)
                || references_client_state(else_branch, client_states)
        }
        Expression::Call { callee, args } => {
            references_client_state(callee, client_states)
                || args
                    .iter()
                    .any(|arg| references_client_state(arg, client_states))
        }
        Expression::MemberAccess { object, .. } => references_client_state(object, client_states),
        _ => false,
    }
}

/// Helper function to check if formatted string template interpolation contains references to client states.
fn text_references_client_state(txt: &str, client_states: &[StateDecl]) -> bool {
    if txt.starts_with("f\"") && txt.ends_with("\"") {
        let content = &txt[2..txt.len() - 1];
        for state in client_states {
            if content.contains(&format!("{{{}}}", state.name)) {
                return true;
            }
        }
    }
    false
}

fn expr_static_value_scoped(
    expr: &Expression,
    fallback: &str,
    aliases: &[(String, String)],
) -> String {
    match expr {
        Expression::StringLiteral(value) => value.clone(),
        Expression::Identifier(id) => format!("<%= {} %>", scoped_identifier(id, aliases)),
        Expression::Number(n) => n.to_string(),
        Expression::Boolean(b) => b.to_string(),
        _ => {
            let js = compile_expression_to_js_scoped(expr, aliases);
            if js.is_empty() {
                fallback.to_string()
            } else {
                format!("<%= {} %>", js)
            }
        }
    }
}

fn get_attr_scoped(
    attributes: &[(String, Expression)],
    name: &str,
    fallback: &str,
    aliases: &[(String, String)],
) -> String {
    attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, expr)| expr_static_value_scoped(expr, fallback, aliases))
        .unwrap_or_else(|| fallback.to_string())
}

fn standard_class(base: &str, classes: &[String]) -> String {
    if classes.is_empty() {
        base.to_string()
    } else {
        format!("{} {}", base, classes.join(" "))
    }
}

fn render_icon(name: &str, class_name: &str) -> String {
    let raw = name.trim();
    if raw.is_empty() {
        return String::new();
    }
    let is_iconify = {
        let parts: Vec<&str> = raw.split(':').collect();
        if parts.len() == 2 {
            let p0 = parts[0];
            let p1 = parts[1];
            !p0.is_empty() && !p1.is_empty() &&
            p0.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') &&
            p1.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')
        } else {
            false
        }
    };
    if is_iconify {
        format!(
            "<iconify-icon class=\"{}\" icon=\"{}\" aria-hidden=\"true\"></iconify-icon>",
            class_name, raw
        )
    } else {
        let fallback = match raw {
            "check" => "✓",
            "close" | "x" => "×",
            "menu" => "☰",
            "search" => "⌕",
            "arrow" | "arrow-right" => "→",
            "arrow-left" => "←",
            "plus" => "+",
            "minus" => "-",
            "star" => "★",
            _ => raw,
        };
        format!(
            "<span class=\"{}\" aria-hidden=\"true\">{}</span>",
            class_name, fallback
        )
    }
}

fn render_standard_component(
    tag: &str,
    classes: &[String],
    attributes: &[(String, Expression)],
    children: &[ViewElement],
    client_states: &[StateDecl],
    aliases: &[(String, String)],
) -> Option<String> {
    let attr = |name: &str, fallback: &str| get_attr_scoped(attributes, name, fallback, aliases);
    let inner = children
        .iter()
        .map(|child| generate_ejs_scoped(child, client_states, aliases))
        .collect::<String>();
    match tag {
        "Button" => {
            let href = attr("href", "");
            let label = {
                let val = attr("label", "");
                if val.is_empty() {
                    attr("text", "")
                } else {
                    val
                }
            };
            let variant = attr("variant", "primary");
            let size = attr("size", "md");
            let intent = attr("intent", "default");
            let icon = attr("icon", "");
            let body = if inner.is_empty() { label } else { inner };
            let icon_markup = if icon.is_empty() {
                String::new()
            } else {
                render_icon(&icon, "amana-btn-icon")
            };
            let content = format!("{}<span>{}</span>", icon_markup, body);
            if href.is_empty() {
                Some(format!(
                    "<button class=\"{}\" type=\"button\">{}</button>",
                    standard_class(&format!("amana-btn amana-btn-{} amana-btn-{} amana-btn-intent-{}", variant, size, intent), classes),
                    content
                ))
            } else {
                Some(format!(
                    "<a class=\"{}\" href=\"{}\">{}</a>",
                    standard_class(&format!("amana-btn amana-btn-{} amana-btn-{} amana-btn-intent-{}", variant, size, intent), classes),
                    href,
                    content
                ))
            }
        }
        "Card" | "FeatureCard" | "PricingCard" => {
            let eyebrow = attr("eyebrow", "");
            let badge = attr("badge", "");
            let title = attr("title", "");
            let subtitle = {
                let value = attr("subtitle", "");
                if value.is_empty() {
                    attr("description", "")
                } else {
                    value
                }
            };
            let price = attr("price", "");
            let meta = attr("meta", "");
            let action_label = attr("action_label", "");
            let action_href = attr("action_href", "#");
            let density = attr("density", "comfortable");
            let kind = match tag {
                "FeatureCard" => " amana-feature-card",
                "PricingCard" => " amana-pricing-card",
                _ => "",
            };
            let card_top = if !eyebrow.is_empty() || !badge.is_empty() || !meta.is_empty() {
                format!(
                    "<div class=\"amana-card-top\">{}{}{}</div>",
                    if eyebrow.is_empty() { "".to_string() } else { format!("<span class=\"amana-eyebrow\">{}</span>", eyebrow) },
                    if badge.is_empty() { "".to_string() } else { format!("<span class=\"amana-badge\">{}</span>", badge) },
                    if meta.is_empty() { "".to_string() } else { format!("<span class=\"amana-card-meta\">{}</span>", meta) }
                )
            } else {
                "".to_string()
            };
            let action = if action_label.is_empty() {
                "".to_string()
            } else {
                format!("<a class=\"amana-card-action\" href=\"{}\">{}</a>", action_href, action_label)
            };
            Some(format!(
                "<article class=\"{}\">{}{}{}{}{}{}</article>",
                standard_class(&format!("amana-card{} amana-card-density-{}", kind, density), classes),
                card_top,
                if title.is_empty() { "".to_string() } else { format!("<h3>{}</h3>", title) },
                if subtitle.is_empty() { "".to_string() } else { format!("<p class=\"amana-muted\">{}</p>", subtitle) },
                if price.is_empty() { "".to_string() } else { format!("<div class=\"amana-price\">{}</div>", price) },
                inner,
                action
            ))
        }
        "Container" => {
            let width = attr("width", "default");
            Some(format!(
                "<div class=\"{}\">{}</div>",
                standard_class(&format!("amana-container amana-container-{}", width), classes),
                inner
            ))
        }
        "Section" => {
            let eyebrow = attr("eyebrow", "");
            let title = attr("title", "");
            let subtitle = {
                let value = attr("subtitle", "");
                if value.is_empty() {
                    attr("description", "")
                } else {
                    value
                }
            };
            let header = if !eyebrow.is_empty() || !title.is_empty() || !subtitle.is_empty() {
                format!(
                    "<header class=\"amana-section-head\">{}{}{}</header>",
                    if eyebrow.is_empty() { "".to_string() } else { format!("<p class=\"amana-eyebrow\">{}</p>", eyebrow) },
                    if title.is_empty() { "".to_string() } else { format!("<h2>{}</h2>", title) },
                    if subtitle.is_empty() { "".to_string() } else { format!("<p class=\"amana-section-copy\">{}</p>", subtitle) }
                )
            } else {
                "".to_string()
            };
            Some(format!(
                "<section class=\"{}\">{}{}</section>",
                standard_class("amana-section", classes),
                header,
                inner
            ))
        }
        "Grid" => {
            let min = attr("min", "16rem");
            let columns = attr("columns", "");
            let style_attr = if columns.is_empty() {
                format!("style=\"--grid-min:{}\"", min)
            } else {
                format!("style=\"--grid-min:{};--dg-columns:{}\"", min, columns)
            };
            Some(format!(
                "<div class=\"{}\" {}>{}</div>",
                standard_class("amana-grid", classes),
                style_attr,
                inner
            ))
        }
        "Stack" => {
            let gap = attr("gap", "md");
            Some(format!(
                "<div class=\"{}\">{}</div>",
                standard_class(&format!("amana-stack amana-stack-gap-{}", gap), classes),
                inner
            ))
        }
        "FormField" => {
            let name = attr("name", "");
            let label = attr("label", &name);
            let placeholder = attr("placeholder", "");
            let input_type = attr("type", "text");
            let help = attr("help", "");
            let required_str = attr("required", "false");
            let required = required_str == "true" || required_str == "yes" || required_str == "1";
            let required_attr = if required { " required aria-required=\"true\"" } else { "" };
            let required_indicator = if required {
                "<span class=\"amana-required\" aria-hidden=\"true\">*</span>"
            } else {
                ""
            };
            let help_html = if help.is_empty() {
                String::new()
            } else {
                format!("<small class=\"amana-help\">{}</small>", help)
            };
            Some(format!(
                "<label class=\"{}\"><span>{}{}</span><input class=\"amana-form-control\" type=\"{}\" name=\"{}\" id=\"{}\" placeholder=\"{}\"{}>{}</label>",
                standard_class("amana-field", classes),
                label,
                required_indicator,
                input_type,
                name,
                name,
                placeholder,
                required_attr,
                help_html
            ))
        }
        "Navbar" => {
            let brand = attr("brand", "<%= title %>");
            let sticky = attr("sticky", "false") == "true";
            let sticky_class = if sticky { " amana-navbar-sticky" } else { "" };
            let links_raw = attr("links", "");
            let nav_content = if !links_raw.is_empty() {
                let mut links_html = String::new();
                for link in links_raw.split(',') {
                    let trimmed = link.trim();
                    if trimmed.is_empty() { continue; }
                    let mut parts = trimmed.splitn(2, ' ');
                    let first = parts.next().unwrap_or("");
                    let second = parts.next();
                    let (text, href) = if let Some(path) = second {
                        (first, path.trim())
                    } else if first.starts_with('/') {
                        (first, first)
                    } else {
                        (first, "#")
                    };
                    links_html.push_str(&format!(
                        "<a class=\"amana-nav-link\" href=\"{}\">{}</a>",
                        href, text
                    ));
                }
                links_html
            } else {
                inner
            };
            Some(format!(
                "<nav class=\"{}\"><a class=\"amana-brand\" href=\"/\">{}</a><div class=\"amana-navlinks\">{}</div></nav>",
                standard_class(&format!("amana-navbar{}", sticky_class), classes),
                brand,
                nav_content
            ))
        }
        "Hero" => {
            let eyebrow = attr("eyebrow", "");
            let title = attr("title", "");
            let subtitle = attr("subtitle", "");
            let media = attr("media", "");
            let proof = attr("proof", "");
            let text = format!(
                "<div class=\"amana-hero-content\">{}{}{}{}<div class=\"amana-hero-actions\">{}</div></div>",
                if eyebrow.is_empty() { "".to_string() } else { format!("<p class=\"amana-eyebrow\">{}</p>", eyebrow) },
                if title.is_empty() { "".to_string() } else { format!("<h1>{}</h1>", title) },
                if subtitle.is_empty() { "".to_string() } else { format!("<p class=\"amana-hero-copy\">{}</p>", subtitle) },
                if proof.is_empty() { "".to_string() } else { format!("<p class=\"amana-hero-proof\">{}</p>", proof) },
                inner
            );
            let media_markup = if media.is_empty() {
                "".to_string()
            } else {
                format!("<div class=\"amana-hero-media\" style=\"background-image:url('{}')\"></div>", media)
            };
            Some(format!(
                "<section class=\"{}\">{}{}</section>",
                standard_class("amana-hero", classes),
                text,
                media_markup
            ))
        }
        "Alert" => {
            let tone = attr("tone", "info");
            let message = attr("message", "");
            Some(format!(
                "<div class=\"{}\">{}</div>",
                standard_class(&format!("amana-alert amana-alert-{}", tone), classes),
                if inner.is_empty() { message } else { inner }
            ))
        }
        "Footer" => Some(format!(
            "<footer class=\"{}\">{}</footer>",
            standard_class("amana-footer", classes),
            inner
        )),
        "Icon" => {
            let name = {
                let val = attr("name", "");
                if val.is_empty() {
                    attr("icon", "")
                } else {
                    val
                }
            };
            Some(render_icon(&name, &standard_class("amana-icon", classes)))
        }
        "Modal" => {
            let open = attr("open", "modal_open");
            Some(format!(
                "<div class=\"{}\" x-show=\"{}\"><div class=\"amana-modal-panel\">{}</div></div>",
                standard_class("amana-modal", classes),
                open,
                inner
            ))
        }
        "Tabs" => Some(format!(
            "<div class=\"{}\">{}</div>",
            standard_class("amana-tabs", classes),
            inner
        )),
        "Badge" => {
            let label = {
                let val = attr("label", "");
                if val.is_empty() { inner } else { val }
            };
            let tone = attr("tone", "neutral");
            Some(format!(
                "<span class=\"{}\">{}</span>",
                standard_class(&format!("amana-badge amana-badge-{}", tone), classes),
                label
            ))
        }
        "Kpi" | "Stat" => {
            let label = attr("label", "");
            let value = attr("value", "");
            let trend = attr("trend", "");
            Some(format!(
                "<article class=\"{}\">{}{}{}{}</article>",
                standard_class("amana-kpi", classes),
                if label.is_empty() { "".to_string() } else { format!("<span class=\"amana-kpi-label\">{}</span>", label) },
                if value.is_empty() { "".to_string() } else { format!("<strong class=\"amana-kpi-value\">{}</strong>", value) },
                if trend.is_empty() { "".to_string() } else { format!("<span class=\"amana-kpi-trend\">{}</span>", trend) },
                inner
            ))
        }
        "LogoCloud" => {
            let title = attr("title", "");
            Some(format!(
                "<section class=\"{}\">{}<div class=\"amana-logo-row\">{}</div></section>",
                standard_class("amana-logo-cloud", classes),
                if title.is_empty() { "".to_string() } else { format!("<p class=\"amana-muted\">{}</p>", title) },
                inner
            ))
        }
        "TestimonialCard" => {
            let quote = attr("quote", "");
            let author = attr("author", "");
            let role = attr("role", "");
            let quote_markup = if quote.is_empty() {
                inner
            } else {
                format!("<blockquote>{}</blockquote>", quote)
            };
            let figcaption = if !author.is_empty() || !role.is_empty() {
                format!(
                    "<figcaption>{}{}</figcaption>",
                    if author.is_empty() { "".to_string() } else { format!("<strong>{}</strong>", author) },
                    if role.is_empty() { "".to_string() } else { format!("<span>{}</span>", role) }
                )
            } else {
                "".to_string()
            };
            Some(format!(
                "<figure class=\"{}\">{}{}</figure>",
                standard_class("amana-testimonial", classes),
                quote_markup,
                figcaption
            ))
        }
        "Timeline" => Some(format!(
            "<ol class=\"{}\">{}</ol>",
            standard_class("amana-timeline", classes),
            inner
        )),
        "TimelineItem" => {
            let title = attr("title", "");
            let meta = attr("meta", "");
            Some(format!(
                "<li class=\"{}\">{}{}{}</li>",
                standard_class("amana-timeline-item", classes),
                if meta.is_empty() { "".to_string() } else { format!("<span class=\"amana-card-meta\">{}</span>", meta) },
                if title.is_empty() { "".to_string() } else { format!("<h3>{}</h3>", title) },
                inner
            ))
        }
        "EmptyState" => {
            let title = attr("title", "");
            let description = attr("description", "");
            let action_label = attr("action_label", "");
            let action_href = attr("action_href", "#");
            let action_markup = if action_label.is_empty() {
                "".to_string()
            } else {
                format!("<a class=\"amana-btn amana-btn-primary\" href=\"{}\">{}</a>", action_href, action_label)
            };
            Some(format!(
                "<section class=\"{}\">{}{}{}{}</section>",
                standard_class("amana-empty-state", classes),
                if title.is_empty() { "".to_string() } else { format!("<h2>{}</h2>", title) },
                if description.is_empty() { "".to_string() } else { format!("<p>{}</p>", description) },
                inner,
                action_markup
            ))
        }
        "Split" => Some(format!(
            "<div class=\"{}\">{}</div>",
            standard_class("amana-split", classes),
            inner
        )),
        "Cluster" => Some(format!(
            "<div class=\"{}\">{}</div>",
            standard_class("amana-cluster", classes),
            inner
        )),
        "Sidebar" => Some(format!(
            "<aside class=\"{}\">{}</aside>",
            standard_class("amana-sidebar", classes),
            inner
        )),
        _ => None,
    }
}

/// Generates standard EJS HTML tags recursively from a ViewElement tree.
/// Binds interactive UI events and state changes to Alpine.js expressions.
pub fn generate_ejs(element: &ViewElement, client_states: &[StateDecl]) -> String {
    generate_ejs_scoped(element, client_states, &[])
}

fn generate_ejs_scoped(
    element: &ViewElement,
    client_states: &[StateDecl],
    aliases: &[(String, String)],
) -> String {
    match element {
        ViewElement::Element {
            tag,
            classes,
            attributes,
            children,
        } => {
            let tag_lower = tag.to_lowercase();
            const BLOCKED_HTML_TAGS: &[&str] = &[
                "script", "iframe", "object", "embed", "applet",
                "link", "meta", "base", "style", "noscript",
            ];
            if !tag.chars().next().is_some_and(|c| c.is_uppercase())
                && BLOCKED_HTML_TAGS.contains(&tag_lower.as_str())
            {
                return format!(
                    "<!-- [Amana Security] Tag <{}> is blocked. Use components instead. -->\n",
                    tag
                );
            }

            if let Some(rendered) = render_standard_component(
                tag,
                classes,
                attributes,
                children,
                client_states,
                aliases,
            )
            {
                return rendered;
            }

            // Extract design blocks from children
            let mut dg_classes = Vec::new();
            let mut dg_styles = Vec::new();
            let mut dg_attrs = Vec::new();
            for child in children {
                if let ViewElement::DesignBlock(block) = child {
                    dg_classes.extend(crate::codegen::express::design_class_list(block));
                    let styles = crate::codegen::express::design_style_vars(block);
                    if !styles.is_empty() {
                        dg_styles.push(styles);
                    }
                    let attrs = crate::codegen::express::design_data_attrs(block);
                    if !attrs.is_empty() {
                        dg_attrs.push(attrs);
                    }
                }
            }

            // Merge classes
            let mut merged_classes = classes.clone();
            merged_classes.extend(dg_classes);
            let mut unique_classes = Vec::new();
            for c in merged_classes {
                if !unique_classes.contains(&c) {
                    unique_classes.push(c);
                }
            }
            let class_str = if !unique_classes.is_empty() {
                format!(" class=\"{}\"", unique_classes.join(" "))
            } else {
                "".to_string()
            };

            // Separate style attribute from others
            let mut style_expr = None;
            let mut other_attributes = Vec::new();
            for (key, expr) in attributes {
                if key == "style" {
                    style_expr = Some(expr);
                } else {
                    other_attributes.push((key, expr));
                }
            }

            let mut style_str = String::new();
            if !dg_styles.is_empty() {
                style_str.push_str(&dg_styles.join(";"));
            }
            if let Some(expr) = style_expr {
                let expr_js = compile_expression_to_js_scoped(expr, aliases);
                if !style_str.is_empty() {
                    style_str.push_str("; ");
                }
                style_str.push_str(&format!("<%= {} %>", expr_js));
            }

            let mut attrs = String::new();
            if !style_str.is_empty() {
                attrs.push_str(&format!(" style=\"{}\"", style_str));
            }

            let event_keys = [
                "click",
                "submit",
                "change",
                "input",
                "keydown",
                "keyup",
                "focus",
                "blur",
                "mouseenter",
                "mouseleave",
            ];
            for (key, expr) in &other_attributes {
                if *key == "bind" || *key == "model" {
                    if let Expression::Identifier(id) = expr {
                        if client_states.iter().any(|s| s.name == *id) || *key == "model" {
                            attrs.push_str(&format!(
                                " x-model=\"{}\" name=\"{}\" id=\"{}\"",
                                id, id, id
                            ));
                        } else {
                            attrs.push_str(&format!(" value=\"<%= typeof {} !== 'undefined' ? {} : '' %>\" name=\"{}\" id=\"{}\"", id, id, id, id));
                        }
                    }
                } else if event_keys.contains(&key.as_str()) {
                    attrs.push_str(&format!(
                        " x-on:{}=\"{}\"",
                        key,
                        compile_expression_to_js_scoped(expr, aliases)
                    ));
                } else if *key == "show" {
                    attrs.push_str(&format!(
                        " x-show=\"{}\"",
                        compile_expression_to_js_scoped(expr, aliases)
                    ));
                } else if *key == "text" {
                    attrs.push_str(&format!(
                        " x-text=\"{}\"",
                        compile_expression_to_js_scoped(expr, aliases)
                    ));
                } else if *key == "init" {
                    let code = match expr {
                        Expression::StringLiteral(s) => s.clone(),
                        _ => compile_expression_to_js_scoped(expr, aliases),
                    };
                    let escaped = code.replace('&', "&amp;")
                                      .replace('"', "&quot;")
                                      .replace('<', "&lt;")
                                      .replace('>', "&gt;");
                    attrs.push_str(&format!(" x-init=\"{}\"", escaped));
                } else if matches!(
                    key.as_str(),
                    "disabled" | "checked" | "selected" | "readonly"
                ) {
                    attrs.push_str(&format!(
                        " :{}=\"{}\"",
                        key,
                        compile_expression_to_js_scoped(expr, aliases)
                    ));
                } else {
                    attrs.push_str(&format!(
                        " {}=\"<%= {} %>\"",
                        key,
                        compile_expression_to_js_scoped(expr, aliases)
                    ));
                }
            }

            for dg_attr in dg_attrs {
                if !dg_attr.is_empty() {
                    attrs.push_str(&format!(" {}", dg_attr));
                }
            }

            let mut inner = String::new();
            for child in children {
                if !matches!(child, ViewElement::DesignBlock(_)) {
                    inner.push_str(&generate_ejs_scoped(child, client_states, aliases));
                }
            }
            format!("<{}{}{}>{}</{}>", tag, class_str, attrs, inner, tag)
        }
        ViewElement::Text(txt) => {
            if text_references_client_state(txt, client_states) {
                let content = &txt[2..txt.len() - 1];
                let js_template = content.replace("{", "${");
                format!("<span x-text=\"`{}`\"></span>", js_template)
            } else if txt.starts_with("f\"") && txt.ends_with("\"") {
                let content = &txt[2..txt.len() - 1];
                // Replace User.current with currentUser in formatted strings
                let mut js_template = content.to_string();

                // Handle all variations of User.current access
                let replacements = [
                    ("{User.current.name}", "${currentUser.name}"),
                    ("{User.current.email}", "${currentUser.email}"),
                    ("{User.current.role}", "${currentUser.role}"),
                    ("{User.current.id}", "${currentUser.id}"),
                    ("{User.current}", "${currentUser}"),
                ];

                for (pattern, replacement) in &replacements {
                    js_template = js_template.replace(pattern, replacement);
                }

                // Replace remaining { with ${
                js_template = js_template.replace("{", "${");

                format!("<%= `{}` %>", js_template)
            } else {
                txt.clone()
            }
        }
        ViewElement::DesignBlock(_) => String::new(),
        ViewElement::FormattedText(exprs) => {
            if exprs
                .iter()
                .any(|e| references_client_state(e, client_states))
            {
                let js_expr = exprs
                    .iter()
                    .map(|expr| compile_expression_to_js_scoped(expr, aliases))
                    .collect::<Vec<String>>()
                    .join(" + ");
                format!("<span x-text=\"{}\"></span>", js_expr)
            } else {
                let mut s = String::new();
                for expr in exprs {
                    s.push_str(&format!(
                        "<%= {} %>",
                        compile_expression_to_js_scoped(expr, aliases)
                    ));
                }
                s
            }
        }
        ViewElement::ForEach {
            item_var,
            list_expr,
            body,
        } => {
            let safe_item_var = safe_js_identifier(item_var);
            let mut scoped_aliases = aliases.to_vec();
            if safe_item_var != *item_var {
                scoped_aliases.push((item_var.clone(), safe_item_var.clone()));
            }
            let inner = body
                .iter()
                .map(|c| generate_ejs_scoped(c, client_states, &scoped_aliases))
                .collect::<String>();
            format!(
                "<% for (let {} of {}) {{ %>\n{}<% }} %>\n",
                safe_item_var,
                compile_expression_to_js_scoped(list_expr, aliases),
                inner
            )
        }
        ViewElement::IfBlock {
            condition,
            then_branch,
            else_branch,
        } => {
            let then_html = then_branch
                .iter()
                .map(|c| generate_ejs_scoped(c, client_states, aliases))
                .collect::<String>();
            let else_html = match else_branch {
                Some(branch) => format!(
                    "<% }} else {{ %>\n{}",
                    branch
                        .iter()
                        .map(|c| generate_ejs_scoped(c, client_states, aliases))
                        .collect::<String>()
                ),
                None => "".to_string(),
            };
            format!(
                "<% if ({}) {{ %>\n{}{}<% }} %>\n",
                compile_expression_to_js_scoped(condition, aliases),
                then_html,
                else_html
            )
        }
        ViewElement::FormBlock {
            fields,
            connect_action,
            redirect_success: _,
            defaults: _,
            constraints: _,
            ui,
            submit_label,
            field_options,
        } => {
            let action_path = format!(
                "/form-submit/{}",
                connect_action.replace(".", "/").to_lowercase()
            );
            let mut form_inner = String::new();

            // Inject CSRF token to prevent CSRF attacks
            form_inner
                .push_str("  <input type=\"hidden\" name=\"_csrf\" value=\"<%= csrfToken %>\">\n");

            for f in fields {
                let field_lower = f.to_lowercase();
                let field_config = field_options
                    .iter()
                    .find(|option| option.name.eq_ignore_ascii_case(f));
                let input_type = field_config
                    .and_then(|option| option.input_type.as_deref())
                    .unwrap_or_else(|| {
                        if field_lower.contains("password") {
                            "password"
                        } else if field_lower.contains("email") {
                            "email"
                        } else {
                            "text"
                        }
                    });
                let label = field_config
                    .and_then(|option| option.label.as_deref())
                    .unwrap_or(f);
                let placeholder = field_config
                    .and_then(|option| option.placeholder.as_deref())
                    .map(|value| format!(" placeholder=\"{}\"", value))
                    .unwrap_or_default();
                let help = field_config
                    .and_then(|option| option.help.as_deref())
                    .map(|value| format!("\n    <small class=\"amana-help\">{}</small>", value))
                    .unwrap_or_default();
                let required = if field_config
                    .and_then(|option| option.required)
                    .unwrap_or(true)
                {
                    " required"
                } else {
                    ""
                };
                if input_type == "textarea" {
                    form_inner.push_str(&format!(
                        "  <label class=\"amana-field\" for=\"{}\">\n    <span>{}</span>\n    <textarea class=\"amana-form-control\" id=\"{}\" name=\"{}\"{}{} rows=\"4\"></textarea>{}\n  </label>\n",
                        f, label, f, f, placeholder, required, help
                    ));
                } else {
                    form_inner.push_str(&format!(
                        "  <label class=\"amana-field\" for=\"{}\">\n    <span>{}</span>\n    <input class=\"amana-form-control\" type=\"{}\" id=\"{}\" name=\"{}\"{}{}>{}\n  </label>\n",
                        f, label, input_type, f, f, placeholder, required, help
                    ));
                }
            }
            form_inner.push_str(&format!(
                "  <button type=\"submit\" class=\"amana-btn amana-btn-primary\">{}</button>\n",
                submit_label.as_deref().unwrap_or("Submit")
            ));

            let form_class = if ui.as_deref() == Some("card") {
                " class=\"amana-form-card\""
            } else {
                ""
            };

            format!(
                "<form{} action=\"{}\" method=\"POST\">\n{}</form>\n",
                form_class, action_path, form_inner
            )
        }
        ViewElement::Chart {
            data_expr,
            chart_type,
            x_field,
            y_field,
        } => {
            format!(
                "<div class=\"chart-container mb-4\" style=\"position: relative; height:40vh; width:80vw\">\n  <canvas id=\"chart_{}\"></canvas>\n</div>\n\
                <script>\n\
                document.addEventListener('DOMContentLoaded', () => {{\n\
                  const ctx = document.getElementById('chart_{}').getContext('2d');\n\
                  const rawData = <%- JSON.stringify({}) %>;\n\
                  new Chart(ctx, {{\n\
                    type: '{}',\n\
                    data: {{\n\
                      labels: rawData.map(row => row.{}),\n\
                      datasets: [{{\n\
                        label: 'بيانات {}',\n\
                        data: rawData.map(row => row.{}),\n\
                        backgroundColor: 'rgba(99, 102, 241, 0.2)',\n\
                        borderColor: 'rgba(99, 102, 241, 1)',\n\
                        borderWidth: 2\n\
                      }}]\n\
                    }},\n\
                    options: {{\n\
                      responsive: true,\n\
                      maintainAspectRatio: false\n\
                    }}\n\
                  }});\n\
                }});\n\
                </script>\n",
                data_expr, data_expr, data_expr, chart_type, x_field, data_expr, y_field
            )
        }
        ViewElement::SlotDecl { name, optional } => {
            if *optional {
                format!("<%- typeof slots !== 'undefined' && slots.{} ? slots.{} : '' %>\n", name, name)
            } else {
                format!("<%- slots.{} %>\n", name)
            }
        }
        ViewElement::ResourceGrid {
            resource_expr,
            item_component,
            item_arg_name,
            empty_element,
            loading_element: _,
            error_element: _,
            filter_fields: _,
            sort_fields: _,
        }
        | ViewElement::ResourceTable {
            resource_expr,
            item_component,
            item_arg_name,
            empty_element,
            loading_element: _,
            error_element: _,
            filter_fields: _,
            sort_fields: _,
        } => {
            let resource_js = compile_expression_to_js_scoped(resource_expr, aliases);
            
            let mut empty_html = String::new();
            if let Some(nodes) = empty_element {
                for node in nodes {
                    empty_html.push_str(&generate_ejs_scoped(node, client_states, aliases));
                }
            }
            
            let loop_html = format!(
                "<% for (let {} of {}) {{ %>\n  <%- include('components/{}', {{ {}: {} }}) %>\n<% }} %>\n",
                item_arg_name, resource_js, item_component, item_arg_name, item_arg_name
            );
            
            format!(
                "<% if (typeof {} !== 'undefined' && {} && {}.length > 0) {{ %>\n\
                 {}\n\
                 <% }} else {{ %>\n\
                 {}\n\
                 <% }} %>\n",
                resource_js, resource_js, resource_js,
                loop_html,
                empty_html
            )
        }
    }
}
