// src/semantic/mod.rs
pub mod ir;
pub mod ir_gen;
pub mod optimizer;
use crate::ast::*;
use std::collections::BTreeMap;

/// Computes the Levenshtein distance between two strings.
/// This algorithm calculates the minimum number of single-character edits (insertions, deletions or substitutions)
/// required to change one word into the other.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let len_a = a_chars.len();
    let len_b = b_chars.len();

    let mut dp = vec![vec![0; len_b + 1]; len_a + 1];

    for (i, row) in dp.iter_mut().enumerate().take(len_a + 1) {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate().take(len_b + 1) {
        *cell = j;
    }

    for i in 1..=len_a {
        for j in 1..=len_b {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = std::cmp::min(
                std::cmp::min(dp[i - 1][j] + 1, dp[i][j - 1] + 1),
                dp[i - 1][j - 1] + cost,
            );
        }
    }

    dp[len_a][len_b]
}

/// Suggests a similar field name in a database model declaration if there is a close match.
fn suggest_similar_field(name: &str, model: &ModelDecl) -> Option<String> {
    let mut best_candidate = None;
    let mut min_distance = 3;
    for field in &model.fields {
        let dist = levenshtein_distance(name, &field.name);
        if dist < min_distance {
            min_distance = dist;
            best_candidate = Some(field.name.clone());
        }
    }
    best_candidate
}

/// Represents a variable or function symbol registered in Amana's symbol table.
#[derive(Clone, Debug)]
pub struct Symbol {
    /// The name of the symbol.
    pub name: String,
    /// The resolved type of the symbol.
    pub data_type: DataType,
}

/// Represents a scoping level in Amana containing local variables and parent scope references.
pub struct Scope {
    /// Reference to the parent scope's index in the allocator vector, if any.
    pub parent: Option<usize>,
    /// Map of symbol names to their respective registered Symbol details.
    pub symbols: BTreeMap<String, Symbol>,
}

/// Represents the database schema for a registered Model.
pub struct TableSchema {
    /// The original model name.
    pub model_name: String,
    /// The physical database table name.
    pub table_name: String,
    /// Map of columns and their corresponding data types.
    pub columns: BTreeMap<String, DataType>,
}

/// Context structure maintaining all whitelisted database table structures.
pub struct SchemaContext {
    /// Mapping from lowercase table names to their table schema.
    pub tables: BTreeMap<String, TableSchema>,
}

impl SchemaContext {
    /// Creates a new SchemaContext mapping and registers implicit fields (e.g. ID).
    pub fn new(models: &[ModelDecl]) -> Self {
        let mut tables = BTreeMap::new();
        for m in models {
            let mut columns = BTreeMap::new();
            for f in &m.fields {
                columns.insert(f.name.to_lowercase(), f.data_type.clone());
            }
            // Add implicit primary key id field
            columns.insert("id".to_string(), DataType::Int);

            let table_name = m.name.to_lowercase();
            tables.insert(
                table_name.clone(),
                TableSchema {
                    model_name: m.name.clone(),
                    table_name,
                    columns,
                },
            );
        }
        Self { tables }
    }

    /// Verifies if a given table name is whitelisted in the application schema.
    pub fn is_whitelisted(&self, table_name: &str) -> bool {
        self.tables.contains_key(&table_name.to_lowercase())
    }
}

/// Semantic analyzer responsible for scope resolution, standard library permission checks, and type validation.
pub struct SemanticAnalyzer {
    /// Allocator vector for nested lexically-scoped Symbol tables.
    pub scopes: Vec<Scope>,
    /// Index of the current active Scope.
    pub current_scope: usize,
    /// Map of all declared database models.
    pub models: BTreeMap<String, ModelDecl>,
    /// Map of all declared reusable components.
    pub components: BTreeMap<String, ComponentDecl>,
    /// Table structures and constraints context.
    pub schema_context: SchemaContext,
    /// The model name chosen for user authentication (e.g. User).
    pub auth_model: String,
    /// Granted application capability list (e.g. time, auth, network.outbound).
    pub capabilities: Vec<String>,
    /// Indicates whether typecheck is running inside a View render block (to enforce view boundaries).
    pub in_render_block: bool,
    /// Indicates whether the current view has an access guard before evaluating render/form policy.
    pub current_view_is_protected: bool,
}

