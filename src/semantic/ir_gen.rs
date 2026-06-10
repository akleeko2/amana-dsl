// src/semantic/ir_gen.rs
use crate::ast::*;
use crate::semantic::SemanticAnalyzer;
use crate::semantic::ir::*;

/// Generates the Amana Intermediate Representation (IR) from the optimized AST node declarations.
/// Performs component inlining, style aggregation, and structured route/view mapping.
pub fn generate_ir(
    _analyzer: &SemanticAnalyzer,
    nodes: &[AmanaNode],
    app_config: &AppConfig,
) -> Result<AmanaIR, String> {
    // 1. AppConfig -> AppIR
    let app = AppIR {
        name: app_config.name.clone(),
        title: app_config.title.clone(),
        db_path: app_config.db_path.clone(),
        auth_model: app_config.auth_model.clone(),
        capabilities: app_config.capabilities.clone(),
    };

    let theme = nodes.iter().find_map(|node| {
        if let AmanaNode::Theme(theme) = node {
            Some(ThemeIR {
                settings: theme.settings.clone(),
            })
        } else {
            None
        }
    });

    // 2. ModelDecl -> ModelIR
    let mut models = Vec::new();
    for node in nodes {
        if let AmanaNode::Model(m) = node {
            let mut fields = Vec::new();
            for f in &m.fields {
                fields.push(ModelFieldIR {
                    name: f.name.clone(),
                    data_type: f.data_type.clone(),
                    is_primary_key: f.is_primary_key,
                    is_unique: f.is_unique,
                    is_required: f.is_required,
                    min_value: f.min_value,
                    max_value: f.max_value,
                    default_value: f.default_value.clone(),
                    foreign_key: f.foreign_key.clone(),
                    on_delete: f.on_delete.clone(),
                });
            }
            models.push(ModelIR {
                name: m.name.clone(),
                table_name: m.name.to_lowercase(),
                fields,
            });
        }
    }

    // 3. ViewDecl & RouteDecl -> RouteIR & ViewIR
    let mut routes = Vec::new();
    let mut views = Vec::new();

    let mut views_map = std::collections::BTreeMap::new();
    for node in nodes {
        if let AmanaNode::View(v) = node {
            views_map.insert(v.name.clone(), v.clone());
        }
    }

    let mut components_map = std::collections::BTreeMap::new();
    for node in nodes {
        if let AmanaNode::Component(c) = node {
            components_map.insert(c.name.clone(), c.clone());
        }
    }

    for node in nodes {
        if let AmanaNode::Route(r) = node {
            if let Some(view) = views_map.get(&r.view_name) {
                // 1. Guard
                let mut guard = view.protected.as_ref().map(|p| GuardIR {
                    cond_expr: p.allow_expr.clone(),
                    deny_path: p.deny_path.clone(),
                    unauth_path: p.unauth_path.clone(),
                });
                if guard.is_none()
                    && let Some(route_guard) = r.guards.first()
                {
                    let redirect_path = route_guard
                        .else_action
                        .strip_prefix("redirect ")
                        .unwrap_or("/login")
                        .to_string();
                    guard = Some(GuardIR {
                        cond_expr: route_guard.condition.clone(),
                        deny_path: redirect_path.clone(),
                        unauth_path: redirect_path,
                    });
                }

                // 2. Fetches
                let mut fetches = Vec::new();
                for fetch in r.fetches.iter().chain(view.server_fetches.iter()) {
                    fetches.push(FetchIR {
                        var_name: fetch.var_name.clone(),
                        model_name: fetch.model_name.clone(),
                        query_method: fetch.query_method.clone(),
                        query_args: fetch.query_args.clone(),
                    });
                }

                // Compile-time Component Inlining
                let inlined_render_body = view
                    .render_body
                    .as_ref()
                    .map(|b| inline_components(b, &components_map));

                // 3. Form Actions
                let mut form_actions = Vec::new();
                if let Some(ref render_body) = inlined_render_body {
                    extract_form_actions(render_body, &mut form_actions)?;
                }

                routes.push(RouteIR {
                    path: r.path.clone(),
                    view_name: r.view_name.clone(),
                    guard,
                    fetches,
                    form_actions,
                });

                // Aggregated styles from view + components used in it
                let mut all_styles = view.styles.clone().unwrap_or_default();
                let mut used_components = std::collections::BTreeSet::new();
                if let Some(ref render_body) = view.render_body {
                    collect_used_components(render_body, &components_map, &mut used_components);
                }
                for comp_name in used_components {
                    if let Some(comp) = components_map.get(&comp_name)
                        && let Some(ref comp_styles) = comp.styles
                    {
                        if !all_styles.is_empty() {
                            all_styles.push('\n');
                        }
                        all_styles.push_str(comp_styles);
                    }
                }
                let styles_option = if all_styles.is_empty() {
                    None
                } else {
                    Some(all_styles)
                };

                // 4. View IR
                views.push(ViewIR {
                    name: view.name.clone(),
                    client_states: view.client_states.clone(),
                    render_body: inlined_render_body,
                    styles: styles_option,
                    canvas: view.canvas.clone(),
                });
            } else {
                return Err(format!(
                    "Route '{}' references missing view '{}'.",
                    r.path, r.view_name
                ));
            }
        }
    }

    let mut seeds = Vec::new();
    for node in nodes {
        if let AmanaNode::Seed(seed) = node {
            seeds.push(SeedIR {
                model_name: seed.model_name.clone(),
                rows: seed.rows.clone(),
            });
        }
    }

    Ok(AmanaIR {
        ir_version: IRVersion {
            major: 1,
            minor: 0,
            patch: 0,
            capabilities: vec![
                "sqlite_sql".to_string(),
                "ejs_views".to_string(),
                "express_routing".to_string(),
                "sandboxed_hooks_v1".to_string(),
            ],
        },
        app,
        models,
        theme,
        routes,
        views,
        seeds,
    })
}

