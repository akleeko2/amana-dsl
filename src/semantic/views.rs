// src/semantic/views.rs
use super::SemanticAnalyzer;
use crate::ast::*;

impl SemanticAnalyzer {
    /// Validates all queries, client states, security permissions, and HTML layout of a View declaration.
    pub fn validate_view(&mut self, view: &ViewDecl) -> Result<(), String> {
        self.enter_scope();
        self.in_render_block = false;
        self.current_view_is_protected = view.protected.is_some();

        // 1. Declare variables resulting from server-side queries
        for fetch in &view.server_fetches {
            for (_, expr) in &fetch.query_args {
                self.check_expression_type(expr)?;
            }

            let fetch_type = if fetch.model_name == "time"
                || fetch.model_name == "http"
                || fetch.model_name == "auth"
            {
                if fetch.model_name == "time" && !self.capabilities.contains(&"time".to_string()) {
                    return Err(
                        "Missing capability 'time' required to use 'time' standard library"
                            .to_string(),
                    );
                }
                if fetch.model_name == "http"
                    && !self.capabilities.contains(&"network.outbound".to_string())
                {
                    return Err("Missing capability 'network.outbound' required to use 'http' standard library".to_string());
                }
                if fetch.model_name == "auth" && !self.capabilities.contains(&"auth".to_string()) {
                    return Err(
                        "Missing capability 'auth' required to use 'auth' standard library"
                            .to_string(),
                    );
                }
                DataType::Str
            } else {
                if !self.models.contains_key(&fetch.model_name) {
                    return Err(format!(
                        "Fetch target model '{}' does not exist in schema.",
                        fetch.model_name
                    ));
                }
                match fetch.query_method.as_str() {
                    "all" | "filter" => {
                        DataType::List(Box::new(DataType::Model(fetch.model_name.clone())))
                    }
                    "find" => DataType::Model(fetch.model_name.clone()),
                    "count" => DataType::Int,
                    _ => return Err(format!("Unknown query method '{}'", fetch.query_method)),
                }
            };

            self.declare_symbol(&fetch.var_name, fetch_type);
        }

        // 2. Declare client state variables
        for state in &view.client_states {
            let state_type = self.check_expression_type(&state.initial_value)?;
            self.declare_symbol(&state.name, state_type);
        }

        // 3. Verify security access expression type
        if let Some(protected) = &view.protected {
            let allow_type = self.check_expression_type(&protected.allow_expr)?;
            if allow_type != DataType::Bool {
                return Err(format!(
                    "Protected allow condition must be boolean, got {:?}",
                    allow_type
                ));
            }
        }

        // 4. Validate UI view elements recursively
        if let Some(render_body) = &view.render_body {
            self.in_render_block = true;
            self.validate_view_element(render_body)?;
            self.in_render_block = false;
        }
        if let Some(canvas) = &view.canvas {
            self.validate_design_block(canvas)?;
        }

        self.current_view_is_protected = false;
        self.exit_scope();
        Ok(())
    }

