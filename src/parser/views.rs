// src/parser/views.rs
use super::css::*;
use super::{LocatedDefinitionKind, Parser};
use crate::ast::*;
use crate::lexer::TokenKind;

impl Parser {
    /// Constructs a new Parser instance utilizing the provided token stream.
    pub(crate) fn parse_view(&mut self) -> Result<ViewDecl, String> {
        self.expect(TokenKind::View)?;
        let line = self.peek_line();
        let column = self.peek_column();
        let name = self.expect_identifier()?;
        self.capture_definition(LocatedDefinitionKind::View, name.clone(), line, column);
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

    pub(crate) fn parse_component(&mut self) -> Result<ComponentDecl, String> {
        self.expect(TokenKind::Component)?;
        let line = self.peek_line();
        let column = self.peek_column();
        let name = self.expect_identifier()?;
        self.capture_definition(LocatedDefinitionKind::Component, name.clone(), line, column);

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
                                _ => {
                                    return Err(format!(
                                        "Unknown variant section '{}' inside component variant at line {}",
                                        sec_name,
                                        self.peek_line()
                                    ));
                                }
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

    pub(crate) fn parse_protected_block(&mut self) -> Result<ProtectedBlock, String> {
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

    pub(crate) fn parse_fetch_stmt(&mut self) -> Result<FetchStmt, String> {
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

    pub(crate) fn parse_state_decl(&mut self) -> Result<StateDecl, String> {
        self.expect(TokenKind::State)?;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::Assign)?;
        let initial_value = self.parse_expression(1)?;

        let mut persist = PersistMode::Memory;
        if self.check(TokenKind::LBracket) {
            self.advance();
            while !self.check(TokenKind::RBracket) {
                let key = self.expect_identifier()?;
                if key == "persist" {
                    self.expect(TokenKind::Colon)?;
                    let persist_value = self.expect_identifier()?;
                    persist = match persist_value.as_str() {
                        "memory" => PersistMode::Memory,
                        "local" => PersistMode::Local,
                        "session" => PersistMode::Session,
                        "cookie" => PersistMode::Cookie,
                        _ => {
                            return Err(format!(
                                "Unknown persist mode '{}' at line {}. Valid values: memory, local, session, cookie.",
                                persist_value,
                                self.peek_line()
                            ));
                        }
                    };
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

    pub(crate) fn parse_view_element(&mut self) -> Result<ViewElement, String> {
        self.consume_newlines();
        let token = self
            .peek_kind()
            .ok_or_else(|| format!("Expected view element at line {}", self.peek_line()))?;
        match token {
            TokenKind::Identifier(ref name) if name == "Accordion" => {
                self.advance();
                self.expect(TokenKind::Colon)?;
                self.consume_newlines();
                self.expect(TokenKind::Indent)?;

                let mut panels = Vec::new();
                while !self.check(TokenKind::Dedent) {
                    self.consume_newlines();
                    if self.check(TokenKind::Dedent) {
                        break;
                    }
                    let peek_token = self.peek_kind().ok_or_else(|| {
                        format!("Expected panel in Accordion at line {}", self.peek_line())
                    })?;
                    match peek_token {
                        TokenKind::Identifier(ref p_name) if p_name == "panel" => {
                            self.advance();
                            let title = match self.peek_kind() {
                                Some(TokenKind::StringLiteral(s)) => {
                                    self.advance();
                                    s
                                }
                                _ => {
                                    return Err(format!(
                                        "Expected string literal panel title at line {}",
                                        self.peek_line()
                                    ));
                                }
                            };
                            self.expect(TokenKind::Colon)?;
                            self.consume_newlines();
                            self.expect(TokenKind::Indent)?;

                            let mut panel_body = Vec::new();
                            while !self.check(TokenKind::Dedent) {
                                panel_body.push(self.parse_view_element()?);
                                self.consume_newlines();
                            }
                            self.expect(TokenKind::Dedent)?;
                            panels.push((title, panel_body));
                        }
                        _ => {
                            return Err(format!(
                                "Expected 'panel' in Accordion at line {}, found {:?}",
                                self.peek_line(),
                                peek_token
                            ));
                        }
                    }
                    self.consume_newlines();
                }
                self.expect(TokenKind::Dedent)?;
                Ok(ViewElement::Accordion { panels })
            }
            TokenKind::Identifier(ref name) if name == "Tabs" => {
                self.advance();
                self.expect(TokenKind::Colon)?;
                self.consume_newlines();
                self.expect(TokenKind::Indent)?;

                let mut tabs = Vec::new();
                while !self.check(TokenKind::Dedent) {
                    self.consume_newlines();
                    if self.check(TokenKind::Dedent) {
                        break;
                    }
                    let peek_token = self.peek_kind().ok_or_else(|| {
                        format!("Expected tab in Tabs at line {}", self.peek_line())
                    })?;
                    match peek_token {
                        TokenKind::Identifier(ref t_name) if t_name == "tab" => {
                            self.advance();
                            let title = match self.peek_kind() {
                                Some(TokenKind::StringLiteral(s)) => {
                                    self.advance();
                                    s
                                }
                                _ => {
                                    return Err(format!(
                                        "Expected string literal tab title at line {}",
                                        self.peek_line()
                                    ));
                                }
                            };
                            self.expect(TokenKind::Colon)?;
                            self.consume_newlines();
                            self.expect(TokenKind::Indent)?;

                            let mut tab_body = Vec::new();
                            while !self.check(TokenKind::Dedent) {
                                tab_body.push(self.parse_view_element()?);
                                self.consume_newlines();
                            }
                            self.expect(TokenKind::Dedent)?;
                            tabs.push((title, tab_body));
                        }
                        _ => {
                            return Err(format!(
                                "Expected 'tab' in Tabs at line {}, found {:?}",
                                self.peek_line(),
                                peek_token
                            ));
                        }
                    }
                    self.consume_newlines();
                }
                self.expect(TokenKind::Dedent)?;
                Ok(ViewElement::Tabs { tabs })
            }
            TokenKind::Identifier(ref name) if name == "Chart" => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let data_expr = self.expect_identifier()?;
                self.expect(TokenKind::Comma)?;
                let chart_type = self.expect_identifier()?;
                self.expect(TokenKind::Comma)?;
                let x_field = self.expect_identifier()?;
                self.expect(TokenKind::Comma)?;
                let y_field = self.expect_identifier()?;
                self.expect(TokenKind::RParen)?;
                if self.check(TokenKind::Colon) {
                    self.advance();
                    self.consume_newlines();
                    if self.check(TokenKind::Indent) {
                        self.advance();
                        self.consume_newlines();
                        if !self.check(TokenKind::Dedent) {
                            return Err("Chart does not accept a nested body.".to_string());
                        }
                        self.expect(TokenKind::Dedent)?;
                    }
                }
                Ok(ViewElement::Chart {
                    data_expr,
                    chart_type,
                    x_field,
                    y_field,
                })
            }
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
                    Ok(ViewElement::SlotDecl {
                        name: "default".to_string(),
                        optional: false,
                    })
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
                            _ => {
                                return Err(format!(
                                    "Unknown resource element option '{}' at line {}",
                                    key,
                                    self.peek_line()
                                ));
                            }
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
                let end_token = if self.check(TokenKind::LParen) {
                    has_call_parens = true;
                    self.advance();
                    Some(TokenKind::RParen)
                } else if self.check(TokenKind::LBracket) {
                    has_call_parens = true;
                    self.advance();
                    Some(TokenKind::RBracket)
                } else {
                    None
                };

                if let Some(end) = end_token {
                    while !self.check(end.clone()) {
                        let attr_name = self.expect_identifier()?;
                        self.expect(TokenKind::Colon)?;
                        let attr_val = self.parse_expression(1)?;
                        attributes.push((attr_name, attr_val));
                        if self.check(TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(end)?;
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
                    } else if self.check(TokenKind::NewLine)
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
                        // الابن المُضمّن في نفس السطر: يمكن أن يكون نصاً أو تعبيراً أو عنصراً كاملاً
                        let next_token = self
                            .peek_kind()
                            .ok_or_else(|| format!("Expected child element at line {}", self.peek_line()))?;
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
                            // عنصر HTML عادي كـ p: أو span: أو مكون PascalCase كـ Card:
                            TokenKind::Identifier(_) | TokenKind::If | TokenKind::For => {
                                let child = self.parse_view_element()?;
                                Ok(ViewElement::Element {
                                    tag,
                                    classes,
                                    attributes,
                                    children: vec![child],
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
}