fn extract_form_actions(
    element: &ViewElement,
    actions: &mut Vec<FormActionIR>,
) -> Result<(), String> {
    match element {
        ViewElement::FormBlock {
            fields,
            connect_action,
            redirect_success,
            defaults,
            constraints,
            ..
        } => {
            let parts: Vec<&str> = connect_action.split('.').collect();
            if parts.len() != 2 {
                return Err(format!("Invalid form connect action: {}", connect_action));
            }
            let action_lowercase = parts[1].to_lowercase();
            let allowed_actions = ["create", "update", "delete", "login", "register", "logout"];
            if !allowed_actions.contains(&action_lowercase.as_str()) {
                return Err(format!(
                    "Unsupported form action '{}.{}'. Allowed actions are: {}.",
                    parts[0],
                    action_lowercase,
                    allowed_actions.join(", ")
                ));
            }
            if (action_lowercase == "update" || action_lowercase == "delete")
                && !fields.iter().any(|f| f.to_lowercase() == "id")
            {
                return Err(format!(
                    "Form connected to '{}.{}' must include the 'id' field.",
                    parts[0], action_lowercase
                ));
            }

            actions.push(FormActionIR {
                model_name: parts[0].to_string(),
                action: action_lowercase,
                fields: fields.clone(),
                defaults: defaults.clone(),
                constraints: constraints.clone(),
                redirect_success: redirect_success.clone(),
            });
        }
        ViewElement::Element { children, .. } => {
            for child in children {
                extract_form_actions(child, actions)?;
            }
        }
        ViewElement::ForEach { body, .. } => {
            for child in body {
                extract_form_actions(child, actions)?;
            }
        }
        ViewElement::IfBlock {
            then_branch,
            else_branch,
            ..
        } => {
            for child in then_branch {
                extract_form_actions(child, actions)?;
            }
            if let Some(branch) = else_branch {
                for child in branch {
                    extract_form_actions(child, actions)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn inline_components(
    element: &ViewElement,
    components: &std::collections::BTreeMap<String, ComponentDecl>,
) -> ViewElement {
    match element {
        ViewElement::Element {
            tag,
            classes,
            attributes,
            children,
        } => {
            if let Some(comp) = components.get(tag) {
                let mut param_map = std::collections::BTreeMap::new();
                for (name, expr) in attributes {
                    param_map.insert(name.clone(), expr.clone());
                }

                if let Some(ref body) = comp.render_body {
                    let expanded = substitute_params(body, &param_map);
                    let inlined_children: Vec<ViewElement> = children
                        .iter()
                        .map(|c| inline_components(c, components))
                        .collect();
                    replace_slots(expanded, &inlined_children)
                } else {
                    ViewElement::Text("".to_string())
                }
            } else {
                let inlined_children = children
                    .iter()
                    .map(|c| inline_components(c, components))
                    .collect();
                ViewElement::Element {
                    tag: tag.clone(),
                    classes: classes.clone(),
                    attributes: attributes.clone(),
                    children: inlined_children,
                }
            }
        }
        ViewElement::ForEach {
            item_var,
            list_expr,
            body,
        } => {
            let inlined_body = body
                .iter()
                .map(|c| inline_components(c, components))
                .collect();
            ViewElement::ForEach {
                item_var: item_var.clone(),
                list_expr: list_expr.clone(),
                body: inlined_body,
            }
        }
        ViewElement::IfBlock {
            condition,
            then_branch,
            else_branch,
        } => {
            let inlined_then = then_branch
                .iter()
                .map(|c| inline_components(c, components))
                .collect();
            let inlined_else = else_branch.as_ref().map(|eb| {
                eb.iter()
                    .map(|c| inline_components(c, components))
                    .collect()
            });
            ViewElement::IfBlock {
                condition: condition.clone(),
                then_branch: inlined_then,
                else_branch: inlined_else,
            }
        }
        _ => element.clone(),
    }
}

fn substitute_params(
    element: &ViewElement,
    param_map: &std::collections::BTreeMap<String, Expression>,
) -> ViewElement {
    match element {
        ViewElement::Element {
            tag,
            classes,
            attributes,
            children,
        } => {
            let substituted_attrs = attributes
                .iter()
                .map(|(k, expr)| (k.clone(), substitute_expr_params(expr, param_map)))
                .collect();
            let substituted_children = children
                .iter()
                .map(|c| substitute_params(c, param_map))
                .collect();
            ViewElement::Element {
                tag: tag.clone(),
                classes: classes.clone(),
                attributes: substituted_attrs,
                children: substituted_children,
            }
        }
        ViewElement::FormattedText(exprs) => {
            let substituted_exprs = exprs
                .iter()
                .map(|expr| substitute_expr_params(expr, param_map))
                .collect();
            ViewElement::FormattedText(substituted_exprs)
        }
        ViewElement::ForEach {
            item_var,
            list_expr,
            body,
        } => {
            let substituted_list = substitute_expr_params(list_expr, param_map);
            let substituted_body = body
                .iter()
                .map(|c| substitute_params(c, param_map))
                .collect();
            ViewElement::ForEach {
                item_var: item_var.clone(),
                list_expr: substituted_list,
                body: substituted_body,
            }
        }
        ViewElement::IfBlock {
            condition,
            then_branch,
            else_branch,
        } => {
            let substituted_cond = substitute_expr_params(condition, param_map);
            let substituted_then = then_branch
                .iter()
                .map(|c| substitute_params(c, param_map))
                .collect();
            let substituted_else = else_branch
                .as_ref()
                .map(|eb| eb.iter().map(|c| substitute_params(c, param_map)).collect());
            ViewElement::IfBlock {
                condition: substituted_cond,
                then_branch: substituted_then,
                else_branch: substituted_else,
            }
        }
        _ => element.clone(),
    }
}

fn substitute_expr_params(
    expr: &Expression,
    param_map: &std::collections::BTreeMap<String, Expression>,
) -> Expression {
    match expr {
        Expression::Identifier(name) => {
            if let Some(sub_expr) = param_map.get(name) {
                sub_expr.clone()
            } else {
                expr.clone()
            }
        }
        Expression::Binary { left, op, right } => Expression::Binary {
            left: Box::new(substitute_expr_params(left, param_map)),
            op: op.clone(),
            right: Box::new(substitute_expr_params(right, param_map)),
        },
        Expression::Unary { op, expr } => Expression::Unary {
            op: op.clone(),
            expr: Box::new(substitute_expr_params(expr, param_map)),
        },
        Expression::Ternary {
            cond,
            then_branch,
            else_branch,
        } => Expression::Ternary {
            cond: Box::new(substitute_expr_params(cond, param_map)),
            then_branch: Box::new(substitute_expr_params(then_branch, param_map)),
            else_branch: Box::new(substitute_expr_params(else_branch, param_map)),
        },
        Expression::Call { callee, args } => {
            let sub_args = args
                .iter()
                .map(|arg| substitute_expr_params(arg, param_map))
                .collect();
            Expression::Call {
                callee: Box::new(substitute_expr_params(callee, param_map)),
                args: sub_args,
            }
        }
        Expression::MemberAccess { object, property } => Expression::MemberAccess {
            object: Box::new(substitute_expr_params(object, param_map)),
            property: property.clone(),
        },
        _ => expr.clone(),
    }
}

fn replace_slots(element: ViewElement, slot_children: &[ViewElement]) -> ViewElement {
    match element {
        ViewElement::SlotDecl { name, optional } => {
            let found_filler = slot_children.iter().find(|child| {
                if let ViewElement::Element { tag, .. } = child {
                    tag == &name
                } else {
                    false
                }
            });
            if let Some(ViewElement::Element { children, .. }) = found_filler {
                ViewElement::Element {
                    tag: "div".to_string(),
                    classes: vec![format!("slot-{}", name)],
                    attributes: vec![],
                    children: children.clone(),
                }
            } else if name == "default" && !slot_children.is_empty() {
                ViewElement::Element {
                    tag: "div".to_string(),
                    classes: vec!["slot-container".to_string()],
                    attributes: vec![],
                    children: slot_children.to_vec(),
                }
            } else if optional {
                ViewElement::Text("".to_string())
            } else {
                ViewElement::Element {
                    tag: "div".to_string(),
                    classes: vec![format!("slot-{}", name)],
                    attributes: vec![],
                    children: vec![],
                }
            }
        }
        ViewElement::Element {
            tag,
            classes,
            attributes,
            children,
        } => {
            if tag == "slot" {
                ViewElement::Element {
                    tag: "div".to_string(),
                    classes: vec!["slot-container".to_string()],
                    attributes: vec![],
                    children: slot_children.to_vec(),
                }
            } else {
                let replaced_children = children
                    .into_iter()
                    .map(|c| replace_slots(c, slot_children))
                    .collect();
                ViewElement::Element {
                    tag,
                    classes,
                    attributes,
                    children: replaced_children,
                }
            }
        }
        ViewElement::ForEach {
            item_var,
            list_expr,
            body,
        } => {
            let replaced_body = body
                .into_iter()
                .map(|c| replace_slots(c, slot_children))
                .collect();
            ViewElement::ForEach {
                item_var,
                list_expr,
                body: replaced_body,
            }
        }
        ViewElement::IfBlock {
            condition,
            then_branch,
            else_branch,
        } => {
            let replaced_then = then_branch
                .into_iter()
                .map(|c| replace_slots(c, slot_children))
                .collect();
            let replaced_else = else_branch.map(|eb| {
                eb.into_iter()
                    .map(|c| replace_slots(c, slot_children))
                    .collect()
            });
            ViewElement::IfBlock {
                condition,
                then_branch: replaced_then,
                else_branch: replaced_else,
            }
        }
        _ => element,
    }
}

fn collect_used_components(
    element: &ViewElement,
    components_map: &std::collections::BTreeMap<String, ComponentDecl>,
    used: &mut std::collections::BTreeSet<String>,
) {
    match element {
        ViewElement::Element { tag, children, .. } => {
            if components_map.contains_key(tag) {
                used.insert(tag.clone());
            }
            for child in children {
                collect_used_components(child, components_map, used);
            }
        }
        ViewElement::ForEach { body, .. } => {
            for child in body {
                collect_used_components(child, components_map, used);
            }
        }
        ViewElement::IfBlock {
            then_branch,
            else_branch,
            ..
        } => {
            for child in then_branch {
                collect_used_components(child, components_map, used);
            }
            if let Some(eb) = else_branch {
                for child in eb {
                    collect_used_components(child, components_map, used);
                }
            }
        }
        _ => {}
    }
}