impl SemanticAnalyzer {
    /// Instantiates a new SemanticAnalyzer, registering the initial global scope and capabilities.
    pub fn new(
        models: &[ModelDecl],
        auth_model: &str,
        capabilities: &[String],
        components: &[ComponentDecl],
    ) -> Self {
        let mut model_map = BTreeMap::new();
        for m in models {
            model_map.insert(m.name.clone(), m.clone());
        }

        let mut component_map = BTreeMap::new();
        for c in components {
            component_map.insert(c.name.clone(), c.clone());
        }

        let schema_context = SchemaContext::new(models);
        let mut analyzer = Self {
            scopes: vec![Scope {
                parent: None,
                symbols: BTreeMap::new(),
            }],
            current_scope: 0,
            models: model_map,
            components: component_map,
            schema_context,
            auth_model: auth_model.to_string(),
            capabilities: capabilities.to_vec(),
            in_render_block: false,
            current_view_is_protected: false,
        };

        // Initialize global variables like the current authenticated user session
        if !auth_model.is_empty() {
            analyzer.declare_symbol(auth_model, DataType::Model(auth_model.to_string()));
        }
        analyzer.declare_symbol("csrfToken", DataType::Str);
        analyzer.declare_symbol("params", DataType::Custom("RequestMap".to_string()));
        analyzer.declare_symbol("query", DataType::Custom("RequestMap".to_string()));
        analyzer.declare_symbol("body", DataType::Custom("RequestMap".to_string()));

        analyzer
    }

    /// Enters a new nested scope block.
    pub fn enter_scope(&mut self) {
        let new_id = self.scopes.len();
        self.scopes.push(Scope {
            parent: Some(self.current_scope),
            symbols: BTreeMap::new(),
        });
        self.current_scope = new_id;
    }

