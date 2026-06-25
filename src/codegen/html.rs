use crate::ast::*;

thread_local! {
    static MODAL_COUNTER: std::cell::Cell<usize> = std::cell::Cell::new(0);
    static CHART_COUNTER: std::cell::Cell<usize> = std::cell::Cell::new(0);
}

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
}fn replace_aliases_in_placeholder(placeholder: &str, aliases: &[(String, String)]) -> String {
    let mut result = String::new();
    let mut current_word = String::new();
    
    for c in placeholder.chars() {
        if c.is_alphanumeric() || c == '_' {
            current_word.push(c);
        } else {
            if !current_word.is_empty() {
                let mut replaced = false;
                for (source, target) in aliases {
                    if source == &current_word {
                        result.push_str(target);
                        replaced = true;
                        break;
                    }
                }
                if !replaced {
                    result.push_str(&current_word);
                }
                current_word.clear();
            }
            result.push(c);
        }
    }
    
    if !current_word.is_empty() {
        let mut replaced = false;
        for (source, target) in aliases {
            if source == &current_word {
                result.push_str(target);
                replaced = true;
                break;
            }
        }
        if !replaced {
            result.push_str(&current_word);
        }
    }
    
    result
}

fn process_formatted_string(s: &str, aliases: &[(String, String)]) -> String {
    let mut result = String::new();
    let mut in_placeholder = false;
    let mut placeholder_content = String::new();
    
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            in_placeholder = true;
            placeholder_content.clear();
        } else if c == '}' && in_placeholder {
            in_placeholder = false;
            let replaced = replace_aliases_in_placeholder(&placeholder_content, aliases);
            result.push_str("${");
            result.push_str(&replaced);
            result.push('}');
        } else if in_placeholder {
            placeholder_content.push(c);
        } else {
            result.push(c);
        }
    }
    result
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

pub fn compile_expression_to_js_with_auth_model(
    expr: &Expression,
    _auth_model: &Option<String>,
) -> String {
    compile_expression_to_js_scoped(expr, &[])
}