    /// Validates semantic constraints of a specific ViewElement node.
    fn validate_view_element(&mut self, element: &ViewElement) -> Result<(), String> {
        match element {
            ViewElement::Element {
                tag,
                children,
                attributes,
                ..
            } => {
                let tag_lower = tag.to_lowercase();
                const BLOCKED_HTML_TAGS: &[&str] = &[
                    "script", "iframe", "object", "embed", "applet", "link", "meta", "base",
                    "style", "noscript",
                ];
                if !tag.chars().next().is_some_and(|c| c.is_uppercase())
                    && BLOCKED_HTML_TAGS.contains(&tag_lower.as_str())
                {
                    return Err(format!(
                        "Security: HTML tag <{}> is not allowed in Amana views. Use Amana components or the style: block instead.",
                        tag
                    ));
                }

                if let Some(comp) = self.components.get(tag).cloned() {
                    // Check parameter count and types
                    for param in &comp.params {
                        let arg = attributes.iter().find(|(k, _)| k == &param.name);
                        if let Some((_, arg_expr)) = arg {
                            if let Some(ref ty_str) = param.ty {
                                let arg_ty = self.check_expression_type(arg_expr)?;
                                let expected_ty = match ty_str.as_str() {
                                    "str" | "string" => DataType::Str,
                                    "int" | "integer" => DataType::Int,
                                    "float" | "double" => DataType::Float,
                                    "bool" | "boolean" => DataType::Bool,
                                    _ => DataType::Custom(ty_str.clone()),
                                };
                                if !self.types_compatible(&expected_ty, &arg_ty) {
                                    return Err(format!(
                                        "Component '{}' parameter '{}' expects type {:?}, but got type {:?}",
                                        tag, param.name, expected_ty, arg_ty
                                    ));
                                }
                            }
                        } else if param.required {
                            return Err(format!(
                                "Component '{}' requires parameter '{}', but it was not provided.",
                                tag, param.name
                            ));
                        }
                    }

                    // Check required slots
                    if let Some(ref body) = comp.render_body {
                        let mut required_slots = Vec::new();
                        collect_required_slots(body, &mut required_slots);
                        let slots_clone = required_slots.clone();
                        for slot in required_slots {
                            let has_slot = if slot == "default" {
                                children.iter().any(|child| {
                                    if let ViewElement::Element { tag: child_tag, .. } = child {
                                        child_tag == "default"
                                            || !slots_clone
                                                .iter()
                                                .any(|s| s != "default" && s == child_tag)
                                    } else {
                                        true
                                    }
                                })
                            } else {
                                children.iter().any(|child| {
                                    if let ViewElement::Element { tag: child_tag, .. } = child {
                                        child_tag == &slot
                                    } else {
                                        false
                                    }
                                })
                            };
                            if !has_slot {
                                return Err(format!(
                                    "Component '{}' call requires slot '{}', but no child matches this slot name.",
                                    tag, slot
                                ));
                            }
                        }
                    }

                    let mut arg_types = Vec::new();
                    for (k, expr) in attributes {
                        let ty = self.check_expression_type(expr)?;
                        arg_types.push((k.clone(), ty));
                    }
                    for child in children {
                        self.validate_view_element(child)?;
                    }
                    self.enter_scope();
                    for (k, ty) in arg_types {
                        self.declare_symbol(&k, ty);
                    }
                    if let Some(ref body) = comp.render_body {
                        self.validate_view_element(body)?;
                    }
                    self.exit_scope();
                } else {
                    for (_, expr) in attributes {
                        self.check_expression_type(expr)?;
                    }
                    for child in children {
                        self.validate_view_element(child)?;
                    }
                }
            }
            ViewElement::Text(_) => {}
            ViewElement::Accordion { panels } => {
                for (_, panel_body) in panels {
                    for child in panel_body {
                        self.validate_view_element(child)?;
                    }
                }
            }
            ViewElement::Tabs { tabs } => {
                for (_, tab_body) in tabs {
                    for child in tab_body {
                        self.validate_view_element(child)?;
                    }
                }
            }
            ViewElement::CollapseSection { body, .. } => {
                for child in body {
                    self.validate_view_element(child)?;
                }
            }
            ViewElement::DesignBlock(block) => {
                self.validate_design_block(block)?;
            }
            ViewElement::FormattedText(exprs) => {
                for expr in exprs {
                    self.check_expression_type(expr)?;
                }
            }
            ViewElement::ForEach {
                item_var,
                list_expr,
                body,
            } => {
                let list_type = self.check_expression_type(list_expr)?;
                match list_type {
                    DataType::List(inner_type) => {
                        self.enter_scope();
                        self.declare_symbol(item_var, *inner_type);
                        for child in body {
                            self.validate_view_element(child)?;
                        }
                        self.exit_scope();
                    }
                    _ => {
                        return Err(format!(
                            "ForEach list expression must be a list, got {:?}",
                            list_type
                        ));
                    }
                }
            }
            ViewElement::IfBlock {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_type = self.check_expression_type(condition)?;
                if cond_type != DataType::Bool {
                    return Err(format!("If condition must be boolean, got {:?}", cond_type));
                }
                for child in then_branch {
                    self.validate_view_element(child)?;
                }
                if let Some(branch) = else_branch {
                    for child in branch {
                        self.validate_view_element(child)?;
                    }
                }
            }
            ViewElement::FormBlock {
                fields,
                connect_action,
                defaults,
                constraints,
                field_options,
                ..
            } => {
                let parts: Vec<&str> = connect_action.split('.').collect();
                if parts.len() != 2 {
                    return Err(format!("Invalid form connect action: '{}'", connect_action));
                }
                let model_name = parts[0];
                let action = parts[1].to_lowercase();
                let allowed_actions = ["create", "update", "delete", "login", "register", "logout"];
                if !allowed_actions.contains(&action.as_str()) {
                    return Err(format!(
                        "Unsupported form action '{}'. Allowed actions are: {}",
                        connect_action,
                        allowed_actions.join(", ")
                    ));
                }
                if (action == "login" || action == "register" || action == "logout")
                    && model_name != self.auth_model
                {
                    return Err(format!(
                        "Authentication action '{}' must target configured auth model '{}'",
                        connect_action, self.auth_model
                    ));
                }
                if action == "login" {
                    let has_email = fields.iter().any(|f| f.eq_ignore_ascii_case("email"));
                    let has_password = fields.iter().any(|f| f.eq_ignore_ascii_case("password"));
                    if !has_email || !has_password {
                        return Err(
                            "Login forms must include 'email' and 'password' fields.".to_string()
                        );
                    }
                }
                let model = self.models.get(model_name).ok_or_else(|| {
                    format!("Model '{}' connected to form does not exist.", model_name)
                })?;

                for field_name in fields {
                    let field_exists = model.fields.iter().any(|f| f.name == *field_name)
                        || field_name.to_lowercase() == "id";
                    if !field_exists {
                        return Err(format!(
                            "Field '{}' specified in form does not exist in model '{}'",
                            field_name, model_name
                        ));
                    }
                }
                for option in field_options {
                    if !fields.iter().any(|f| f.eq_ignore_ascii_case(&option.name)) {
                        return Err(format!(
                            "Field UI options reference '{}' but it is not listed in form fields.",
                            option.name
                        ));
                    }
                    if Self::field_type_for(model, &option.name).is_none() {
                        return Err(format!(
                            "Field UI options reference '{}' but it does not exist in model '{}'.",
                            option.name, model_name
                        ));
                    }
                }

                for (field_name, value_expr) in defaults {
                    let field_type = Self::field_type_for(model, field_name).ok_or_else(|| {
                        format!(
                            "Default field '{}' specified in form does not exist in model '{}'",
                            field_name, model_name
                        )
                    })?;
                    let value_type = self.check_expression_type(value_expr)?;
                    if !self.types_compatible(&field_type, &value_type) {
                        return Err(format!(
                            "Default field '{}' expects {:?}, got {:?}",
                            field_name, field_type, value_type
                        ));
                    }
                    if self.expression_uses_current_user(value_expr)
                        && !self.current_view_is_protected
                    {
                        return Err(format!(
                            "Form default '{} = ...' uses {}.current and must be inside a protected view.",
                            field_name, self.auth_model
                        ));
                    }
                }

                if !constraints.is_empty() && action != "update" && action != "delete" {
                    return Err(format!(
                        "Form where constraints are only supported for update/delete actions, not '{}'.",
                        action
                    ));
                }
                for (field_name, value_expr) in constraints {
                    let field_type = Self::field_type_for(model, field_name).ok_or_else(|| {
                        format!(
                            "Where field '{}' specified in form does not exist in model '{}'",
                            field_name, model_name
                        )
                    })?;
                    let value_type = self.check_expression_type(value_expr)?;
                    if !self.types_compatible(&field_type, &value_type) {
                        return Err(format!(
                            "Where field '{}' expects {:?}, got {:?}",
                            field_name, field_type, value_type
                        ));
                    }
                    if self.expression_uses_current_user(value_expr)
                        && !self.current_view_is_protected
                    {
                        return Err(format!(
                            "Form where '{} = ...' uses {}.current and must be inside a protected view.",
                            field_name, self.auth_model
                        ));
                    }
                }
            }
            ViewElement::Chart {
                data_expr,
                x_field,
                y_field,
                ..
            } => {
                let data_type = self
                    .resolve_symbol(data_expr)
                    .map(|sym| sym.data_type.clone())
                    .ok_or_else(|| {
                        if let Some(suggested) = self.suggest_similar_variable(data_expr) {
                            format!(
                                "Chart data variable '{}' is not defined. Did you mean '{}'?",
                                data_expr, suggested
                            )
                        } else {
                            format!("Chart data variable '{}' is not defined.", data_expr)
                        }
                    })?;

                match data_type {
                    DataType::List(inner_type) => {
                        if let DataType::Model(model_name) = *inner_type {
                            let model = self.models.get(&model_name).unwrap();
                            let x_exists = model.fields.iter().any(|f| f.name == *x_field);
                            let y_exists = model.fields.iter().any(|f| f.name == *y_field);
                            if !x_exists || !y_exists {
                                return Err(format!(
                                    "Chart columns '{}' or '{}' do not exist in model '{}'",
                                    x_field, y_field, model_name
                                ));
                            }
                        }
                    }
                    _ => return Err("Chart data must be a list of models.".to_string()),
                }
            }
            ViewElement::SlotDecl { .. } => {}
            ViewElement::ResourceGrid {
                resource_expr,
                item_component,
                item_arg_name,
                empty_element,
                loading_element,
                error_element,
                filter_fields: _,
                sort_fields: _,
            }
            | ViewElement::ResourceTable {
                resource_expr,
                item_component,
                item_arg_name,
                empty_element,
                loading_element,
                error_element,
                filter_fields: _,
                sort_fields: _,
            } => {
                let list_type = self.check_expression_type(resource_expr)?;
                match list_type {
                    DataType::List(inner_type) => {
                        // Check if the component item_component exists (if component registry has it)
                        let _comp = self.components.get(item_component).ok_or_else(|| {
                            format!(
                                "Component '{}' referenced in Resource block does not exist.",
                                item_component
                            )
                        })?;

                        self.enter_scope();
                        self.declare_symbol(item_arg_name, *inner_type);

                        if let Some(nodes) = empty_element {
                            for node in nodes {
                                self.validate_view_element(node)?;
                            }
                        }
                        if let Some(nodes) = loading_element {
                            for node in nodes {
                                self.validate_view_element(node)?;
                            }
                        }
                        if let Some(nodes) = error_element {
                            for node in nodes {
                                self.validate_view_element(node)?;
                            }
                        }
                        self.exit_scope();
                    }
                    _ => {
                        return Err(format!(
                            "Resource block expression must be a list, got {:?}",
                            list_type
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

fn collect_required_slots(element: &ViewElement, required_slots: &mut Vec<String>) {
    match element {
        ViewElement::SlotDecl { name, optional } => {
            if !*optional {
                required_slots.push(name.clone());
            }
        }
        ViewElement::Element { children, .. } => {
            for child in children {
                collect_required_slots(child, required_slots);
            }
        }
        ViewElement::ForEach { body, .. } => {
            for child in body {
                collect_required_slots(child, required_slots);
            }
        }
        ViewElement::Accordion { panels } => {
            for (_, panel_body) in panels {
                for child in panel_body {
                    collect_required_slots(child, required_slots);
                }
            }
        }
        ViewElement::Tabs { tabs } => {
            for (_, tab_body) in tabs {
                for child in tab_body {
                    collect_required_slots(child, required_slots);
                }
            }
        }
        ViewElement::CollapseSection { body, .. } => {
            for child in body {
                collect_required_slots(child, required_slots);
            }
        }
        ViewElement::IfBlock {
            then_branch,
            else_branch,
            ..
        } => {
            for child in then_branch {
                collect_required_slots(child, required_slots);
            }
            if let Some(nodes) = else_branch {
                for child in nodes {
                    collect_required_slots(child, required_slots);
                }
            }
        }
        ViewElement::ResourceGrid {
            empty_element,
            loading_element,
            error_element,
            ..
        }
        | ViewElement::ResourceTable {
            empty_element,
            loading_element,
            error_element,
            ..
        } => {
            if let Some(nodes) = empty_element {
                for child in nodes {
                    collect_required_slots(child, required_slots);
                }
            }
            if let Some(nodes) = loading_element {
                for child in nodes {
                    collect_required_slots(child, required_slots);
                }
            }
            if let Some(nodes) = error_element {
                for child in nodes {
                    collect_required_slots(child, required_slots);
                }
            }
        }
        _ => {}
    }
}