    /// Exits the current scope, returning to its parent.
    pub fn exit_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            self.current_scope = parent;
        }
    }

    /// Declares a new Symbol inside the current active Scope.
    pub fn declare_symbol(&mut self, name: &str, data_type: DataType) {
        self.scopes[self.current_scope].symbols.insert(
            name.to_string(),
            Symbol {
                name: name.to_string(),
                data_type,
            },
        );
    }

    /// Resolves a symbol by searching up the nested scopes hierarchy.
    pub fn resolve_symbol(&self, name: &str) -> Option<&Symbol> {
        let mut curr = self.current_scope;
        loop {
            if let Some(sym) = self.scopes[curr].symbols.get(name) {
                return Some(sym);
            }
            if let Some(parent) = self.scopes[curr].parent {
                curr = parent;
            } else {
                break;
            }
        }
        None
    }

    /// Retrieves all visible variable and function names within the current scope hierarchy.
    pub fn get_all_symbols_in_scope(&self) -> Vec<String> {
        let mut symbols = vec![
            "time".to_string(),
            "http".to_string(),
            "auth".to_string(),
            "env".to_string(),
        ];

        let mut curr = self.current_scope;
        loop {
            for name in self.scopes[curr].symbols.keys() {
                symbols.push(name.clone());
            }
            if let Some(parent) = self.scopes[curr].parent {
                curr = parent;
            } else {
                break;
            }
        }
        symbols
    }

    /// Finds and returns the closest matching symbol in scope if within Levenshtein threshold.
    pub fn suggest_similar_variable(&self, name: &str) -> Option<String> {
        let candidates = self.get_all_symbols_in_scope();
        let mut best_candidate = None;
        let mut min_distance = 3;

        for candidate in candidates {
            let dist = levenshtein_distance(name, &candidate);
            if dist < min_distance {
                min_distance = dist;
                best_candidate = Some(candidate);
            }
        }
        best_candidate
    }



    /// Checks if a standard library identifier can be accessed under the current configuration.
    fn check_std_access(&self, name: &str) -> Result<DataType, String> {
        if name == "time" {
            if !self.capabilities.contains(&"time".to_string()) {
                return Err(
                    "Missing capability 'time' required to use 'time' standard library".to_string(),
                );
            }
            if self.in_render_block {
                return Err("Standard library 'time' is not allowed in render blocks. Views must be pure rendering layers.".to_string());
            }
            return Ok(DataType::Custom("StdTime".to_string()));
        }
        if name == "http" {
            if !self.capabilities.contains(&"network.outbound".to_string()) {
                return Err(
                    "Missing capability 'network.outbound' required to use 'http' standard library"
                        .to_string(),
                );
            }
            if self.in_render_block {
                return Err("Standard library 'http' is not allowed in render blocks. Views must be pure rendering layers.".to_string());
            }
            return Ok(DataType::Custom("StdHttp".to_string()));
        }
        if name == "auth" {
            if !self.capabilities.contains(&"auth".to_string()) {
                return Err(
                    "Missing capability 'auth' required to use 'auth' standard library".to_string(),
                );
            }
            if self.in_render_block {
                return Err("Standard library 'auth' is not allowed in render blocks. Views must be pure rendering layers.".to_string());
            }
            return Ok(DataType::Custom("StdAuth".to_string()));
        }

        let err_msg = if let Some(suggested) = self.suggest_similar_variable(name) {
            format!(
                "Undefined variable: '{}'. Did you mean '{}'?",
                name, suggested
            )
        } else {
            format!("Undefined variable: '{}'", name)
        };
        Err(err_msg)
    }

    /// Evaluates and checks the DataType of an AST Expression node, verifying coercions and rules.
    pub fn check_expression_type(&self, expr: &Expression) -> Result<DataType, String> {
        match expr {
            Expression::Number(_) => Ok(DataType::Float),
            Expression::StringLiteral(_) => Ok(DataType::Str),
            Expression::Boolean(_) => Ok(DataType::Bool),
            Expression::Null => Ok(DataType::Str),

            Expression::Identifier(name) => {
                if name == "time" || name == "http" || name == "auth" {
                    self.check_std_access(name)
                } else if name == "env" {
                    Ok(DataType::Custom("StdEnv".to_string()))
                } else {
                    self.resolve_symbol(name)
                        .map(|sym| sym.data_type.clone())
                        .ok_or_else(|| {
                            if let Some(suggested) = self.suggest_similar_variable(name) {
                                format!(
                                    "Undefined variable: '{}'. Did you mean '{}'?",
                                    name, suggested
                                )
                            } else {
                                format!("Undefined variable: '{}'", name)
                            }
                        })
                }
            }

            Expression::Binary { left, op, right } => {
                let left_type = self.check_expression_type(left)?;
                let right_type = self.check_expression_type(right)?;

                match op.as_str() {
                    "+" | "-" | "*" | "/" => {
                        if self.is_numeric(&left_type) && self.is_numeric(&right_type) {
                            if left_type == DataType::Money || right_type == DataType::Money {
                                Ok(DataType::Money)
                            } else if left_type == DataType::Float || right_type == DataType::Float
                            {
                                Ok(DataType::Float)
                            } else {
                                Ok(DataType::Int)
                            }
                        } else if op == "+"
                            && (left_type == DataType::Str || right_type == DataType::Str)
                        {
                            Ok(DataType::Str)
                        } else {
                            Err(format!(
                                "Arithmetic operation '{}' is not valid for types {:?} and {:?}",
                                op, left_type, right_type
                            ))
                        }
                    }
                    "=" => {
                        match &**left {
                            Expression::Identifier(_) | Expression::MemberAccess { .. } => {}
                            _ => {
                                return Err(
                                    "Left side of assignment must be a variable or property"
                                        .to_string(),
                                );
                            }
                        }
                        if left_type == right_type
                            || (self.is_numeric(&left_type) && self.is_numeric(&right_type))
                            || matches!(**right, Expression::Null)
                            || left_type == DataType::Str
                        {
                            Ok(left_type)
                        } else {
                            Err(format!(
                                "Cannot assign type {:?} to variable of type {:?}",
                                right_type, left_type
                            ))
                        }
                    }
                    "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                        if left_type == right_type
                            || (self.is_numeric(&left_type) && self.is_numeric(&right_type))
                            || matches!(**left, Expression::Null)
                            || matches!(**right, Expression::Null)
                        {
                            Ok(DataType::Bool)
                        } else {
                            Err(format!(
                                "Comparison '{}' is not valid between types {:?} and {:?}",
                                op, left_type, right_type
                            ))
                        }
                    }
                    "and" | "or" => {
                        if left_type == DataType::Bool && right_type == DataType::Bool {
                            Ok(DataType::Bool)
                        } else {
                            Err(format!(
                                "Logical operation '{}' is only valid for boolean operands, got {:?} and {:?}",
                                op, left_type, right_type
                            ))
                        }
                    }
                    _ => Err(format!("Unsupported binary operator '{}'", op)),
                }
            }

            Expression::Unary { op, expr } => {
                let expr_type = self.check_expression_type(expr)?;
                match op.as_str() {
                    "not" | "!" => {
                        if expr_type == DataType::Bool {
                            Ok(DataType::Bool)
                        } else {
                            Err(format!(
                                "Unary operation '{}' requires boolean operand, got {:?}",
                                op, expr_type
                            ))
                        }
                    }
                    "-" => {
                        if self.is_numeric(&expr_type) {
                            Ok(expr_type)
                        } else {
                            Err(format!(
                                "Unary negation requires numeric operand, got {:?}",
                                expr_type
                            ))
                        }
                    }
                    _ => Err(format!("Unsupported unary operator '{}'", op)),
                }
            }

            Expression::MemberAccess { object, property } => {
                let object_type = self.check_expression_type(object)?;
                match object_type {
                    DataType::Custom(ref s)
                        if s == "StdTime" || s == "StdHttp" || s == "StdAuth" =>
                    {
                        Ok(DataType::Custom("StdFunc".to_string()))
                    }
                    DataType::Custom(ref s) if s == "RequestMap" => Ok(DataType::Str),
                    DataType::Model(model_name) => {
                        if model_name == self.auth_model && property == "current" {
                            return Ok(DataType::Model(model_name));
                        }
                        if property == "id" {
                            return Ok(DataType::Int);
                        }

                        let model = self.models.get(&model_name).ok_or_else(|| {
                            format!("Model '{}' not found in database schema.", model_name)
                        })?;

                        let field = model.fields.iter().find(|f| f.name == *property)
                            .ok_or_else(|| {
                                if let Some(suggested) = suggest_similar_field(property, model) {
                                    format!("Property '{}' does not exist in model '{}'. Did you mean '{}'?", property, model_name, suggested)
                                } else {
                                    format!("Property '{}' does not exist in model '{}'", property, model_name)
                                }
                            })?;

                        Ok(field.data_type.clone())
                    }
                    DataType::List(_) => {
                        if property == "length" {
                            Ok(DataType::Int)
                        } else {
                            Err(format!(
                                "Cannot read property '{}' on non-model type {:?}",
                                property, object_type
                            ))
                        }
                    }
                    _ => Err(format!(
                        "Cannot read property '{}' on non-model type {:?}",
                        property, object_type
                    )),
                }
            }

            Expression::Call { callee, args } => {
                let callee_type = self.check_expression_type(callee)?;
                match callee_type {
                    DataType::Custom(ref s) if s == "StdEnv" => {
                        if args.is_empty() || args.len() > 2 {
                            return Err(
                                "Function 'env' expects 1 or 2 string arguments".to_string()
                            );
                        }
                        for arg in args {
                            let arg_type = self.check_expression_type(arg)?;
                            if arg_type != DataType::Str {
                                return Err(format!(
                                    "Function 'env' arguments must be string, got {:?}",
                                    arg_type
                                ));
                            }
                        }
                        Ok(DataType::Str)
                    }
                    DataType::Custom(ref s) if s == "StdFunc" => {
                        for arg in args {
                            self.check_expression_type(arg)?;
                        }
                        Ok(DataType::Str)
                    }
                    DataType::Model(model_name) => Ok(DataType::Model(model_name)),
                    _ => {
                        let _arg_types = args
                            .iter()
                            .map(|arg| self.check_expression_type(arg))
                            .collect::<Result<Vec<DataType>, String>>()?;
                        Ok(DataType::Str)
                    }
                }
            }

            Expression::Ternary {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond_type = self.check_expression_type(cond)?;
                if cond_type != DataType::Bool {
                    return Err(format!(
                        "Ternary condition must be boolean, got {:?}",
                        cond_type
                    ));
                }
                let then_type = self.check_expression_type(then_branch)?;
                let else_type = self.check_expression_type(else_branch)?;
                if then_type == else_type {
                    Ok(then_type)
                } else if self.is_numeric(&then_type) && self.is_numeric(&else_type) {
                    if then_type == DataType::Money || else_type == DataType::Money {
                        Ok(DataType::Money)
                    } else if then_type == DataType::Float || else_type == DataType::Float {
                        Ok(DataType::Float)
                    } else {
                        Ok(DataType::Int)
                    }
                } else if then_type == DataType::Str || else_type == DataType::Str {
                    Ok(DataType::Str)
                } else {
                    Err(format!(
                        "Ternary branches must have matching types, got {:?} and {:?}",
                        then_type, else_type
                    ))
                }
            }
        }
    }

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
                    "script", "iframe", "object", "embed", "applet",
                    "link", "meta", "base", "style", "noscript",
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
                                        child_tag == "default" || !slots_clone.iter().any(|s| s != "default" && s == child_tag)
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
                            format!("Component '{}' referenced in Resource block does not exist.", item_component)
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
            ViewElement::Tabs { tabs } => {
                for (_, tab_body) in tabs {
                    for node in tab_body {
                        self.validate_view_element(node)?;
                    }
                }
            }
            ViewElement::Accordion { panels } => {
                for (_, panel_body) in panels {
                    for node in panel_body {
                        self.validate_view_element(node)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_design_block(&self, block: &DesignBlock) -> Result<(), String> {
        let allowed = [
            "canvas",
            "compose",
            "visual",
            "type",
            "motion",
            "creative",
            "brand",
            "art",
            "responsive",
            "interaction",
            "a11y",
            "component",
            "tokens",
            "states",
        ];
        if !allowed.contains(&block.kind.as_str()) {
            return Err(format!(
                "Unknown design grammar block '{}'. Allowed blocks are: {}.",
                block.kind,
                allowed.join(", ")
            ));
        }
        for (key, value) in &block.settings {
            if key.trim().is_empty() || value.trim().is_empty() {
                return Err(format!(
                    "Design grammar block '{}' contains an empty key or value.",
                    block.kind
                ));
            }
            let lower = value.to_lowercase();
            let blocked = ["javascript:", "expression(", "<script", "</", "behavior:"];
            if blocked.iter().any(|needle| lower.contains(needle)) {
                return Err(format!(
                    "Unsafe design grammar value rejected in '{}.{}': '{}'",
                    block.kind, key, value
                ));
            }
            if value.len() > 240 {
                return Err(format!(
                    "Design grammar value '{}.{}' is too long. Keep it below 240 characters.",
                    block.kind, key
                ));
            }

            let normalized = normalize_design_value(value);

            let (valid_values, prop_display): (&[&str], &str) = match key.as_str() {
                "layout" => (LAYOUT_VALUES, "layout"),
                "surface" => (SURFACE_VALUES, "surface"),
                "hover" => (HOVER_VALUES, "hover"),
                "entrance" | "reveal" => (ENTRANCE_VALUES, "entrance"),
                "gradient" => (GRADIENT_VALUES, "gradient"),
                "density" => (DENSITY_VALUES, "density"),
                "shadow" => (SHADOW_VALUES, "shadow"),
                _ => continue,
            };

            if !valid_values.contains(&normalized.as_str()) {
                let msg = if let Some(suggestion) = suggest_from_list(&normalized, valid_values) {
                    format!(
                        "Unknown {} value \"{}\". Did you mean \"{}\"?",
                        prop_display, value, suggestion
                    )
                } else {
                    format!(
                        "Unknown {} value \"{}\". Valid values: {}.",
                        prop_display, value,
                        valid_values.join(", ")
                    )
                };
                return Err(msg);
            }
        }

        if block.kind == "compose" {
            let layout_type = block.settings.iter().find(|(k, _)| k == "layout").map(|(_, v)| v.as_str());
            if let Some(layout) = layout_type {
                let allowed_keys = match layout {
                    "bento" => vec!["layout", "columns", "rows", "gap", "auto_place", "responsive", "rhythm", "focus_path", "density"],
                    "masonry" => vec!["layout", "columns", "image_ratio", "gap"],
                    "split" => vec!["layout", "ratio", "align", "visual_position"],
                    "asymmetric" => vec!["layout", "rhythm", "dominant", "overlap"],
                    "magazine" => vec!["layout", "columns", "headline_span", "aside_span", "pull_quote"],
                    "sidebar" => vec!["layout", "sidebar_width", "sidebar_position", "sticky_sidebar"],
                    "timeline" => vec!["layout", "axis", "marker", "alternate"],
                    "dashboard-shell" => vec!["layout", "sidebar", "topbar", "content_width", "density", "rhythm"],
                    _ => vec![],
                };
                if !allowed_keys.is_empty() {
                    for (key, _) in &block.settings {
                        if !allowed_keys.contains(&key.as_str()) {
                            let err_msg = if let Some(sug) = suggest_from_list(key, &allowed_keys) {
                                format!("Property '{}' is not valid for layout '{}'. Did you mean '{}'?", key, layout, sug)
                            } else {
                                format!("Property '{}' is not valid for layout '{}'.", key, layout)
                            };
                            return Err(err_msg);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validates seed rows before code generation so `amana check` catches data/schema drift early.
    pub fn validate_seed(&self, seed: &SeedDecl) -> Result<(), String> {
        let model = self.models.get(&seed.model_name).ok_or_else(|| {
            format!(
                "Seed references model '{}' but no such model is declared.",
                seed.model_name
            )
        })?;

        for (row_index, row) in seed.rows.iter().enumerate() {
            let mut seen_fields = std::collections::BTreeSet::new();
            for (field_name, expr) in row {
                if !seen_fields.insert(field_name.to_lowercase()) {
                    return Err(format!(
                        "Seed for model '{}' row {} defines field '{}' more than once.",
                        seed.model_name,
                        row_index + 1,
                        field_name
                    ));
                }

                let expected = Self::field_type_for(model, field_name).ok_or_else(|| {
                    format!(
                        "Seed field '{}' does not exist in model '{}'.",
                        field_name, seed.model_name
                    )
                })?;
                let actual = self.check_expression_type(expr)?;
                if !self.types_compatible(&expected, &actual) {
                    return Err(format!(
                        "Seed field '{}' in model '{}' expects {:?}, got {:?}.",
                        field_name, seed.model_name, expected, actual
                    ));
                }
                if self.expression_uses_current_user(expr) {
                    return Err(format!(
                        "Seed field '{}' in model '{}' cannot use {}.current because seeds run without a request session.",
                        field_name, seed.model_name, self.auth_model
                    ));
                }
            }

            for field in &model.fields {
                if field.is_required
                    && !field.is_primary_key
                    && field.default_value.is_none()
                    && !seen_fields.contains(&field.name.to_lowercase())
                {
                    return Err(format!(
                        "Seed for model '{}' row {} is missing required field '{}'.",
                        seed.model_name,
                        row_index + 1,
                        field.name
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validates global and local variants, and applies CSS sanitization.
    pub fn validate_variant(&self, var: &VariantDecl) -> Result<(), String> {
        let standard_components = [
            // Layer 1: Layout
            "Container", "Section", "Stack", "Grid", "Split", "Cluster", "Sidebar",
            "Center", "Cover", "Reel", "Masonry",
            // Layer 2: Primitives
            "Button", "Card", "FeatureCard", "PricingCard", "FormField",
            "Modal", "Alert", "Badge", "Kpi", "Stat", "Icon", "Accordion",
            "EmptyState", "Tabs", "Skeleton", "LoadingState", "ErrorState", "OfflineState",
            "Toast", "Banner",
            // Layer 3: Application
            "Navbar", "Hero", "Footer", "Timeline", "TimelineItem",
            "LogoCloud", "TestimonialCard", "Slides",
            // Layer 4: Patterns (Core Shells & Sections)
            "DashboardShell", "AuthPage", "PricingSection",
            // Phase 2B/3A: Navigation & Data
            "Breadcrumb", "Dropdown", "CommandPalette", "SearchBar", "FilterBar", "Paginator", "DataTable",
            // Phase 3B/3C: Advanced Interaction & Pages
            "FileUpload", "RichEditor", "ColorPicker", "HeroSection", "SettingsPage", "StatsSection",
            "FAQSection", "BlogSection", "TestimonialsSection", "ContactSection"
        ];
        if !standard_components.contains(&var.target.as_str()) && !self.components.contains_key(&var.target) {
            return Err(format!(
                "Variant target component '{}' is neither a standard component nor a declared custom component.",
                var.target
            ));
        }
        
        for rule in &var.base_rules {
            self.validate_style_rule(rule)?;
        }
        for rule in &var.hover_rules {
            self.validate_style_rule(rule)?;
        }
        for (_, rules) in &var.slot_rules {
            for rule in rules {
                self.validate_style_rule(rule)?;
            }
        }
        for resp in &var.responsive_rules {
            if resp.breakpoint != "desktop" && resp.breakpoint != "tablet" && resp.breakpoint != "mobile" {
                return Err(format!("Invalid breakpoint '{}' in responsive rules.", resp.breakpoint));
            }
            for rule in &resp.rules {
                self.validate_style_rule(rule)?;
            }
        }
        Ok(())
    }

    /// Validates custom CSS style rules according to Amana's 4-layer sanitizer rules.
    pub fn validate_style_rule(&self, rule: &StyleRule) -> Result<(), String> {
        let trimmed_selector = rule.selector.trim();
        let blocked_selectors = ["body", "html", "*", "script", "iframe", "object", "embed", "link", "meta", "base"];
        for blocked in blocked_selectors {
            if trimmed_selector == blocked || trimmed_selector.contains(&format!(" {}", blocked)) || trimmed_selector.contains(&format!(",{}", blocked)) {
                return Err(format!(
                    "Security: Selector '{}' attempts to modify globally blocked tag '{}'. Style block must remain scoped.",
                    rule.selector, blocked
                ));
            }
        }
        if trimmed_selector.contains("[onclick]") || trimmed_selector.contains("[on") {
            return Err(format!("Security: Attribute selector manipulation is not allowed in: '{}'", rule.selector));
        }

        const ALLOWED_PROPERTIES: &[&str] = &[
            "display", "position", "top", "right", "bottom", "left", "inset", "inset-inline", "inset-inline-start", "inset-inline-end",
            "width", "height", "min-width", "min-height", "max-width", "max-height",
            "padding", "padding-top", "padding-right", "padding-bottom", "padding-left", "padding-inline", "padding-block",
            "margin", "margin-top", "margin-right", "margin-bottom", "margin-left", "margin-inline", "margin-block",
            "gap", "row-gap", "column-gap",
            "grid-template-columns", "grid-template-rows", "grid-column", "grid-row", "grid-auto-flow",
            "flex-direction", "flex-wrap", "justify-content", "align-items", "flex-grow", "flex-shrink", "flex-basis",
            "background", "background-color", "background-image", "background-size", "background-position", "background-repeat",
            "color", "font-family", "font-size", "font-weight", "line-height", "letter-spacing", "text-align", "text-transform",
            "border", "border-color", "border-width", "border-style", "border-radius", "border-top-left-radius", "border-top-right-radius",
            "box-shadow", "opacity", "transform", "filter", "backdrop-filter", "z-index", "overflow", "overflow-x", "overflow-y",
            "transition", "transition-property", "transition-duration", "transition-timing-function", "transition-delay",
            "animation", "animation-name", "animation-duration", "animation-timing-function", "animation-delay", "animation-fill-mode",
            "will-change", "pointer-events", "user-select", "clip-path", "align-self", "justify-self", "justify-items",
            "layout", "columns", "radius", "shadow", "size"
        ];

        for decl in &rule.declarations {
            let clean_prop = decl.property.trim().to_lowercase().replace('_', "-");
            if !ALLOWED_PROPERTIES.contains(&clean_prop.as_str()) {
                return Err(format!(
                    "Security: CSS property '{}' is not supported or is restricted in Amana scopes.",
                    decl.property
                ));
            }

            let lower_val = decl.value.to_lowercase();
            let blocked_patterns = [
                "javascript:",
                "expression(",
                "behavior:",
                "url(data:",
                "url(http:",
                "url(https:",
                "<script",
                "</style",
                "-moz-binding",
                "binding:"
            ];
            for pattern in blocked_patterns {
                if lower_val.contains(pattern) {
                    return Err(format!(
                        "Security violation: CSS value '{}' contains dangerous or unapproved pattern '{}'",
                        decl.value, pattern
                    ));
                }
            }
        }
        Ok(())
    }

    /// Evaluates if a given DataType is numeric (Int, Float, or Money).
    fn is_numeric(&self, dt: &DataType) -> bool {
        matches!(dt, DataType::Int | DataType::Float | DataType::Money)
    }

    fn types_compatible(&self, expected: &DataType, actual: &DataType) -> bool {
        expected == actual
            || (self.is_numeric(expected) && self.is_numeric(actual))
            || matches!(
                (expected, actual),
                (
                    DataType::Int
                        | DataType::Float
                        | DataType::Bool
                        | DataType::Money
                        | DataType::Email
                        | DataType::Password
                        | DataType::DateTime,
                    DataType::Str
                )
            )
            || *expected == DataType::Str
    }

    fn field_type_for(model: &ModelDecl, field_name: &str) -> Option<DataType> {
        if field_name.eq_ignore_ascii_case("id") {
            return Some(DataType::Int);
        }
        model
            .fields
            .iter()
            .find(|field| field.name.eq_ignore_ascii_case(field_name))
            .map(|field| field.data_type.clone())
    }

    fn expression_uses_current_user(&self, expr: &Expression) -> bool {
        match expr {
            Expression::MemberAccess { object, property } if property == "current" => {
                matches!(&**object, Expression::Identifier(name) if name == &self.auth_model)
            }
            Expression::MemberAccess { object, .. } => self.expression_uses_current_user(object),
            Expression::Binary { left, right, .. } => {
                self.expression_uses_current_user(left) || self.expression_uses_current_user(right)
            }
            Expression::Unary { expr, .. } => self.expression_uses_current_user(expr),
            Expression::Ternary {
                cond,
                then_branch,
                else_branch,
            } => {
                self.expression_uses_current_user(cond)
                    || self.expression_uses_current_user(then_branch)
                    || self.expression_uses_current_user(else_branch)
            }
            Expression::Call { callee, args } => {
                self.expression_uses_current_user(callee)
                    || args
                        .iter()
                        .any(|arg| self.expression_uses_current_user(arg))
            }
            _ => false,
        }
    }
}

fn suggest_from_list(input: &str, options: &[&str]) -> Option<String> {
    let mut best: Option<(&str, usize)> = None;
    for &opt in options {
        let d = levenshtein_distance(input, opt);
        if d <= 2 {
            match best {
                None => best = Some((opt, d)),
                Some((_, prev_d)) if d < prev_d => best = Some((opt, d)),
                _ => {}
            }
        }
    }
    best.map(|(s, _)| s.to_string())
}

const LAYOUT_VALUES: &[&str] = &[
    "row", "column", "stack", "grid", "center", "inline", "cluster", "split", "bento",
    "split-diagonal", "asymmetric", "editorial", "dashboard-shell", "magazine",
    "command-center", "showcase-rail", "masonry", "sidebar",
];
const SURFACE_VALUES: &[&str] = &[
    "base", "muted", "elevated", "glass", "custom", "outline", "flat", "layered",
    "glass-layered",
];
const HOVER_VALUES: &[&str] = &["lift", "glow", "scale", "lift-glow", "none"];
const ENTRANCE_VALUES: &[&str] = &["fade", "slide-up", "slide-down", "zoom", "blur", "clip", "stagger-up", "none"];
const GRADIENT_VALUES: &[&str] = &[
    "primary", "accent", "hero", "mesh", "aurora", "spotlight", "custom", "brand",
    "sunset", "ocean", "mesh-cyan-indigo", "mesh-aurora",
];
const DENSITY_VALUES: &[&str] = &["compact", "comfortable", "spacious"];
const SHADOW_VALUES: &[&str] = &[
    "sm", "md", "lg", "xl", "soft", "floating", "strong", "smooth", "none",
];

fn normalize_design_value(val: &str) -> String {
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
        ViewElement::IfBlock { then_branch, else_branch, .. } => {
            for child in then_branch {
                collect_required_slots(child, required_slots);
            }
            if let Some(nodes) = else_branch {
                for child in nodes {
                    collect_required_slots(child, required_slots);
                }
            }
        }
        ViewElement::ResourceGrid { empty_element, loading_element, error_element, .. }
        | ViewElement::ResourceTable { empty_element, loading_element, error_element, .. } => {
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