fn compile_expression_to_js_scoped(expr: &Expression, aliases: &[(String, String)]) -> String {
    match expr {
        Expression::Number(n) => n.to_string(),
        Expression::StringLiteral(s) => {
            if s.starts_with("f\"") && s.ends_with("\"") {
                let content = &s[2..s.len() - 1];
                let js_template = process_formatted_string(content, aliases);
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
            "check" => Some("✓"),
            "close" | "x" => Some("×"),
            "menu" => Some("☰"),
            "search" => Some("⌕"),
            "arrow" | "arrow-right" => Some("→"),
            "arrow-left" => Some("←"),
            "plus" => Some("+"),
            "minus" => Some("-"),
            "star" => Some("★"),
            _ => None,
        };
        if let Some(symbol) = fallback {
            format!(
                "<span class=\"{}\" aria-hidden=\"true\">{}</span>",
                class_name, symbol
            )
        } else if raw.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            format!(
                "<iconify-icon class=\"{}\" icon=\"heroicons:{}\" aria-hidden=\"true\"></iconify-icon>",
                class_name, raw
            )
        } else {
            format!(
                "<span class=\"{}\" aria-hidden=\"true\">{}</span>",
                class_name, raw
            )
        }
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
            let mut event_attrs = String::new();
            for (key, expr) in attributes {
                if event_keys.contains(&key.as_str()) {
                    let code = match expr {
                        Expression::StringLiteral(s) => s.clone(),
                        _ => compile_expression_to_js_scoped(expr, aliases),
                    };
                    let escaped = code.replace('&', "&amp;")
                                      .replace('"', "&quot;")
                                      .replace('<', "&lt;")
                                      .replace('>', "&gt;");
                    event_attrs.push_str(&format!(
                        " x-on:{}=\"{}\"",
                        key,
                        escaped
                    ));
                }
            }

            let content = format!("{}<span>{}</span>", icon_markup, body);
            if href.is_empty() {
                Some(format!(
                    "<button class=\"{}\" type=\"button\"{}>{}</button>",
                    standard_class(&format!("amana-btn amana-btn-{} amana-btn-{} amana-btn-intent-{}", variant, size, intent), classes),
                    event_attrs,
                    content
                ))
            } else {
                Some(format!(
                    "<a class=\"{}\" href=\"{}\"{}>{}</a>",
                    standard_class(&format!("amana-btn amana-btn-{} amana-btn-{} amana-btn-intent-{}", variant, size, intent), classes),
                    href,
                    event_attrs,
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
            let variant = attr("variant", "elevated");
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
                standard_class(&format!("amana-card{} amana-card-variant-{} amana-card-density-{}", kind, variant, density), classes),
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
        "Center" => {
            let max_width = attr("max_width", "");
            let style = if max_width.is_empty() {
                "".to_string()
            } else {
                format!(" style=\"--center-max-width: {};\"", max_width)
            };
            Some(format!(
                "<div class=\"{}\"{}>{}</div>",
                standard_class("amana-center", classes),
                style,
                inner
            ))
        }
        "Cover" => {
            let min_height = attr("min_height", "100vh");
            let style = format!(" style=\"--cover-min-height: {};\"", min_height);
            Some(format!(
                "<div class=\"{}\"{}>{}</div>",
                standard_class("amana-cover", classes),
                style,
                inner
            ))
        }
        "Reel" => {
            let gap = attr("gap", "1.5rem");
            let style = format!(" style=\"--reel-gap: {};\"", gap);
            Some(format!(
                "<div class=\"{}\"{}>{}</div>",
                standard_class("amana-reel", classes),
                style,
                inner
            ))
        }
        "Masonry" => {
            let columns = attr("columns", "3");
            let gap = attr("gap", "1.5rem");
            let style = format!(" style=\"--masonry-cols: {}; --masonry-gap: {};\"", columns, gap);
            Some(format!(
                "<div class=\"{}\"{}>{}</div>",
                standard_class("amana-masonry", classes),
                style,
                inner
            ))
        }
        "Skeleton" => {
            let kind = attr("type", "text");
            let lines_str = attr("lines", "1");
            let lines = lines_str.parse::<u32>().unwrap_or(1);
            let content = if kind == "text" {
                (0..lines)
                    .map(|i| {
                        let width = match i % 3 {
                            0 => "width: 80%;",
                            1 => "width: 95%;",
                            _ => "width: 65%;",
                        };
                        format!("<div class=\"amana-skeleton-line\" style=\"{}\"></div>", width)
                    })
                    .collect::<String>()
            } else if kind == "card" {
                format!(
                    "<div class=\"amana-skeleton-image\"></div>\
                     <div class=\"amana-skeleton-line\" style=\"width: 60%; margin-top: 1rem;\"></div>\
                     <div class=\"amana-skeleton-line\" style=\"width: 85%;\"></div>"
                )
            } else if kind == "avatar" {
                "<div class=\"amana-skeleton-avatar\"></div>".to_string()
            } else {
                String::new()
            };
            Some(format!(
                "<div class=\"{}\" aria-busy=\"true\" aria-label=\"جاري التحميل\">{}</div>",
                standard_class(&format!("amana-skeleton amana-skeleton-{}", kind), classes),
                content
            ))
        }
        "LoadingState" => {
            let rows_str = attr("rows", "3");
            let rows = rows_str.parse::<u32>().unwrap_or(3);
            let default_label = "جاري تحميل البيانات...";
            let label = if inner.is_empty() { default_label } else { &inner };
            
            let skeletons = (0..rows)
                .map(|i| {
                    let width = match i % 3 {
                        0 => "width: 75%;",
                        1 => "width: 90%;",
                        _ => "width: 60%;",
                    };
                    format!("<div class=\"amana-skeleton-line\" style=\"{}\"></div>", width)
                })
                .collect::<String>();

            Some(format!(
                "<div class=\"{}\" aria-busy=\"true\" aria-live=\"polite\">\
                   <div class=\"amana-loading-header\">\
                     <div class=\"amana-loading-spinner\" aria-hidden=\"true\"></div>\
                     <div class=\"amana-loading-text\">{}</div>\
                   </div>\
                   <div class=\"amana-loading-body\">{}</div>\
                 </div>",
                standard_class("amana-loading-state", classes),
                label,
                skeletons
            ))
        }
        "ErrorState" => {
            let code = attr("code", "");
            let title = attr("title", "حدث خطأ ما");
            let description = attr("description", "يرجى المحاولة مرة أخرى لاحقاً");
            let action_label = attr("action_label", "");
            let action_href = attr("action_href", "#");
            
            let code_markup = if code.is_empty() {
                String::new()
            } else {
                format!("<span class=\"amana-error-code\">{}</span>", code)
            };
            
            let action_markup = if action_label.is_empty() {
                String::new()
            } else {
                format!(
                    "<a class=\"amana-btn amana-btn-primary\" href=\"{}\">{}</a>",
                    action_href, action_label
                )
            };
            
            Some(format!(
                "<section class=\"{}\" role=\"alert\">\
                   <div class=\"amana-error-state-icon-wrapper\" aria-hidden=\"true\">\
                     <iconify-icon icon=\"heroicons:exclamation-triangle\"></iconify-icon>\
                   </div>\
                   {}{}\
                   <h2>{}</h2>\
                   <p class=\"amana-muted\">{}</p>\
                   {}\
                 </section>",
                standard_class("amana-error-state", classes),
                code_markup,
                inner,
                title,
                description,
                action_markup
            ))
        }
        "OfflineState" => {
            let title = attr("title", "لا يوجد اتصال بالشبكة");
            let description = attr("description", "يرجى التحقق من اتصال الإنترنت الخاص بك.");
            let force = attr("force", "false");
            let x_data_attr = if force == "true" {
                "".to_string()
            } else {
                " x-data=\"{ online: navigator.onLine }\" @online.window=\"online = true\" @offline.window=\"online = false\" x-show=\"!online\" x-transition style=\"display: none;\"".to_string()
            };
            Some(format!(
                "<div class=\"{}\"{} role=\"alert\">\
                   <div class=\"amana-offline-icon-wrapper\" aria-hidden=\"true\">\
                     <iconify-icon icon=\"heroicons:wifi\"></iconify-icon>\
                   </div>\
                   <h2>{}</h2>\
                   <p class=\"amana-muted\">{}</p>\
                   {}\
                 </div>",
                standard_class("amana-offline-state", classes),
                x_data_attr,
                title,
                description,
                inner
            ))
        }
        "Toast" => {
            let message = attr("message", "");
            let tone = attr("tone", "success");
            let show_var = attr("show", "toastShow");
            let duration = attr("duration", "3000");
            let body = if message.is_empty() { inner } else { message };

            let icon = match tone.as_str() {
                "success" => "check-circle",
                "info" => "information-circle",
                "warning" => "exclamation-triangle",
                "danger" => "x-circle",
                _ => "check-circle"
            };

            Some(format!(
                "<div class=\"amana-toast-wrapper\" x-data=\"{{ show: false }}\" \
                      x-init=\"$watch('{}', value => {{ show = value; if (value) {{ setTimeout(() => {{ show = false; {} = false; }}, {}); }} }})\" \
                      x-show=\"show\" x-transition:enter=\"transition ease-out duration-300\" \
                      x-transition:enter-start=\"opacity-0 translate-y-2\" \
                      x-transition:enter-end=\"opacity-100 translate-y-0\" \
                      x-transition:leave=\"transition ease-in duration-200\" \
                      x-transition:leave-start=\"opacity-100 translate-y-0\" \
                      x-transition:leave-end=\"opacity-0 translate-y-2\" \
                      style=\"display: none;\" role=\"status\" aria-live=\"polite\">\
                   <div class=\"{}\">\
                     <iconify-icon class=\"amana-toast-icon\" icon=\"heroicons:{}\" aria-hidden=\"true\"></iconify-icon>\
                     <span class=\"amana-toast-message\">{}</span>\
                   </div>\
                 </div>",
                show_var, show_var, duration,
                standard_class(&format!("amana-toast amana-toast-{}", tone), classes),
                icon,
                body
            ))
        }
        "Banner" => {
            let tone = attr("tone", "info");
            let closable_str = attr("closable", "true");
            let closable = closable_str == "true" || closable_str == "yes" || closable_str == "1";
            
            let icon = match tone.as_str() {
                "success" => "check-circle",
                "info" => "information-circle",
                "warning" => "exclamation-triangle",
                "danger" => "x-circle",
                _ => "information-circle"
            };

            let (x_data, x_show, close_button) = if closable {
                (
                    " x-data=\"{ show: true }\"",
                    " x-show=\"show\" x-transition",
                    "<button type=\"button\" class=\"amana-banner-close\" @click=\"show = false\" aria-label=\"إغلاق\">\
                       <iconify-icon icon=\"heroicons:x-mark\" aria-hidden=\"true\"></iconify-icon>\
                     </button>"
                )
            } else {
                ("", "", "")
            };

            Some(format!(
                "<div class=\"{}\"{}{} role=\"alert\">\
                   <iconify-icon class=\"amana-banner-icon\" icon=\"heroicons:{}\" aria-hidden=\"true\"></iconify-icon>\
                   <div class=\"amana-banner-content\">{}</div>\
                   {}\
                 </div>",
                standard_class(&format!("amana-banner amana-banner-{}", tone), classes),
                x_data,
                x_show,
                icon,
                inner,
                close_button
            ))
        }
        "DashboardShell" => {
            let brand = attr("brand", "أمانة");
            let user = attr("user", "");
            let logo = attr("logo", "");
            
            let mut sidebar_html = String::new();
            let mut main_html = String::new();
            
            for child in children {
                match child {
                    ViewElement::Element { tag, children: sub_children, .. } if tag == "sidebar" => {
                        for sub_child in sub_children {
                            sidebar_html.push_str(&generate_ejs_scoped(sub_child, client_states, aliases));
                        }
                    }
                    ViewElement::Element { tag, children: sub_children, .. } if tag == "main" => {
                        for sub_child in sub_children {
                            main_html.push_str(&generate_ejs_scoped(sub_child, client_states, aliases));
                        }
                    }
                    _ => {
                        main_html.push_str(&generate_ejs_scoped(child, client_states, aliases));
                    }
                }
            }
            
            let logo_markup = if logo.is_empty() {
                String::new()
            } else {
                format!("<img class=\"amana-db-logo\" src=\"{}\" alt=\"Logo\" />", logo)
            };
            
            Some(format!(
                "<div class=\"{}\" x-data=\"{{ sidebarOpen: false }}\">\
                   <header class=\"amana-db-navbar\">\
                     <button class=\"amana-db-toggle\" @click=\"sidebarOpen = !sidebarOpen\" aria-label=\"فتح القائمة\">\
                       <iconify-icon icon=\"heroicons:bars-3\"></iconify-icon>\
                     </button>\
                     <div class=\"amana-db-brand\">\
                       {}\
                       <span>{}</span>\
                     </div>\
                     <div class=\"amana-db-user\">\
                       {}\
                     </div>\
                   </header>\
                   <div class=\"amana-db-container\">\
                     <aside class=\"amana-db-sidebar\" :class=\"{{ 'open': sidebarOpen }}\">\
                       <div class=\"amana-db-sidebar-backdrop\" @click=\"sidebarOpen = false\"></div>\
                       <nav class=\"amana-db-sidebar-nav\">\
                         {}\
                       </nav>\
                     </aside>\
                     <main class=\"amana-db-main\">\
                       {}\
                     </main>\
                   </div>\
                 </div>",
                standard_class("amana-dashboard-shell", classes),
                logo_markup,
                brand,
                user,
                sidebar_html,
                main_html
            ))
        }
        "AuthPage" => {
            let kind = attr("type", "login");
            let title = attr("title", if kind == "signup" { "إنشاء حساب جديد" } else { "تسجيل الدخول" });
            let logo = attr("logo", "");
            
            let logo_markup = if logo.is_empty() {
                String::new()
            } else {
                format!("<img class=\"amana-auth-logo\" src=\"{}\" alt=\"Logo\" />", logo)
            };
            
            let form_body = if inner.is_empty() {
                if kind == "signup" {
                    format!(
                        "<form class=\"amana-auth-form\" method=\"POST\" action=\"/signup\">\
                           <div class=\"amana-field\">\
                             <label class=\"amana-label\">الاسم الكامل</label>\
                             <input class=\"amana-input\" type=\"text\" name=\"name\" required />\
                           </div>\
                           <div class=\"amana-field\">\
                             <label class=\"amana-label\">البريد الإلكتروني</label>\
                             <input class=\"amana-input\" type=\"email\" name=\"email\" required />\
                           </div>\
                           <div class=\"amana-field\">\
                             <label class=\"amana-label\">كلمة المرور</label>\
                             <input class=\"amana-input\" type=\"password\" name=\"password\" required />\
                           </div>\
                           <button class=\"amana-btn amana-btn-primary amana-btn-md amana-btn-intent-default\" type=\"submit\">\
                             <span>إنشاء الحساب</span>\
                           </button>\
                         </form>"
                    )
                } else {
                    format!(
                        "<form class=\"amana-auth-form\" method=\"POST\" action=\"/login\">\
                           <div class=\"amana-field\">\
                             <label class=\"amana-label\">البريد الإلكتروني</label>\
                             <input class=\"amana-input\" type=\"email\" name=\"email\" required />\
                           </div>\
                           <div class=\"amana-field\">\
                             <label class=\"amana-label\">كلمة المرور</label>\
                             <input class=\"amana-input\" type=\"password\" name=\"password\" required />\
                           </div>\
                           <button class=\"amana-btn amana-btn-primary amana-btn-md amana-btn-intent-default\" type=\"submit\">\
                             <span>دخول</span>\
                           </button>\
                         </form>"
                    )
                }
            } else {
                inner
            };
            
            Some(format!(
                "<div class=\"{}\">\
                   <div class=\"amana-auth-card\">\
                     <div class=\"amana-auth-header\">\
                       {}\
                       <h2>{}</h2>\
                     </div>\
                     <div class=\"amana-auth-body\">\
                       {}\
                     </div>\
                   </div>\
                 </div>",
                standard_class("amana-auth-page", classes),
                logo_markup,
                title,
                form_body
            ))
        }
        "PricingSection" => {
            let title = attr("title", "اختر خطتك المناسبة");
            let billing = attr("billing", "monthly");
            
            let mut plans_html = String::new();
            
            for child in children {
                match child {
                    ViewElement::Element { tag, attributes: plan_attrs, children: plan_features, .. } if tag == "plan" => {
                        let plan_attr = |name: &str, fallback: &str| get_attr_scoped(plan_attrs, name, fallback, aliases);
                        let name = plan_attr("name", "باقة");
                        let price = plan_attr("price", "0");
                        let yearly_price = plan_attr("yearly_price", &price);
                        let highlight = plan_attr("highlight", "false");
                        let cta = plan_attr("cta", "اشترك الآن");
                        let cta_href = plan_attr("cta_href", "#");
                        
                        let is_highlighted = highlight == "true" || highlight == "yes" || highlight == "1";
                        let card_class = if is_highlighted {
                            "amana-pricing-card amana-pricing-card-highlighted"
                        } else {
                            "amana-pricing-card"
                        };
                        
                        let mut features_html = String::new();
                        for feat in plan_features {
                            match feat {
                                ViewElement::Element { tag: feat_tag, children: feat_children, .. } if feat_tag == "feature" => {
                                    let feat_text = feat_children.iter().map(|c| generate_ejs_scoped(c, client_states, aliases)).collect::<String>();
                                    features_html.push_str(&format!(
                                        "<li class=\"amana-pricing-feature\">\
                                           <iconify-icon icon=\"heroicons:check\" aria-hidden=\"true\"></iconify-icon>\
                                           <span>{}</span>\
                                         </li>",
                                        feat_text
                                    ));
                                }
                                _ => {
                                    features_html.push_str(&generate_ejs_scoped(feat, client_states, aliases));
                                }
                            }
                        }
                        
                        plans_html.push_str(&format!(
                            "<div class=\"{}\">\
                               <div class=\"amana-pricing-card-header\">\
                                 <h3>{}</h3>\
                               </div>\
                               <div class=\"amana-pricing-card-price\">\
                                 <span class=\"amana-price-value\" x-show=\"billing === 'monthly'\">{}</span>\
                                 <span class=\"amana-price-value\" x-show=\"billing === 'yearly'\">{}</span>\
                               </div>\
                               <ul class=\"amana-pricing-card-features\">\
                                 {}\
                               </ul>\
                               <div class=\"amana-pricing-card-cta\">\
                                 <a class=\"amana-btn amana-btn-primary amana-btn-md amana-btn-intent-default\" href=\"{}\">{}</a>\
                               </div>\
                             </div>",
                            card_class,
                            name,
                            price,
                            yearly_price,
                            features_html,
                            cta_href,
                            cta
                        ));
                    }
                    _ => {
                        plans_html.push_str(&generate_ejs_scoped(child, client_states, aliases));
                    }
                }
            }
            
            Some(format!(
                "<section class=\"{}\" x-data=\"{{ billing: '{}' }}\">\
                   <div class=\"amana-pricing-section-header\">\
                     <h2>{}</h2>\
                     <div class=\"amana-pricing-billing-toggle\">\
                       <button type=\"button\" :class=\"{{ 'active': billing === 'monthly' }}\" @click=\"billing = 'monthly'\">شهرياً</button>\
                       <button type=\"button\" :class=\"{{ 'active': billing === 'yearly' }}\" @click=\"billing = 'yearly'\">سنوياً</button>\
                     </div>\
                   </div>\
                   <div class=\"amana-pricing-section-grid\">\
                     {}\
                   </div>\
                 </section>",
                standard_class("amana-pricing-section", classes),
                billing,
                title,
                plans_html
            ))
        }
        "Breadcrumb" => {
            let mut items_html = String::new();
            let len = children.len();
            for (i, child) in children.iter().enumerate() {
                let child_html = generate_ejs_scoped(child, client_states, aliases);
                if i == len - 1 {
                    items_html.push_str(&format!(
                        "<li class=\"amana-breadcrumb-item amana-breadcrumb-current\" aria-current=\"page\">{}</li>",
                        child_html
                    ));
                } else {
                    items_html.push_str(&format!(
                        "<li class=\"amana-breadcrumb-item\">{}</li>",
                        child_html
                    ));
                    items_html.push_str("<li class=\"amana-breadcrumb-sep\" aria-hidden=\"true\">/</li>");
                }
            }
            Some(format!(
                "<nav class=\"{}\" aria-label=\"Breadcrumb\">\
                   <ol class=\"amana-breadcrumb-list\">\
                     {}\
                   </ol>\
                 </nav>",
                standard_class("amana-breadcrumb", classes),
                items_html
            ))
        }
        "Dropdown" => {
            let trigger = attr("trigger", "خيارات");
            let icon = attr("icon", "chevron-down");
            let align = attr("align", "right");
            
            let icon_markup = if icon.is_empty() {
                String::new()
            } else {
                render_icon(&icon, "amana-dropdown-trigger-icon")
            };
            
            let mut menu_items = String::new();
            for child in children {
                let item_html = generate_ejs_scoped(child, client_states, aliases);
                menu_items.push_str(&format!(
                    "<div role=\"none\" class=\"amana-dropdown-item-wrapper\">{}</div>",
                    item_html
                ));
            }
            
            Some(format!(
                "<div class=\"{}\" x-data=\"{{ open: false }}\" @keydown.escape.window=\"open = false\" style=\"position: relative; display: inline-block;\">\
                   <button class=\"amana-dropdown-trigger amana-btn amana-btn-secondary amana-btn-md amana-btn-intent-default\" \
                           @click=\"open = !open\" :aria-expanded=\"open\" aria-haspopup=\"true\" type=\"button\">\
                     <span>{}</span>\
                     {}\
                   </button>\
                   <div class=\"amana-dropdown-menu amana-dropdown-align-{}\" x-show=\"open\" \
                        @click.away=\"open = false\" role=\"menu\" x-transition \
                        style=\"display: none; position: absolute; z-index: var(--z-dropdown, 60);\">\
                     {}\
                   </div>\
                 </div>",
                standard_class("amana-dropdown", classes),
                trigger,
                icon_markup,
                align,
                menu_items
            ))
        }
        "CommandPalette" => {
            let open_var = attr("open", "paletteOpen");
            let placeholder = attr("placeholder", "ابحث عن أي شيء...");
            
            Some(format!(
                "<div class=\"amana-command-palette-backdrop\" x-show=\"{}\" \
                      x-transition @keydown.escape.window=\"{} = false\" \
                      style=\"display: none; position: fixed; inset: 0; z-index: var(--z-modal, 100);\" role=\"dialog\" aria-modal=\"true\">\
                   <div class=\"{}\" @click.away=\"{} = false\">\
                     <div class=\"amana-cp-header\">\
                       <iconify-icon icon=\"heroicons:magnifying-glass\" class=\"amana-cp-search-icon\" aria-hidden=\"true\"></iconify-icon>\
                       <input class=\"amana-cp-input\" type=\"text\" placeholder=\"{}\" \
                              x-ref=\"cpInput\" x-init=\"$watch('{}', v => {{ if (v) $nextTick(() => $refs.cpInput.focus()); }})\" />\
                       <button class=\"amana-cp-close\" @click=\"{} = false\" aria-label=\"إغلاق\">\
                         <iconify-icon icon=\"heroicons:x-mark\" aria-hidden=\"true\"></iconify-icon>\
                       </button>\
                     </div>\
                     <div class=\"amana-cp-body\">\
                       {}\
                       <div class=\"amana-cp-no-results\" style=\"display: none;\">\
                         <iconify-icon icon=\"heroicons:face-frown\" aria-hidden=\"true\"></iconify-icon>\
                         <p>لا توجد نتائج تطابق بحثك</p>\
                       </div>\
                     </div>\
                     <div class=\"amana-cp-footer\">\
                       <div class=\"amana-cp-shortcuts\">\
                         <span><kbd>↑↓</kbd> للتنقل</span>\
                         <span><kbd>Enter</kbd> للاختيار</span>\
                         <span><kbd>ESC</kbd> للإغلاق</span>\
                       </div>\
                     </div>\
                   </div>\
                 </div>",
                open_var, open_var,
                standard_class("amana-command-palette", classes),
                open_var,
                placeholder,
                open_var,
                open_var,
                if inner.is_empty() {
                    format!(
                        "<ul class=\"amana-cp-results\" role=\"listbox\">\
                           <li class=\"amana-cp-item\" role=\"option\" tabindex=\"0\">\
                             <iconify-icon icon=\"heroicons:home\" aria-hidden=\"true\"></iconify-icon>\
                             <span>لوحة التحكم الرئيسية</span>\
                           </li>\
                           <li class=\"amana-cp-item\" role=\"option\" tabindex=\"0\">\
                             <iconify-icon icon=\"heroicons:shopping-cart\" aria-hidden=\"true\"></iconify-icon>\
                             <span>إدارة الطلبات والمبيعات</span>\
                           </li>\
                           <li class=\"amana-cp-item\" role=\"option\" tabindex=\"0\">\
                             <iconify-icon icon=\"heroicons:cog-6-tooth\" aria-hidden=\"true\"></iconify-icon>\
                             <span>إعدادات النظام والحساب</span>\
                           </li>\
                         </ul>"
                    )
                } else {
                    inner
                }
            ))
        }
        "SearchBar" => {
            let placeholder = attr("placeholder", "بحث...");
            let action = attr("action", "");
            let name = attr("name", "q");
            let shortcut = attr("shortcut", "");
            
            let shortcut_kbd = if shortcut.is_empty() {
                String::new()
            } else {
                let key_label = shortcut.to_uppercase();
                format!(
                    "<kbd class=\"amana-searchbar-shortcut\" aria-hidden=\"true\">⌘{}</kbd>",
                    key_label
                )
            };
            
            let x_init = if shortcut.is_empty() {
                String::new()
            } else {
                format!(
                    " @keydown.meta.{}.window.prevent=\"$refs.searchField.focus()\"",
                    shortcut.to_lowercase()
                )
            };

            Some(format!(
                "<form class=\"{}\" action=\"{}\" method=\"GET\" role=\"search\"{}>\
                   <iconify-icon icon=\"heroicons:magnifying-glass\" class=\"amana-searchbar-icon\" aria-hidden=\"true\"></iconify-icon>\
                   <input class=\"amana-searchbar-input\" type=\"search\" name=\"{}\" placeholder=\"{}\" x-ref=\"searchField\" />\
                   {}\
                 </form>",
                standard_class("amana-searchbar", classes),
                action,
                x_init,
                name,
                placeholder,
                shortcut_kbd
            ))
        }
        "FilterBar" => {
            Some(format!(
                "<div class=\"{}\">{}</div>",
                standard_class("amana-filterbar", classes),
                inner
            ))
        }
        "Paginator" => {
            let page_var = attr("page", "page");
            let total_pages_var = attr("total_pages", "totalPages");
            
            Some(format!(
                "<nav class=\"{}\" aria-label=\"التنقل بين الصفحات\">\
                   <button type=\"button\" class=\"amana-page-btn\" :disabled=\"{} === 1\" @click=\"{}--\" aria-label=\"الصفحة السابقة\">\
                     <iconify-icon icon=\"heroicons:chevron-right\" aria-hidden=\"true\"></iconify-icon>\
                   </button>\
                   <span class=\"amana-page-info\" aria-live=\"polite\">\
                     الصفحة <strong x-text=\"{}\"></strong> من <strong x-text=\"{}\"></strong>\
                   </span>\
                   <button type=\"button\" class=\"amana-page-btn\" :disabled=\"{} === {}\" @click=\"{}++\" aria-label=\"الصفحة التالية\">\
                     <iconify-icon icon=\"heroicons:chevron-left\" aria-hidden=\"true\"></iconify-icon>\
                   </button>\
                 </nav>",
                standard_class("amana-paginator", classes),
                page_var, page_var,
                page_var, total_pages_var,
                page_var, total_pages_var, page_var
            ))
        }
        "DataTable" => {
            let data_expr = attributes
                .iter()
                .find(|(k, _)| k == "data" || k == "value" || k == "items")
                .map(|(_, expr)| compile_expression_to_js_scoped(expr, aliases))
                .unwrap_or_else(|| "items".to_string());
                
            let as_var = attr("as", &if data_expr.ends_with('s') {
                data_expr[..data_expr.len() - 1].to_string()
            } else {
                "item".to_string()
            });
            
            let searchable_str = attr("searchable", "false");
            let searchable = searchable_str == "true" || searchable_str == "yes" || searchable_str == "1";
            
            let selectable_str = attr("selectable", "false");
            let selectable = selectable_str == "true" || selectable_str == "yes" || selectable_str == "1";
            
            let mut headers_html = String::new();
            let mut cells_html = String::new();
            
            for child in children {
                match child {
                    ViewElement::Element { tag, attributes: col_attrs, children: col_children, .. } if tag == "column" => {
                        let col_attr = |name: &str, fallback: &str| get_attr_scoped(col_attrs, name, fallback, aliases);
                        let title = col_attr("title", &col_attr("name", "عمود"));
                        
                        headers_html.push_str(&format!("<th>{}</th>", title));
                        
                        let cell_content = col_children.iter().map(|c| generate_ejs_scoped(c, client_states, aliases)).collect::<String>();
                        cells_html.push_str(&format!("<td>{}</td>", cell_content));
                    }
                    _ => {}
                }
            }
            
            let search_ui = if searchable {
                format!(
                    "<div class=\"amana-table-search-wrapper\">\
                       <iconify-icon icon=\"heroicons:magnifying-glass\" aria-hidden=\"true\"></iconify-icon>\
                       <input type=\"search\" class=\"amana-table-search-input\" placeholder=\"بحث في الجدول...\" x-model=\"search\" />\
                     </div>"
                )
            } else {
                String::new()
            };
            
            let select_header = if selectable {
                "<th class=\"amana-table-select-col\"><input type=\"checkbox\" @change=\"toggleAll($el.checked)\" /></th>"
            } else {
                ""
            };
            
            let select_cell = if selectable {
                format!("<td><input type=\"checkbox\" :value=\"{}.id || {}.name || ''\" x-model=\"selected\" /></td>", as_var, as_var)
            } else {
                String::new()
            };
            
            let row_tr = if searchable {
                format!("<tr x-show=\"Object.values({}).some(v => String(v).toLowerCase().includes(search.toLowerCase()))\">", as_var)
            } else {
                "<tr>".to_string()
            };

            Some(format!(
                "<% if (typeof {} !== 'undefined' && {} && {}.length > 0) {{ %>\
                   <div class=\"{}\" x-data=\"{{ \
                       search: '', \
                       selected: [], \
                       toggleAll(checked) {{ this.selected = checked ? <%- JSON.stringify({}.map(row => row.id || row.name || '')) %> : []; }} \
                   }}\">\
                     {}\
                     <div class=\"amana-table-responsive\">\
                       <table class=\"amana-table\">\
                         <thead>\
                           <tr>\
                             {}\
                             {}\
                           </tr>\
                         </thead>\
                         <tbody>\
                           <% for (let {} of {}) {{ %>\
                             {}\
                               {}\
                               {}\
                             </tr>\
                           <% }} %>\
                         </tbody>\
                       </table>\
                     </div>\
                   </div>\
                 <% }} else {{ %>\
                   <div class=\"amana-table-empty\">\
                     <iconify-icon icon=\"heroicons:inbox\" aria-hidden=\"true\"></iconify-icon>\
                     <p>لا توجد بيانات متوفرة في الجدول</p>\
                   </div>\
                 <% }} %>",
                data_expr, data_expr, data_expr,
                standard_class("amana-datatable-container", classes),
                data_expr,
                search_ui,
                select_header,
                headers_html,
                as_var, data_expr,
                row_tr,
                select_cell,
                cells_html
            ))
        }
        "FileUpload" => {
            let name = attr("name", "file");
            let accept = attr("accept", "*/*");
            let max_size = attr("max_size", "10MB");
            let label = attr("label", "رفع ملف");
            let multiple_str = attr("multiple", "false");
            let multiple = multiple_str == "true" || multiple_str == "yes" || multiple_str == "1";
            let multiple_attr = if multiple { "multiple" } else { "" };
            let id = attr("id", &format!("upload_{}", name));

            Some(format!(
                "<div class=\"{}\" x-data=\"{{ \
                    isDragOver: false, \
                    files: [], \
                    parseSize(sizeStr) {{ \
                        if (!sizeStr) return 0; \
                        const match = sizeStr.match(/^(\\d+(?:\\.\\d+)?)\\s*([a-zA-Z]+)$/); \
                        if (!match) return 0; \
                        const num = parseFloat(match[1]); \
                        const unit = match[2].toUpperCase(); \
                        switch (unit) {{ \
                            case 'KB': case 'K': return num * 1024; \
                            case 'MB': case 'M': return num * 1024 * 1024; \
                            case 'GB': case 'G': return num * 1024 * 1024 * 1024; \
                            default: return num; \
                        }} \
                    }}, \
                    formatBytes(bytes) {{ \
                        if (bytes === 0) return '0 Bytes'; \
                        const k = 1024; \
                        const sizes = ['Bytes', 'KB', 'MB', 'GB']; \
                        const i = Math.floor(Math.log(bytes) / Math.log(k)); \
                        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]; \
                    }}, \
                    handleFiles(fileList) {{ \
                        const maxBytes = this.parseSize('{}'); \
                        const multiple = {}; \
                        let newFiles = Array.from(fileList); \
                        if (!multiple) {{ \
                            newFiles = newFiles.slice(0, 1); \
                            this.files = []; \
                        }} \
                        newFiles.forEach(file => {{ \
                            if (maxBytes && file.size > maxBytes) {{ \
                                alert('الملف ' + file.name + ' يتجاوز الحد الأقصى المسموح به {}'); \
                                return; \
                            }} \
                            let preview = ''; \
                            if (file.type.startsWith('image/')) {{ \
                                preview = URL.createObjectURL(file); \
                            }} \
                            this.files.push({{ name: file.name, size: this.formatBytes(file.size), type: file.type, preview: preview }}); \
                        }}); \
                    }}, \
                    removeFile(index) {{ \
                        if (this.files[index].preview) {{ \
                            URL.revokeObjectURL(this.files[index].preview); \
                        }} \
                        this.files.splice(index, 1); \
                    }} \
                }}\" \
                @dragover.prevent=\"isDragOver = true\" \
                @dragleave.prevent=\"isDragOver = false\" \
                @drop.prevent=\"isDragOver = false; handleFiles($event.dataTransfer.files)\">\
                  <label for=\"{}\" class=\"amana-fileupload-label\">{}</label>\
                  <div class=\"amana-fileupload-dropzone\" \
                       :class=\"{{ 'amana-fileupload-dropzone-active': isDragOver }}\" \
                       @click=\"$refs.fileInput.click()\" \
                       @keydown.enter.space.prevent=\"$refs.fileInput.click()\" \
                       tabindex=\"0\" \
                       role=\"button\" \
                       aria-label=\"{} - اسحب الملفات هنا أو انقر للاختيار\">\
                    <input type=\"file\" id=\"{}\" name=\"{}\" accept=\"{}\" x-ref=\"fileInput\" @change=\"handleFiles($event.target.files)\" style=\"display: none;\" {} />\
                    <div class=\"amana-fileupload-icon\">\
                      <iconify-icon icon=\"heroicons:cloud-arrow-up\" aria-hidden=\"true\"></iconify-icon>\
                    </div>\
                    <p class=\"amana-fileupload-text\">اسحب وأفلت الملفات هنا أو <span class=\"amana-fileupload-browse\">تصفح من جهازك</span></p>\
                    <p class=\"amana-fileupload-info\">الحد الأقصى لحجم الملف: {} - الامتدادات المقبولة: {}</p>\
                  </div>\
                  <template x-if=\"files.length > 0\">\
                    <ul class=\"amana-fileupload-preview-list\">\
                      <template x-for=\"(file, index) in files\" :key=\"index\">\
                        <li class=\"amana-fileupload-preview-item\">\
                          <template x-if=\"file.preview\">\
                            <img :src=\"file.preview\" class=\"amana-fileupload-preview-img\" alt=\"معاينة الصورة\" />\
                          </template>\
                          <template x-if=\"!file.preview\">\
                            <div class=\"amana-fileupload-preview-fallback\">\
                              <iconify-icon icon=\"heroicons:document\" aria-hidden=\"true\"></iconify-icon>\
                            </div>\
                          </template>\
                          <div class=\"amana-fileupload-preview-details\">\
                            <span class=\"amana-fileupload-preview-name\" x-text=\"file.name\"></span>\
                            <span class=\"amana-fileupload-preview-size\" x-text=\"file.size\"></span>\
                          </div>\
                          <button type=\"button\" class=\"amana-fileupload-preview-remove\" @click.stop=\"removeFile(index)\" aria-label=\"إزالة الملف\">\
                            <iconify-icon icon=\"heroicons:x-mark\" aria-hidden=\"true\"></iconify-icon>\
                          </button>\
                        </li>\
                      </template>\
                    </ul>\
                  </template>\
                </div>",
                standard_class("amana-fileupload", classes),
                max_size,
                multiple,
                max_size,
                id,
                label,
                label,
                id,
                name,
                accept,
                multiple_attr,
                max_size,
                accept
            ))
        }
        "RichEditor" => {
            let name = attr("name", "content");
            let label = attr("label", "المحتوى");
            let default_value = attr("value", &attr("default", ""));

            Some(format!(
                "<div class=\"{}\" x-data=\"{{ \
                    content: '', \
                    exec(cmd, arg = null) {{ \
                        document.execCommand(cmd, false, arg); \
                        this.updateContent(); \
                    }}, \
                    updateContent() {{ \
                        this.content = this.$refs.editor.innerHTML; \
                    }} \
                }}\" x-init=\"content = $refs.editor.innerHTML\">\
                  <label class=\"amana-richeditor-label\">{}</label>\
                  <div class=\"amana-richeditor-container\">\
                    <div class=\"amana-richeditor-toolbar\" role=\"toolbar\" aria-label=\"أدوات تنسيق النص\">\
                      <button type=\"button\" class=\"amana-richeditor-btn\" @click=\"exec('bold')\" title=\"عريض\" aria-label=\"تنسيق عريض\">\
                        <iconify-icon icon=\"lucide:bold\" aria-hidden=\"true\"></iconify-icon>\
                      </button>\
                      <button type=\"button\" class=\"amana-richeditor-btn\" @click=\"exec('italic')\" title=\"مائل\" aria-label=\"تنسيق مائل\">\
                        <iconify-icon icon=\"lucide:italic\" aria-hidden=\"true\"></iconify-icon>\
                      </button>\
                      <button type=\"button\" class=\"amana-richeditor-btn\" @click=\"exec('underline')\" title=\"مسطر\" aria-label=\"تنسيق مسطر\">\
                        <iconify-icon icon=\"lucide:underline\" aria-hidden=\"true\"></iconify-icon>\
                      </button>\
                      <button type=\"button\" class=\"amana-richeditor-btn\" @click=\"exec('strikeThrough')\" title=\"يتوسطه خط\" aria-label=\"يتوسطه خط\">\
                        <iconify-icon icon=\"lucide:strikethrough\" aria-hidden=\"true\"></iconify-icon>\
                      </button>\
                      <div class=\"amana-richeditor-separator\"></div>\
                      <button type=\"button\" class=\"amana-richeditor-btn\" @click=\"exec('formatBlock', 'h1')\" title=\"عنوان 1\" aria-label=\"عنوان رئيسي 1\">\
                        <span style=\"font-weight: bold; font-size: 14px;\">H1</span>\
                      </button>\
                      <button type=\"button\" class=\"amana-richeditor-btn\" @click=\"exec('formatBlock', 'h2')\" title=\"عنوان 2\" aria-label=\"عنوان رئيسي 2\">\
                        <span style=\"font-weight: bold; font-size: 12px;\">H2</span>\
                      </button>\
                      <button type=\"button\" class=\"amana-richeditor-btn\" @click=\"exec('formatBlock', 'blockquote')\" title=\"اقتباس\" aria-label=\"اقتباس\">\
                        <iconify-icon icon=\"lucide:quote\" aria-hidden=\"true\"></iconify-icon>\
                      </button>\
                      <div class=\"amana-richeditor-separator\"></div>\
                      <button type=\"button\" class=\"amana-richeditor-btn\" @click=\"exec('insertUnorderedList')\" title=\"قائمة منقطة\" aria-label=\"قائمة منقطة\">\
                        <iconify-icon icon=\"lucide:list\" aria-hidden=\"true\"></iconify-icon>\
                      </button>\
                      <button type=\"button\" class=\"amana-richeditor-btn\" @click=\"exec('insertOrderedList')\" title=\"قائمة مرقمة\" aria-label=\"قائمة مرقمة\">\
                        <iconify-icon icon=\"lucide:list-ordered\" aria-hidden=\"true\"></iconify-icon>\
                      </button>\
                      <div class=\"amana-richeditor-separator\"></div>\
                      <button type=\"button\" class=\"amana-richeditor-btn\" @click=\"exec('justifyRight')\" title=\"محاذاة لليمين\" aria-label=\"محاذاة لليمين\">\
                        <iconify-icon icon=\"lucide:align-right\" aria-hidden=\"true\"></iconify-icon>\
                      </button>\
                      <button type=\"button\" class=\"amana-richeditor-btn\" @click=\"exec('justifyCenter')\" title=\"محاذاة للوسط\" aria-label=\"محاذاة للوسط\">\
                        <iconify-icon icon=\"lucide:align-center\" aria-hidden=\"true\"></iconify-icon>\
                      </button>\
                      <button type=\"button\" class=\"amana-richeditor-btn\" @click=\"exec('justifyLeft')\" title=\"محاذاة لليسار\" aria-label=\"محاذاة لليسار\">\
                        <iconify-icon icon=\"lucide:align-left\" aria-hidden=\"true\"></iconify-icon>\
                      </button>\
                      <div class=\"amana-richeditor-separator\"></div>\
                      <button type=\"button\" class=\"amana-richeditor-btn\" @click=\"exec('removeFormat')\" title=\"مسح التنسيق\" aria-label=\"مسح التنسيق\">\
                        <iconify-icon icon=\"lucide:trash-2\" aria-hidden=\"true\"></iconify-icon>\
                      </button>\
                    </div>\
                    <div class=\"amana-richeditor-content\" \
                         contenteditable=\"true\" \
                         x-ref=\"editor\" \
                         @input=\"updateContent\" \
                         @blur=\"updateContent\" \
                         role=\"textbox\" \
                         aria-multiline=\"true\" \
                         aria-label=\"{}\">\
                      {}\
                    </div>\
                  </div>\
                  <input type=\"hidden\" name=\"{}\" :value=\"content\" />\
                </div>",
                standard_class("amana-richeditor", classes),
                label,
                label,
                if default_value.is_empty() { inner } else { default_value },
                name
            ))
        }
        "ColorPicker" => {
            let name = attr("name", "color");
            let label = attr("label", "اللون");
            let default_color = attr("default", "#6366f1");
            let id = attr("id", &format!("picker_{}", name));

            Some(format!(
                "<div class=\"{}\" x-data=\"{{ \
                    color: '{}', \
                    presets: ['#6366f1', '#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#ec4899', '#8b5cf6', '#1e293b'] \
                }}\">\
                  <label class=\"amana-colorpicker-label\" for=\"{}\">{}</label>\
                  <div class=\"amana-colorpicker-input-group\">\
                    <div class=\"amana-colorpicker-preview-wrapper\" :style=\"'background-color: ' + color\" @click=\"$refs.nativePicker.click()\">\
                      <input type=\"color\" id=\"{}\" name=\"{}\" x-ref=\"nativePicker\" x-model=\"color\" class=\"amana-colorpicker-native\" />\
                    </div>\
                    <input type=\"text\" class=\"amana-colorpicker-text-input\" x-model=\"color\" pattern=\"^#([A-Fa-f0-9]{{6}})$\" placeholder=\"#000000\" aria-label=\"رمز اللون بصيغة Hex\" />\
                  </div>\
                  <div class=\"amana-colorpicker-presets\" aria-label=\"ألوان مقترحة\">\
                    <template x-for=\"preset in presets\" :key=\"preset\">\
                      <button type=\"button\" \
                              class=\"amana-colorpicker-preset-btn\" \
                              :style=\"'background-color: ' + preset\" \
                              :class=\"{{ 'amana-colorpicker-preset-active': color === preset }}\" \
                              @click=\"color = preset\" \
                              :aria-label=\"'اختر اللون ' + preset\">\
                      </button>\
                    </template>\
                  </div>\
                </div>",
                standard_class("amana-colorpicker", classes),
                default_color,
                id,
                label,
                id,
                name
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
        "HeroSection" => {
            let title = attr("title", "");
            let subtitle = attr("subtitle", "");
            let description = attr("description", &attr("copy", ""));
            let eyebrow = attr("eyebrow", "");
            let cta_label = attr("cta_label", "");
            let cta_href = attr("cta_href", "");
            let secondary_cta_label = attr("secondary_cta_label", "");
            let secondary_cta_href = attr("secondary_cta_href", "");
            let image = attr("image", &attr("media", ""));
            let align = attr("align", "center");

            let eyebrow_markup = if eyebrow.is_empty() {
                String::new()
            } else {
                format!("<span class=\"amana-hero-eyebrow\">{}</span>", eyebrow)
            };

            let cta_primary = if cta_label.is_empty() {
                String::new()
            } else {
                format!(
                    "<a class=\"amana-btn amana-btn-primary amana-btn-lg\" href=\"{}\">{}</a>",
                    if cta_href.is_empty() { "#" } else { &cta_href },
                    cta_label
                )
            };

            let cta_secondary = if secondary_cta_label.is_empty() {
                String::new()
            } else {
                format!(
                    "<a class=\"amana-btn amana-btn-outline amana-btn-lg\" href=\"{}\">{}</a>",
                    if secondary_cta_href.is_empty() { "#" } else { &secondary_cta_href },
                    secondary_cta_label
                )
            };

            let image_markup = if image.is_empty() {
                String::new()
            } else {
                format!(
                    "<div class=\"amana-hero-media\">\
                       <img src=\"{}\" alt=\"{}\" class=\"amana-hero-image\" />\
                     </div>",
                    image, title
                )
            };

            if image.is_empty() {
                Some(format!(
                    "<section class=\"{}\">\
                       <div class=\"amana-hero-center-container\">\
                         {}\
                         <h1 class=\"amana-hero-title\">{}</h1>\
                         <p class=\"amana-hero-subtitle\">{}</p>\
                         <p class=\"amana-hero-description\">{}</p>\
                         <div class=\"amana-hero-actions\">{} {}</div>\
                         {}\
                       </div>\
                     </section>",
                    standard_class("amana-hero-section amana-hero-align-center", classes),
                    eyebrow_markup, title, subtitle, description, cta_primary, cta_secondary, inner
                ))
            } else {
                let layout_class = if align == "right" {
                    "amana-hero-section amana-hero-align-right amana-hero-split"
                } else {
                    "amana-hero-section amana-hero-align-left amana-hero-split"
                };
                Some(format!(
                    "<section class=\"{}\">\
                       <div class=\"amana-hero-split-container\">\
                         <div class=\"amana-hero-content\">\
                           {}\
                           <h1 class=\"amana-hero-title\">{}</h1>\
                           <p class=\"amana-hero-subtitle\">{}</p>\
                           <p class=\"amana-hero-description\">{}</p>\
                           <div class=\"amana-hero-actions\">{} {}</div>\
                           {}\
                         </div>\
                         {}\
                       </div>\
                     </section>",
                    standard_class(layout_class, classes),
                    eyebrow_markup, title, subtitle, description, cta_primary, cta_secondary, inner, image_markup
                ))
            }
        }
        "SettingsPage" => {
            let title = attr("title", "إعدادات الحساب");
            let description = attr("description", "إدارة وتعديل إعدادات حسابك وتفضيلاتك");

            let mut nav_items = Vec::new();
            let mut sections_html = String::new();

            for (idx, child) in children.iter().enumerate() {
                match child {
                    ViewElement::Element { tag, attributes: sec_attrs, children: sec_children, .. } if tag == "section" || tag == "Section" => {
                        let sec_attr = |name: &str, fallback: &str| get_attr_scoped(sec_attrs, name, fallback, aliases);
                        let sec_title = sec_attr("title", &format!("القسم {}", idx + 1));
                        let sec_desc = sec_attr("description", "");
                        let sec_id = format!("settings_sec_{}", idx);

                        nav_items.push((sec_id.clone(), sec_title.clone()));

                        let sec_inner = sec_children.iter().map(|c| generate_ejs_scoped(c, client_states, aliases)).collect::<String>();

                        sections_html.push_str(&format!(
                            "<section id=\"{}\" class=\"amana-settings-section\">\
                               <div class=\"amana-settings-section-info\">\
                                 <h3>{}</h3>\
                                 <p>{}</p>\
                               </div>\
                               <div class=\"amana-settings-section-fields\">\
                                 {}\
                               </div>\
                             </section>",
                            sec_id, sec_title, sec_desc, sec_inner
                        ));
                    }
                    _ => {
                        let direct_content = generate_ejs_scoped(child, client_states, aliases);
                        sections_html.push_str(&direct_content);
                    }
                }
            }

            let mut nav_links_html = String::new();
            for (id, name) in &nav_items {
                nav_links_html.push_str(&format!(
                    "<a href=\"#{}\" class=\"amana-settings-link\" :class=\"{{ 'active': activeSection === '{}' }}\" @click=\"activeSection = '{}'\">{}</a>",
                    id, id, id, name
                ));
            }

            let active_sec_init = if nav_items.is_empty() {
                "''".to_string()
            } else {
                format!("'{}'", nav_items[0].0)
            };

            Some(format!(
                "<div class=\"{}\" x-data=\"{{ activeSection: {} }}\">\
                   <div class=\"amana-settings-header\">\
                     <h2>{}</h2>\
                     <p>{}</p>\
                   </div>\
                   <div class=\"amana-settings-layout\">\
                     <aside class=\"amana-settings-sidebar\">\
                       <nav class=\"amana-settings-nav\">\
                         {}\
                       </nav>\
                     </aside>\
                     <main class=\"amana-settings-main\">\
                       {}\
                     </main>\
                   </div>\
                 </div>",
                standard_class("amana-settings-page", classes),
                active_sec_init,
                title,
                description,
                nav_links_html,
                sections_html
            ))
        }
        "StatsSection" => {
            let title = attr("title", "");
            let description = attr("description", "");

            let header = if !title.is_empty() || !description.is_empty() {
                format!(
                    "<header class=\"amana-section-head\">\
                       <h2>{}</h2>\
                       <p class=\"amana-section-copy\">{}</p>\
                     </header>",
                    title, description
                )
            } else {
                String::new()
            };

            Some(format!(
                "<section class=\"{}\">\
                   {}\
                   <div class=\"amana-stats-grid\">\
                     {}\
                   </div>\
                 </section>",
                standard_class("amana-stats-section", classes),
                header,
                inner
            ))
        }
        "FAQSection" => {
            let title = attr("title", "الأسئلة الشائعة");
            let description = attr("description", "ابحث عن إجابات سريعة للأسئلة الأكثر تكراراً");

            Some(format!(
                "<section class=\"{}\">\
                   <div class=\"amana-section-head\">\
                     <h2>{}</h2>\
                     <p class=\"amana-section-copy\">{}</p>\
                   </div>\
                   <div class=\"amana-faq-group\">\
                     {}\
                   </div>\
                 </section>",
                standard_class("amana-faq-section", classes),
                title,
                description,
                inner
            ))
        }
        "BlogSection" => {
            let title = attr("title", "آخر الأخبار والمقالات");
            let description = attr("description", "تابع أحدث المستجدات والنصائح من فريقنا");
            let view_all_label = attr("view_all_label", "عرض جميع المقالات");
            let view_all_href = attr("view_all_href", "");

            let view_all_link = if view_all_href.is_empty() {
                String::new()
            } else {
                format!(
                    "<a href=\"{}\" class=\"amana-blog-view-all\">\
                       <span>{}</span>\
                       <iconify-icon icon=\"heroicons:arrow-left\" aria-hidden=\"true\"></iconify-icon>\
                     </a>",
                    view_all_href, view_all_label
                )
            };

            Some(format!(
                "<section class=\"{}\">\
                   <div class=\"amana-blog-header\">\
                     <div class=\"amana-blog-header-content\">\
                       <h2>{}</h2>\
                       <p class=\"amana-section-copy\">{}</p>\
                     </div>\
                     {}\
                   </div>\
                   <div class=\"amana-blog-grid\">\
                     {}\
                   </div>\
                 </section>",
                standard_class("amana-blog-section", classes),
                title,
                description,
                view_all_link,
                inner
            ))
        }
        "TestimonialsSection" => {
            let title = attr("title", "ماذا يقول شركاؤنا");
            let description = attr("description", "آراء وتجارب بعض عملائنا المتميزين");

            Some(format!(
                "<section class=\"{}\">\
                   <div class=\"amana-section-head\">\
                     <h2>{}</h2>\
                     <p class=\"amana-section-copy\">{}</p>\
                   </div>\
                   <div class=\"amana-testimonials-grid\">\
                     {}\
                   </div>\
                 </section>",
                standard_class("amana-testimonials-section", classes),
                title,
                description,
                inner
            ))
        }
        "ContactSection" => {
            let title = attr("title", "تواصل معنا");
            let description = attr("description", "نحن هنا لمساعدتك، لا تتردد في الاتصال بنا");
            let email = attr("email", "");
            let phone = attr("phone", "");
            let address = attr("address", "");

            let mut info_items = String::new();
            if !email.is_empty() {
                info_items.push_str(&format!(
                    "<div class=\"amana-contact-info-item\">\
                       <div class=\"amana-contact-info-icon\"><iconify-icon icon=\"heroicons:envelope\"></iconify-icon></div>\
                       <div><h4>البريد الإلكتروني</h4><a href=\"mailto:{}\">{}</a></div>\
                     </div>",
                    email, email
                ));
            }
            if !phone.is_empty() {
                info_items.push_str(&format!(
                    "<div class=\"amana-contact-info-item\">\
                       <div class=\"amana-contact-info-icon\"><iconify-icon icon=\"heroicons:phone\"></iconify-icon></div>\
                       <div><h4>رقم الهاتف</h4><a href=\"tel:{}\">{}</a></div>\
                     </div>",
                    phone, phone
                ));
            }
            if !address.is_empty() {
                info_items.push_str(&format!(
                    "<div class=\"amana-contact-info-item\">\
                       <div class=\"amana-contact-info-icon\"><iconify-icon icon=\"heroicons:map-pin\"></iconify-icon></div>\
                       <div><h4>العنوان</h4><p>{}</p></div>\
                     </div>",
                    address
                ));
            }

            let form_body = if inner.is_empty() {
                "<div class=\"amana-form-field\">\
                   <label class=\"amana-form-field-label\">الاسم الكامل</label>\
                   <input type=\"text\" class=\"amana-form-field-input\" required />\
                 </div>\
                 <div class=\"amana-form-field\">\
                   <label class=\"amana-form-field-label\">البريد الإلكتروني</label>\
                   <input type=\"email\" class=\"amana-form-field-input\" required />\
                 </div>\
                 <div class=\"amana-form-field\">\
                   <label class=\"amana-form-field-label\">الرسالة</label>\
                   <textarea class=\"amana-form-field-input amana-textarea\" rows=\"4\" required></textarea>\
                 </div>\
                 <button type=\"submit\" class=\"amana-btn amana-btn-primary amana-btn-lg\">إرسال الرسالة</button>".to_string()
            } else {
                inner
            };

            Some(format!(
                "<section class=\"{}\">\
                   <div class=\"amana-contact-container\">\
                     <div class=\"amana-contact-info-pane\">\
                       <h2>{}</h2>\
                       <p class=\"amana-contact-pane-description\">{}</p>\
                       <div class=\"amana-contact-info-list\">\
                         {}\
                       </div>\
                     </div>\
                     <div class=\"amana-contact-form-pane\">\
                       <form class=\"amana-contact-form\" @submit.prevent=\"alert('تم إرسال رسالتك بنجاح!')\">\
                         {}\
                       </form>\
                     </div>\
                   </div>\
                 </section>",
                standard_class("amana-contact-section", classes),
                title,
                description,
                info_items,
                form_body
            ))
        }
        "Grid" => {
            let min = attr("min", "16rem");
            let columns = attr("columns", "");
            let stretch = attr("stretch", "false");
            let style_attr = if columns.is_empty() {
                format!("style=\"--grid-min:{}\"", min)
            } else {
                let col_val = if let Ok(n) = columns.parse::<u32>() {
                    format!("repeat({}, minmax(0, 1fr))", n)
                } else {
                    columns.clone()
                };
                format!("style=\"--grid-min:{};--dg-columns:{}\"", min, col_val)
            };
            let mut grid_classes = classes.to_vec();
            if stretch == "true" {
                grid_classes.push("amana-grid-stretch".to_string());
            }
            Some(format!(
                "<div class=\"{}\" {}>{}</div>",
                standard_class("amana-grid", &grid_classes),
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
                "<nav class=\"{}\" x-data=\"{{ open: false }}\">\n  \
                   <a class=\"amana-brand\" href=\"/\">{}</a>\n  \
                   <button type=\"button\" class=\"amana-navbar-menu-btn\" @click.stop=\"open = !open\" aria-label=\"Toggle menu\">\n    \
                     <svg fill=\"none\" stroke=\"currentColor\" viewBox=\"0 0 24 24\">\n      \
                       <path :class=\"open ? 'hidden' : 'inline-flex'\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"2.5\" d=\"M4 6h16M4 12h16M4 18h16\" />\n      \
                       <path :class=\"open ? 'inline-flex' : 'hidden'\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"2.5\" d=\"M6 18L18 6M6 6l12 12\" />\n    \
                     </svg>\n  \
                   </button>\n  \
                   <div class=\"amana-navlinks\" :class=\"open ? 'active' : ''\" @click.away=\"open = false\">{}</div>\n\
                 </nav>",
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
            let open_expr = attributes.iter().find(|(k, _)| k == "open").map(|(_, e)| e);
            let open = match open_expr {
                Some(expr) => match expr {
                    Expression::StringLiteral(s) => s.clone(),
                    _ => compile_expression_to_js_scoped(expr, aliases),
                },
                None => "modal_open".to_string(),
            };
            let modal_index = MODAL_COUNTER.with(|counter| {
                let val = counter.get();
                counter.set(val + 1);
                val
            });
            let title_id = format!("amana-modal-title-{}", modal_index);

            let title_expr = attributes.iter().find(|(k, _)| k == "title").map(|(_, e)| e);
            let mut has_title = false;
            let title_html = match title_expr {
                Some(expr) => {
                    let escaped = match expr {
                        Expression::StringLiteral(s) => {
                            if s.starts_with("f\"") && s.ends_with("\"") {
                                let content = &s[2..s.len() - 1];
                                let content_escaped = process_formatted_string(content, aliases);
                                format!("<%= `{}` %>", content_escaped)
                            } else {
                                format!("<%= \"{}\" %>", s.replace("\"", "\\\""))
                            }
                        }
                        Expression::Identifier(id) => {
                            format!("<%= {} %>", scoped_identifier(id, aliases))
                        }
                        _ => {
                            let js = compile_expression_to_js_scoped(expr, aliases);
                            if js.starts_with("<%=") && js.ends_with("%>") {
                                js
                            } else {
                                format!("<%= {} %>", js)
                            }
                        }
                    };
                    has_title = true;
                    format!("<h3 id=\"{}\" class=\"amana-modal-title\">{}</h3>\n", title_id, escaped)
                }
                None => String::new(),
            };

            let closable = {
                let val = attr("closable", "true");
                val != "false" && val != "no"
            };

            let close_button = if closable {
                format!("<button type=\"button\" class=\"amana-modal-close\" @click=\"{} = false\">&times;</button>\n", open)
            } else {
                String::new()
            };

            let aria_labelledby = if has_title {
                format!(" aria-labelledby=\"{}\"", title_id)
            } else {
                String::new()
            };

            let focus_trap_js = format!(
                "@keydown.tab=\"\
                    let focusables = $el.querySelectorAll('button, [href], input, select, textarea, [tabindex]:not([tabindex=\\'-1\\'])');\
                    if (focusables.length > 0) {{\
                        let first = focusables[0];\
                        let last = focusables[focusables.length - 1];\
                        if ($event.shiftKey && $event.target === first) {{\
                            last.focus();\
                            $event.preventDefault();\
                        }} else if (!$event.shiftKey && $event.target === last) {{\
                            first.focus();\
                            $event.preventDefault();\
                        }}\
                    }}\
                \""
            );

            let scroll_lock_js = format!(
                "x-effect=\"\
                    if ({}) {{\
                        document.body.style.overflow = 'hidden';\
                        $nextTick(() => {{\
                            let focusables = $el.querySelectorAll('button, [href], input, select, textarea, [tabindex]:not([tabindex=\\'-1\\'])');\
                            if (focusables.length > 0) {{\
                                focusables[0].focus();\
                            }}\
                        }});\
                    }} else {{\
                        document.body.style.overflow = '';\
                    }}\
                \"",
                open
            );

            Some(format!(
                "<div class=\"{}\" x-show=\"{}\" \
                      @keydown.escape.window=\"{} = false\" \
                      @click.self=\"{} = false\" \
                      {} \
                      {} \
                      role=\"dialog\" \
                      aria-modal=\"true\" \
                      {}>\
                   <div class=\"amana-modal-panel\">\
                      {}{}{}\
                   </div>\
                 </div>",
                standard_class("amana-modal", classes),
                open,
                open,
                open,
                scroll_lock_js,
                focus_trap_js,
                aria_labelledby,
                close_button,
                title_html,
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
            let trend_class = if trend.starts_with('-') {
                "amana-kpi-trend amana-kpi-trend-down"
            } else if trend.starts_with('+') {
                "amana-kpi-trend amana-kpi-trend-up"
            } else if !trend.is_empty() {
                "amana-kpi-trend amana-kpi-trend-neutral"
            } else {
                ""
            };
            Some(format!(
                "<article class=\"{}\">{}{}{}{}</article>",
                standard_class("amana-kpi", classes),
                if label.is_empty() { "".to_string() } else { format!("<span class=\"amana-kpi-label\">{}</span>", label) },
                if value.is_empty() { "".to_string() } else { format!("<strong class=\"amana-kpi-value\">{}</strong>", value) },
                if trend.is_empty() { "".to_string() } else { format!("<span class=\"{}\">{}</span>", trend_class, trend) },
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
                let avatar_html = if !author.is_empty() {
                    let first_char = author.trim().chars().next().unwrap_or(' ');
                    format!("<div class=\"amana-testimonial-avatar\">{}</div>", first_char)
                } else {
                    "".to_string()
                };
                let info_html = format!(
                    "<div class=\"amana-testimonial-info\">{}{}</div>",
                    if author.is_empty() { "".to_string() } else { format!("<strong>{}</strong>", author) },
                    if role.is_empty() { "".to_string() } else { format!("<span>{}</span>", role) }
                );
                format!(
                    "<figcaption>{}{}</figcaption>",
                    avatar_html, info_html
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
            let icon_markup = "<iconify-icon icon=\"heroicons:folder-open\" class=\"amana-empty-state-icon\" aria-hidden=\"true\"></iconify-icon>";
            Some(format!(
                "<section class=\"{}\">{}{}{}{}{}</section>",
                standard_class("amana-empty-state", classes),
                icon_markup,
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
        "Slides" => {
            let height = attr("height", "22rem");
            let autoplay = attr("autoplay", "false");
            let child_count = children.len();
            
            let mut slides_html = String::new();
            for (i, child) in children.iter().enumerate() {
                let active_class = if i == 0 { " active" } else { "" };
                let child_html = generate_ejs_scoped(child, client_states, aliases);
                slides_html.push_str(&format!(
                    "<div class=\"amana-slide{}\" :class=\"{{ 'active': activeSlide === {} }}\">{}</div>",
                    active_class, i, child_html
                ));
            }

            let mut dots_html = String::new();
            for i in 0..child_count {
                dots_html.push_str(&format!(
                    "<span class=\"amana-slides-dot\" :class=\"{{ 'active': activeSlide === {} }}\" @click=\"activeSlide = {}\"></span>",
                    i, i
                ));
            }

            Some(format!(
                "<div class=\"{}\" x-data=\"{{ activeSlide: 0, slidesCount: {}, autoplay: {}, init() {{ if (this.autoplay) {{ setInterval(() => {{ this.activeSlide = (this.activeSlide + 1) % this.slidesCount; }}, 5000); }} }} }}\" style=\"--slides-height:{}\">\n\
                   <div class=\"amana-slides-inner\">\n\
                     {}\n\
                   </div>\n\
                   <button type=\"button\" class=\"amana-slides-arrow prev\" @click=\"activeSlide = (activeSlide - 1 + slidesCount) % slidesCount\">&larr;</button>\n\
                   <button type=\"button\" class=\"amana-slides-arrow next\" @click=\"activeSlide = (activeSlide + 1) % slidesCount\">&rarr;</button>\n\
                   <div class=\"amana-slides-dots\">\n\
                     {}\n\
                   </div>\n\
                 </div>",
                standard_class("amana-slides", classes),
                child_count,
                autoplay,
                height,
                slides_html,
                dots_html
            ))
        }
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
    MODAL_COUNTER.with(|counter| counter.set(0));
    CHART_COUNTER.with(|counter| counter.set(0));
    generate_ejs_scoped(element, client_states, &[])
}

pub fn generate_ejs_with_auth_model(
    element: &ViewElement,
    client_states: &[StateDecl],
    _auth_model: &Option<String>,
) -> String {
    MODAL_COUNTER.with(|counter| counter.set(0));
    CHART_COUNTER.with(|counter| counter.set(0));
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
                    let code = match expr {
                        Expression::StringLiteral(s) => s.clone(),
                        _ => compile_expression_to_js_scoped(expr, aliases),
                    };
                    let escaped = code.replace('&', "&amp;")
                                      .replace('"', "&quot;")
                                      .replace('<', "&lt;")
                                      .replace('>', "&gt;");
                    attrs.push_str(&format!(
                        " x-on:{}=\"{}\"",
                        key,
                        escaped
                    ));
                } else if *key == "show" {
                    let code = match expr {
                        Expression::StringLiteral(s) => s.clone(),
                        _ => compile_expression_to_js_scoped(expr, aliases),
                    };
                    let escaped = code.replace('&', "&amp;")
                                      .replace('"', "&quot;")
                                      .replace('<', "&lt;")
                                      .replace('>', "&gt;");
                    attrs.push_str(&format!(
                        " x-show=\"{}\"",
                        escaped
                    ));
                } else if *key == "text" {
                    let code = match expr {
                        Expression::StringLiteral(s) => s.clone(),
                        _ => compile_expression_to_js_scoped(expr, aliases),
                    };
                    let escaped = code.replace('&', "&amp;")
                                      .replace('"', "&quot;")
                                      .replace('<', "&lt;")
                                      .replace('>', "&gt;");
                    attrs.push_str(&format!(
                        " x-text=\"{}\"",
                        escaped
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
                } else if *key == "x_data" || *key == "xdata" {
                    let code = match expr {
                        Expression::StringLiteral(s) => s.clone(),
                        _ => compile_expression_to_js_scoped(expr, aliases),
                    };
                    let escaped = code.replace('&', "&amp;")
                                      .replace('"', "&quot;")
                                      .replace('<', "&lt;")
                                      .replace('>', "&gt;");
                    attrs.push_str(&format!(" x-data=\"{}\"", escaped));
                } else if matches!(
                    key.as_str(),
                    "disabled" | "checked" | "selected" | "readonly"
                ) {
                    let js = compile_expression_to_js_scoped(expr, aliases);
                    let escaped = js.replace('&', "&amp;")
                                    .replace('"', "&quot;")
                                    .replace('<', "&lt;")
                                    .replace('>', "&gt;");
                    attrs.push_str(&format!(
                        " :{}=\"{}\"",
                        key,
                        escaped
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
                let js_template = process_formatted_string(content, aliases);
                format!("<span x-text=\"`{}`\"></span>", js_template)
            } else if txt.starts_with("f\"") && txt.ends_with("\"") {
                let content = &txt[2..txt.len() - 1];
                // Replace User.current with currentUser in formatted strings
                let mut js_template = content.to_string();

                // Handle all variations of User.current access
                let replacements = [
                    ("{User.current.name}", "{currentUser.name}"),
                    ("{User.current.email}", "{currentUser.email}"),
                    ("{User.current.role}", "{currentUser.role}"),
                    ("{User.current.id}", "{currentUser.id}"),
                    ("{User.current}", "{currentUser}"),
                ];

                for (pattern, replacement) in &replacements {
                    js_template = js_template.replace(pattern, replacement);
                }

                // Process remaining placeholders with aliases
                let processed = process_formatted_string(&js_template, aliases);

                format!("<%= `{}` %>", processed)
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
                "/form-submit/{}<%= typeof query !== 'undefined' && query && Object.keys(query).length > 0 ? '?' + new URLSearchParams(query).toString() : '' %>",
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
            let chart_idx = CHART_COUNTER.with(|counter| {
                let val = counter.get();
                counter.set(val + 1);
                val
            });
            let canvas_id = format!("chart_{}_{}", data_expr, chart_idx);
            format!(
                "<div class=\"chart-container amana-chart-wrapper\">\n  <canvas id=\"{}\"></canvas>\n</div>\n\
                <script>\n\
                document.addEventListener('DOMContentLoaded', () => {{\n\
                  const ctx = document.getElementById('{}').getContext('2d');\n\
                  const rawData = JSON.parse(decodeURIComponent('<%- encodeURIComponent(JSON.stringify({})) %>'));\n\
                  \n\
                  const style = getComputedStyle(document.documentElement);\n\
                  const primaryColor = style.getPropertyValue('--color-primary').trim() || '#6366f1';\n\
                  const accentColor = style.getPropertyValue('--color-accent').trim() || '#06b6d4';\n\
                  const textColor = style.getPropertyValue('--text-secondary').trim() || '#94a3b8';\n\
                  const gridColor = 'rgba(255, 255, 255, 0.03)';\n\
                  \n\
                  const gradient = ctx.createLinearGradient(0, 0, 0, 320);\n\
                  gradient.addColorStop(0, primaryColor.replace('rgb', 'rgba').replace(')', ', 0.15)'));\n\
                  gradient.addColorStop(1, 'rgba(0, 0, 0, 0)');\n\
                  \n\
                  new Chart(ctx, {{\n\
                    type: '{}',\n\
                    data: {{\n\
                      labels: rawData.map(row => typeof fixArabicText !== 'undefined' ? fixArabicText(row.{}) : row.{}),\n\
                      datasets: [{{\n\
                        label: typeof fixArabicText !== 'undefined' ? fixArabicText('بيانات {}') : 'بيانات {}',\n\
                        data: rawData.map(row => row.{}),\n\
                        backgroundColor: '{}' === 'line' ? gradient : primaryColor,\n\
                        borderColor: primaryColor,\n\
                        borderWidth: 2.5,\n\
                        borderRadius: '{}' === 'bar' ? 6 : 0,\n\
                        tension: 0.4,\n\
                        pointBackgroundColor: primaryColor,\n\
                        pointBorderColor: '#fff',\n\
                        pointHoverRadius: 6\n\
                      }}]\n\
                    }},\n\
                    options: {{\n\
                      responsive: true,\n\
                      maintainAspectRatio: false,\n\
                      plugins: {{\n\
                        legend: {{\n\
                          labels: {{\n\
                            color: textColor,\n\
                            font: {{ family: 'system-ui, sans-serif', weight: 'bold' }}\n\
                          }}\n\
                        }},\n\
                        tooltip: {{\n\
                          backgroundColor: 'rgba(15, 23, 42, 0.8)',\n\
                          titleFont: {{ family: 'system-ui, sans-serif' }},\n\
                          bodyFont: {{ family: 'system-ui, sans-serif' }},\n\
                          padding: 12,\n\
                          cornerRadius: 8,\n\
                          borderWidth: 1,\n\
                          borderColor: 'rgba(255, 255, 255, 0.1)'\n\
                        }}\n\
                      }},\n\
                      scales: {{\n\
                        x: {{\n\
                          grid: {{ color: gridColor }},\n\
                          ticks: {{ color: textColor, font: {{ family: 'system-ui, sans-serif' }} }}\n\
                        }},\n\
                        y: {{\n\
                          grid: {{ color: gridColor }},\n\
                          ticks: {{ color: textColor, font: {{ family: 'system-ui, sans-serif' }} }}\n\
                        }}\n\
                      }}\n\
                    }}\n\
                  }});\n\
                }});\n\
                </script>\n",
                canvas_id, canvas_id, data_expr, chart_type, x_field, x_field, data_expr, data_expr, y_field, chart_type, chart_type
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
        } => {
            let resource_js = compile_expression_to_js_scoped(resource_expr, aliases);
            
            let mut empty_html = String::new();
            if let Some(nodes) = empty_element {
                for node in nodes {
                    empty_html.push_str(&generate_ejs_scoped(node, client_states, aliases));
                }
            }
            
            let loop_html = format!(
                "<div class=\"amana-grid\" style=\"--grid-min: 18rem; --dg-columns: repeat(auto-fit, minmax(18rem, 1fr));\">\n<% for (let {} of {}) {{ %>\n  <%- include('components/{}', {{ {}: {} }}) %>\n<% }} %>\n</div>\n",
                item_arg_name, resource_js, item_component, item_arg_name, item_arg_name
            );
            
            format!(
                "<% if (typeof {} !== 'undefined' && {} && {}.length > 0) {{ %>\n\
                 {}\n\
                 <% }} else {{ %>\n\
                 <div class=\"amana-resource-empty\">\n{}\n</div>\n\
                 <% }} %>\n",
                resource_js, resource_js, resource_js,
                loop_html,
                empty_html
            )
        }
        ViewElement::ResourceTable {
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
                "<div class=\"amana-resource\">\n\
                 <% for (let {} of {}) {{ %>\n\
                   <div class=\"amana-resource-item\">\n\
                     <%- include('components/{}', {{ {}: {} }}) %>\n\
                   </div>\n\
                 <% }} %>\n\
                 </div>\n",
                item_arg_name, resource_js, item_component, item_arg_name, item_arg_name
            );
            
            format!(
                "<% if (typeof {} !== 'undefined' && {} && {}.length > 0) {{ %>\n\
                 {}\n\
                 <% }} else {{ %>\n\
                 <div class=\"amana-resource-empty\">\n{}\n</div>\n\
                 <% }} %>\n",
                resource_js, resource_js, resource_js,
                loop_html,
                empty_html
            )
        }
        ViewElement::Tabs { tabs } => {
            let mut headers = String::new();
            let mut contents = String::new();
            let tabs_count = tabs.len();

            use std::sync::atomic::{AtomicUsize, Ordering};
            static TABS_COUNTER: AtomicUsize = AtomicUsize::new(0);
            let tabs_id = TABS_COUNTER.fetch_add(1, Ordering::SeqCst);

            for (i, (title, tab_body)) in tabs.iter().enumerate() {
                headers.push_str(&format!(
                    "<button type=\"button\" \
                             role=\"tab\" \
                             id=\"amana-tab-btn-{}-{}\" \
                             aria-selected=\"activeTab === {}\" \
                             :aria-selected=\"activeTab === {}\" \
                             aria-controls=\"amana-tab-panel-{}-{}\" \
                             :tabindex=\"activeTab === {} ? 0 : -1\" \
                             class=\"amana-tab-button\" \
                             :class=\"activeTab === {} && 'active'\" \
                             @click=\"activeTab = {}\">{}</button>\n",
                    tabs_id, i, i, i, tabs_id, i, i, i, i, title
                ));

                let panel_html = tab_body
                    .iter()
                    .map(|c| generate_ejs_scoped(c, client_states, aliases))
                    .collect::<String>();
                contents.push_str(&format!(
                    "<div class=\"amana-tab-panel\" \
                          role=\"tabpanel\" \
                          id=\"amana-tab-panel-{}-{}\" \
                          aria-labelledby=\"amana-tab-btn-{}-{}\" \
                          x-show=\"activeTab === {}\">\n{}</div>\n",
                    tabs_id, i, tabs_id, i, i, panel_html
                ));
            }
            format!(
                "<div class=\"amana-tabs\" \
                      x-data=\"{{ activeTab: 0, tabsCount: {} }}\" \
                      @keydown.arrow-right=\"activeTab = (activeTab + 1) % tabsCount; $nextTick(() => $el.querySelectorAll('[role=tab]')[activeTab].focus())\" \
                      @keydown.arrow-left=\"activeTab = (activeTab - 1 + tabsCount) % tabsCount; $nextTick(() => $el.querySelectorAll('[role=tab]')[activeTab].focus())\">\n  \
                    <div class=\"amana-tabs-header\" role=\"tablist\">\n{}  </div>\n  \
                    <div class=\"amana-tabs-content\">\n{}  </div>\n\
                 </div>\n",
                tabs_count, headers, contents
            )
        }
        ViewElement::Accordion { panels } => {
            let mut panels_html = String::new();

            static ACCORDION_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let acc_id = ACCORDION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            for (i, (title, panel_body)) in panels.iter().enumerate() {
                let body_html = panel_body
                    .iter()
                    .map(|c| generate_ejs_scoped(c, client_states, aliases))
                    .collect::<String>();
                panels_html.push_str(&format!(
                    "<div class=\"amana-accordion-item\" x-data=\"{{ open: false }}\">\n\
                       <button type=\"button\" \
                               id=\"amana-acc-btn-{}-{}\" \
                               class=\"amana-accordion-header\" \
                               :aria-expanded=\"open\" \
                               aria-controls=\"amana-acc-panel-{}-{}\" \
                               @click=\"open = !open\">\n\
                         <span class=\"amana-accordion-title\">{}</span>\n\
                         <svg class=\"amana-accordion-chevron\" :class=\"open && 'rotate-180'\" fill=\"none\" viewBox=\"0 0 24 24\" stroke=\"currentColor\">\n\
                           <path stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"2\" d=\"M19 9l-7 7-7-7\" />\n\
                         </svg>\n\
                       </button>\n\
                       <div class=\"amana-accordion-content\" \
                            id=\"amana-acc-panel-{}-{}\" \
                            role=\"region\" \
                            aria-labelledby=\"amana-acc-btn-{}-{}\" \
                            x-show=\"open\">\n\
                      {}\n\
                       </div>\n\
                     </div>\n",
                    acc_id, i, acc_id, i, title, acc_id, i, acc_id, i, body_html
                ));
            }
            format!(
                "<div class=\"amana-accordion\">\n{}</div>\n",
                panels_html
            )
        }
    }
}
