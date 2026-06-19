// src/parser/top_level.rs
use super::{LocatedDefinition, LocatedDefinitionKind, Parser};
use crate::ast::*;
use crate::lexer::{Token, TokenKind};

impl Parser {
    pub fn new(mut tokens: Vec<Token>) -> Self {
        // Preprocess tokens to convert keywords to identifiers if adjacent to Minus
        let n = tokens.len();
        for i in 0..n {
            let adjacent_to_minus = (i > 0 && tokens[i - 1].kind == TokenKind::Minus)
                || (i + 1 < n && tokens[i + 1].kind == TokenKind::Minus);
            if adjacent_to_minus {
                let current_kind = &tokens[i].kind;
                let new_kind = match current_kind {
                    TokenKind::App => Some(TokenKind::Identifier("app".to_string())),
                    TokenKind::Model => Some(TokenKind::Identifier("model".to_string())),
                    TokenKind::Route => Some(TokenKind::Identifier("route".to_string())),
                    TokenKind::View => Some(TokenKind::Identifier("view".to_string())),
                    TokenKind::Component => Some(TokenKind::Identifier("component".to_string())),
                    TokenKind::Protected => Some(TokenKind::Identifier("protected".to_string())),
                    TokenKind::Server => Some(TokenKind::Identifier("server".to_string())),
                    TokenKind::Client => Some(TokenKind::Identifier("client".to_string())),
                    TokenKind::Render => Some(TokenKind::Identifier("render".to_string())),
                    TokenKind::State => Some(TokenKind::Identifier("state".to_string())),
                    TokenKind::Form => Some(TokenKind::Identifier("form".to_string())),
                    TokenKind::If => Some(TokenKind::Identifier("if".to_string())),
                    TokenKind::Else => Some(TokenKind::Identifier("else".to_string())),
                    TokenKind::For => Some(TokenKind::Identifier("for".to_string())),
                    TokenKind::In => Some(TokenKind::Identifier("in".to_string())),
                    TokenKind::Permit => Some(TokenKind::Identifier("permit".to_string())),
                    TokenKind::Fetch => Some(TokenKind::Identifier("fetch".to_string())),
                    TokenKind::Style => Some(TokenKind::Identifier("style".to_string())),
                    TokenKind::Variant => Some(TokenKind::Identifier("variant".to_string())),
                    TokenKind::Slot => Some(TokenKind::Identifier("slot".to_string())),
                    TokenKind::Optional => Some(TokenKind::Identifier("optional".to_string())),
                    TokenKind::Tokens => Some(TokenKind::Identifier("tokens".to_string())),
                    TokenKind::Str => Some(TokenKind::Identifier("str".to_string())),
                    TokenKind::Int => Some(TokenKind::Identifier("int".to_string())),
                    TokenKind::Float => Some(TokenKind::Identifier("float".to_string())),
                    TokenKind::Bool => Some(TokenKind::Identifier("bool".to_string())),
                    TokenKind::Email => Some(TokenKind::Identifier("email".to_string())),
                    TokenKind::Password => Some(TokenKind::Identifier("password".to_string())),
                    TokenKind::DateTime => Some(TokenKind::Identifier("datetime".to_string())),
                    TokenKind::Money => Some(TokenKind::Identifier("money".to_string())),
                    TokenKind::Boolean(true) => Some(TokenKind::Identifier("true".to_string())),
                    TokenKind::Boolean(false) => Some(TokenKind::Identifier("false".to_string())),
                    TokenKind::Null => Some(TokenKind::Identifier("null".to_string())),
                    TokenKind::And => Some(TokenKind::Identifier("and".to_string())),
                    TokenKind::Or => Some(TokenKind::Identifier("or".to_string())),
                    TokenKind::Not => Some(TokenKind::Identifier("not".to_string())),
                    _ => None,
                };
                if let Some(kind) = new_kind {
                    tokens[i].kind = kind;
                }
            }
        }
        Self {
            tokens,
            position: 0,
            captured_definitions: Vec::new(),
        }
    }

    pub(crate) fn capture_definition(
        &mut self,
        kind: LocatedDefinitionKind,
        name: String,
        line: usize,
        column: usize,
    ) {
        self.captured_definitions.push(LocatedDefinition {
            kind,
            name,
            line,
            column,
        });
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
                Some(TokenKind::Tokens) => nodes.push(AmanaNode::Tokens(self.parse_tokens_decl()?)),
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

    pub(crate) fn parse_simple_setting_value(&mut self) -> Result<String, String> {
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
        let line = self.peek_line();
        let column = self.peek_column();
        let name = self.expect_identifier()?;
        self.capture_definition(LocatedDefinitionKind::App, name.clone(), line, column);
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
        let line = self.peek_line();
        let column = self.peek_column();
        let name = self.expect_identifier()?;
        self.capture_definition(LocatedDefinitionKind::Model, name.clone(), line, column);
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
        let mut where_expr = None;
        let mut fields = Vec::new();

        while !self.check(TokenKind::NewLine)
            && !self.check(TokenKind::Dedent)
            && self.peek_kind().is_some()
        {
            let key = self.expect_identifier()?;
            match key.as_str() {
                "where" => {
                    where_expr = Some(self.parse_expression(1)?);
                }
                "fields" => {
                    if self.check(TokenKind::LBracket) {
                        self.advance();
                        if !self.check(TokenKind::RBracket) {
                            loop {
                                fields.push(self.expect_identifier()?);
                                if self.check(TokenKind::Comma) {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                        }
                        self.expect(TokenKind::RBracket)?;
                    } else {
                        loop {
                            if self.check(TokenKind::NewLine) || self.check(TokenKind::Dedent) {
                                break;
                            }
                            if let Some(TokenKind::Identifier(next)) = self.peek_kind()
                                && (next == "where" || next == "fields")
                            {
                                break;
                            }
                            fields.push(self.expect_identifier()?);
                            if self.check(TokenKind::Comma) {
                                self.advance();
                            } else if self.check(TokenKind::NewLine)
                                || self.check(TokenKind::Dedent)
                            {
                                break;
                            } else if let Some(TokenKind::Identifier(next)) = self.peek_kind()
                                && (next == "where" || next == "fields")
                            {
                                break;
                            }
                        }
                    }
                }
                _ => {
                    return Err(format!(
                        "Unexpected permission option '{}' at line {}",
                        key,
                        self.peek_line()
                    ));
                }
            }
        }
        Ok(PermissionRule {
            role,
            action,
            resource,
            where_expr,
            fields,
        })
    }

    fn parse_route(&mut self) -> Result<RouteDecl, String> {
        self.expect(TokenKind::Route)?;
        let line = self.peek_line();
        let column = self.peek_column();
        let path = self.parse_path()?;
        self.capture_definition(LocatedDefinitionKind::Route, path.clone(), line, column);

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

    pub(crate) fn parse_path(&mut self) -> Result<String, String> {
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
}
