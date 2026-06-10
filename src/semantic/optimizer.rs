// src/semantic/optimizer.rs
use crate::ast::*;

/// Optimizes the entire AST tree, executing constant folding and dead code elimination (DCE) for models.
pub fn optimize_ast(nodes: Vec<AmanaNode>) -> Vec<AmanaNode> {
    // 1. Constant folding
    let mut optimized_nodes = Vec::new();
    for node in nodes {
        match node {
            AmanaNode::View(mut view) => {
                view.server_fetches = view
                    .server_fetches
                    .into_iter()
                    .map(|mut f| {
                        f.query_args = f
                            .query_args
                            .into_iter()
                            .map(|(k, expr)| (k, fold_constants(expr)))
                            .collect();
                        f
                    })
                    .collect();

                if let Some(mut protected) = view.protected {
                    protected.allow_expr = fold_constants(protected.allow_expr);
                    view.protected = Some(protected);
                }

                view.client_states = view
                    .client_states
                    .into_iter()
                    .map(|mut s| {
                        s.initial_value = fold_constants(s.initial_value);
                        s
                    })
                    .collect();

                if let Some(body) = view.render_body {
                    view.render_body = Some(fold_constants_element(body));
                }

                optimized_nodes.push(AmanaNode::View(view));
            }
            AmanaNode::App(app) => {
                optimized_nodes.push(AmanaNode::App(app));
            }
            AmanaNode::Model(model) => {
                optimized_nodes.push(AmanaNode::Model(model));
            }
            AmanaNode::Theme(theme) => {
                optimized_nodes.push(AmanaNode::Theme(theme));
            }
            AmanaNode::Route(route) => {
                optimized_nodes.push(AmanaNode::Route(route));
            }
            AmanaNode::Component(mut comp) => {
                if let Some(body) = comp.render_body {
                    comp.render_body = Some(fold_constants_element(body));
                }
                optimized_nodes.push(AmanaNode::Component(comp));
            }
            AmanaNode::Seed(mut seed) => {
                seed.rows = seed
                    .rows
                    .into_iter()
                    .map(|row| {
                        row.into_iter()
                            .map(|(field, expr)| (field, fold_constants(expr)))
                            .collect()
                    })
                    .collect();
                optimized_nodes.push(AmanaNode::Seed(seed));
            }
            AmanaNode::Variant(v) => {
                optimized_nodes.push(AmanaNode::Variant(v));
            }
            AmanaNode::Tokens(t) => {
                optimized_nodes.push(AmanaNode::Tokens(t));
            }
        }
    }

    // 2. Dead Code Elimination (DCE) for models
    let mut used_models = std::collections::HashSet::new();
    let mut auth_model = String::new();

    for node in &optimized_nodes {
        if let AmanaNode::App(app) = node {
            auth_model = app.auth_model.clone();
        }
    }
    if !auth_model.is_empty() {
        used_models.insert(auth_model);
    }

    for node in &optimized_nodes {
        if let AmanaNode::View(view) = node {
            for fetch in &view.server_fetches {
                if fetch.model_name != "time"
                    && fetch.model_name != "http"
                    && fetch.model_name != "auth"
                {
                    used_models.insert(fetch.model_name.clone());
                }
            }
            if let Some(body) = &view.render_body {
                collect_used_models_from_element(body, &mut used_models);
            }
        }
        if let AmanaNode::Seed(seed) = node {
            used_models.insert(seed.model_name.clone());
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for node in &optimized_nodes {
            if let AmanaNode::Model(model) = node
                && used_models.contains(&model.name)
            {
                for field in &model.fields {
                    if let Some((target_model, _)) = &field.foreign_key
                        && used_models.insert(target_model.clone())
                    {
                        changed = true;
                    }
                }
            }
        }
    }

    optimized_nodes
        .into_iter()
        .filter(|node| {
            if let AmanaNode::Model(model) = node {
                used_models.contains(&model.name)
            } else {
                true
            }
        })
        .collect()
}

fn collect_used_models_from_element(
    element: &ViewElement,
    used: &mut std::collections::HashSet<String>,
) {
    match element {
        ViewElement::FormBlock { connect_action, .. } => {
            let parts: Vec<&str> = connect_action.split('.').collect();
            if !parts.is_empty() {
                used.insert(parts[0].to_string());
            }
        }
        ViewElement::Element { children, .. } => {
            for child in children {
                collect_used_models_from_element(child, used);
            }
        }
        ViewElement::ForEach { body, .. } => {
            for child in body {
                collect_used_models_from_element(child, used);
            }
        }
        ViewElement::IfBlock {
            then_branch,
            else_branch,
            ..
        } => {
            for child in then_branch {
                collect_used_models_from_element(child, used);
            }
            if let Some(branch) = else_branch {
                for child in branch {
                    collect_used_models_from_element(child, used);
                }
            }
        }
        ViewElement::ResourceGrid { empty_element, loading_element, error_element, .. }
        | ViewElement::ResourceTable { empty_element, loading_element, error_element, .. } => {
            if let Some(nodes) = empty_element {
                for child in nodes {
                    collect_used_models_from_element(child, used);
                }
            }
            if let Some(nodes) = loading_element {
                for child in nodes {
                    collect_used_models_from_element(child, used);
                }
            }
            if let Some(nodes) = error_element {
                for child in nodes {
                    collect_used_models_from_element(child, used);
                }
            }
        }
        _ => {}
    }
}

/// Recursively evaluates and folds numeric, string, and boolean constant operations in an AST expression.
pub fn fold_constants(expr: Expression) -> Expression {
    match expr {
        Expression::Binary { left, op, right } => {
            let left_folded = fold_constants(*left);
            let right_folded = fold_constants(*right);

            match (&left_folded, op.as_str(), &right_folded) {
                (Expression::Number(nl), "+", Expression::Number(nr)) => {
                    Expression::Number(nl + nr)
                }
                (Expression::Number(nl), "-", Expression::Number(nr)) => {
                    Expression::Number(nl - nr)
                }
                (Expression::Number(nl), "*", Expression::Number(nr)) => {
                    Expression::Number(nl * nr)
                }
                (Expression::Number(nl), "/", Expression::Number(nr)) if *nr != 0.0 => {
                    Expression::Number(nl / nr)
                }

                (Expression::StringLiteral(sl), "+", Expression::StringLiteral(sr)) => {
                    Expression::StringLiteral(format!("{}{}", sl, sr))
                }

                (Expression::Boolean(bl), "and", Expression::Boolean(br)) => {
                    Expression::Boolean(*bl && *br)
                }
                (Expression::Boolean(bl), "or", Expression::Boolean(br)) => {
                    Expression::Boolean(*bl || *br)
                }

                _ => Expression::Binary {
                    left: Box::new(left_folded),
                    op,
                    right: Box::new(right_folded),
                },
            }
        }
        Expression::Unary { op, expr } => {
            let expr_folded = fold_constants(*expr);
            match (op.as_str(), &expr_folded) {
                ("not", Expression::Boolean(b)) => Expression::Boolean(!b),
                ("-", Expression::Number(n)) => Expression::Number(-n),
                _ => Expression::Unary {
                    op,
                    expr: Box::new(expr_folded),
                },
            }
        }
        Expression::Ternary {
            cond,
            then_branch,
            else_branch,
        } => {
            let cond_folded = fold_constants(*cond);
            match &cond_folded {
                Expression::Boolean(true) => fold_constants(*then_branch),
                Expression::Boolean(false) => fold_constants(*else_branch),
                _ => Expression::Ternary {
                    cond: Box::new(cond_folded),
                    then_branch: Box::new(fold_constants(*then_branch)),
                    else_branch: Box::new(fold_constants(*else_branch)),
                },
            }
        }
        Expression::Call { callee, args } => Expression::Call {
            callee: Box::new(fold_constants(*callee)),
            args: args.into_iter().map(fold_constants).collect(),
        },
        Expression::MemberAccess { object, property } => Expression::MemberAccess {
            object: Box::new(fold_constants(*object)),
            property,
        },
        _ => expr,
    }
}

fn fold_constants_element(element: ViewElement) -> ViewElement {
    match element {
        ViewElement::Element {
            tag,
            classes,
            attributes,
            children,
        } => ViewElement::Element {
            tag,
            classes,
            attributes: attributes
                .into_iter()
                .map(|(k, e)| (k, fold_constants(e)))
                .collect(),
            children: children.into_iter().map(fold_constants_element).collect(),
        },
        ViewElement::FormattedText(exprs) => {
            ViewElement::FormattedText(exprs.into_iter().map(fold_constants).collect())
        }
        ViewElement::FormBlock {
            fields,
            connect_action,
            redirect_success,
            defaults,
            constraints,
            ui,
            submit_label,
            field_options,
        } => ViewElement::FormBlock {
            fields,
            connect_action,
            redirect_success,
            defaults: defaults
                .into_iter()
                .map(|(field, expr)| (field, fold_constants(expr)))
                .collect(),
            constraints: constraints
                .into_iter()
                .map(|(field, expr)| (field, fold_constants(expr)))
                .collect(),
            ui,
            submit_label,
            field_options,
        },
        ViewElement::ForEach {
            item_var,
            list_expr,
            body,
        } => ViewElement::ForEach {
            item_var,
            list_expr: fold_constants(list_expr),
            body: body.into_iter().map(fold_constants_element).collect(),
        },
        ViewElement::IfBlock {
            condition,
            then_branch,
            else_branch,
        } => ViewElement::IfBlock {
            condition: fold_constants(condition),
            then_branch: then_branch
                .into_iter()
                .map(fold_constants_element)
                .collect(),
            else_branch: else_branch.map(|b| b.into_iter().map(fold_constants_element).collect()),
        },
        ViewElement::ResourceGrid {
            resource_expr,
            item_component,
            item_arg_name,
            empty_element,
            loading_element,
            error_element,
            filter_fields,
            sort_fields,
        } => ViewElement::ResourceGrid {
            resource_expr: fold_constants(resource_expr),
            item_component,
            item_arg_name,
            empty_element: empty_element.map(|nodes| nodes.into_iter().map(fold_constants_element).collect()),
            loading_element: loading_element.map(|nodes| nodes.into_iter().map(fold_constants_element).collect()),
            error_element: error_element.map(|nodes| nodes.into_iter().map(fold_constants_element).collect()),
            filter_fields,
            sort_fields,
        },
        ViewElement::ResourceTable {
            resource_expr,
            item_component,
            item_arg_name,
            empty_element,
            loading_element,
            error_element,
            filter_fields,
            sort_fields,
        } => ViewElement::ResourceTable {
            resource_expr: fold_constants(resource_expr),
            item_component,
            item_arg_name,
            empty_element: empty_element.map(|nodes| nodes.into_iter().map(fold_constants_element).collect()),
            loading_element: loading_element.map(|nodes| nodes.into_iter().map(fold_constants_element).collect()),
            error_element: error_element.map(|nodes| nodes.into_iter().map(fold_constants_element).collect()),
            filter_fields,
            sort_fields,
        },
        _ => element,
    }
}
