// src/parser/styles.rs
use super::css::{compile_css_decl, compile_hover_rule};
use super::{LocatedDefinitionKind, Parser};
use crate::ast::*;
use crate::lexer::TokenKind;

impl Parser {
    pub(crate) fn parse_styles(&mut self) -> Result<String, String> {
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
                if self.position > 0 {
                    let prev_t = &self.tokens[self.position - 1];
                    let curr_t = &self.tokens[self.position];
                    if prev_t.line == curr_t.line {
                        let len = match &prev_t.kind {
                            TokenKind::Identifier(s) => s.len(),
                            TokenKind::HashColor(s) => s.len() + 1,
                            TokenKind::StringLiteral(s) => s.len(),
                            TokenKind::Number(n) => n.to_string().len(),
                            TokenKind::Boolean(b) => b.to_string().len(),
                            TokenKind::Null => 4,
                            TokenKind::LParen
                            | TokenKind::RParen
                            | TokenKind::LBracket
                            | TokenKind::RBracket
                            | TokenKind::Dot
                            | TokenKind::Colon
                            | TokenKind::Comma
                            | TokenKind::Percent
                            | TokenKind::Star
                            | TokenKind::Slash
                            | TokenKind::Plus
                            | TokenKind::Minus
                            | TokenKind::Assign
                            | TokenKind::Not
                            | TokenKind::Question
                            | TokenKind::EqEq
                            | TokenKind::Neq
                            | TokenKind::Gt
                            | TokenKind::Lt
                            | TokenKind::Gte
                            | TokenKind::Lte => 1,
                            TokenKind::Arrow => 2,
                            TokenKind::And => 3,
                            TokenKind::Or => 2,
                            _ => 0,
                        };
                        let len = if len == 0 {
                            match &prev_t.kind {
                                TokenKind::App => 3,
                                TokenKind::Model => 5,
                                TokenKind::Route => 5,
                                TokenKind::View => 4,
                                TokenKind::Component => 9,
                                TokenKind::Protected => 9,
                                TokenKind::Server => 6,
                                TokenKind::Client => 6,
                                TokenKind::Render => 6,
                                TokenKind::State => 5,
                                TokenKind::Form => 4,
                                TokenKind::If => 2,
                                TokenKind::Else => 4,
                                TokenKind::For => 3,
                                TokenKind::In => 2,
                                TokenKind::Permit => 6,
                                TokenKind::Fetch => 5,
                                TokenKind::Style => 5,
                                TokenKind::Variant => 7,
                                TokenKind::Slot => 4,
                                TokenKind::Optional => 8,
                                TokenKind::Tokens => 6,
                                TokenKind::Str => 3,
                                TokenKind::Int => 3,
                                TokenKind::Float => 5,
                                TokenKind::Bool => 4,
                                TokenKind::Email => 5,
                                TokenKind::Password => 8,
                                TokenKind::DateTime => 8,
                                TokenKind::Money => 5,
                                _ => 1,
                            }
                        } else {
                            len
                        };

                        if curr_t.column > prev_t.column + len {
                            if !selector.is_empty()
                                && !selector.ends_with(' ')
                                && !selector.ends_with(", ")
                            {
                                selector.push(' ');
                            }
                        }
                    }
                }
                if tk == TokenKind::Colon {
                    let is_terminal = match self.tokens.get(self.position + 1).map(|t| &t.kind) {
                        Some(TokenKind::NewLine)
                        | Some(TokenKind::Indent)
                        | Some(TokenKind::Dedent)
                        | None => true,
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
                    TokenKind::Gt => {
                        selector.push('>');
                        prev_was_ident = false;
                    }
                    TokenKind::Lt => {
                        selector.push('<');
                        prev_was_ident = false;
                    }
                    TokenKind::Plus => {
                        selector.push('+');
                        prev_was_ident = false;
                    }
                    TokenKind::LParen => {
                        selector.push('(');
                        prev_was_ident = false;
                    }
                    TokenKind::RParen => {
                        selector.push(')');
                        prev_was_ident = false;
                    }
                    TokenKind::LBracket => {
                        selector.push('[');
                        prev_was_ident = false;
                    }
                    TokenKind::RBracket => {
                        selector.push(']');
                        prev_was_ident = false;
                    }
                    TokenKind::Assign => {
                        selector.push('=');
                        prev_was_ident = false;
                    }
                    TokenKind::Not => {
                        selector.push('!');
                        prev_was_ident = false;
                    }
                    TokenKind::StringLiteral(ref s) => {
                        selector.push_str(&format!("\"{}\"", s));
                        prev_was_ident = false;
                    }
                    TokenKind::Number(n) => {
                        selector.push_str(&n.to_string());
                        prev_was_ident = false;
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
                        TokenKind::Not => {
                            self.advance();
                            ("!".to_string(), false)
                        }
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
                            let prop_lower = prop.to_lowercase();
                            if prop_lower == "content" || prop_lower == "font-family" || prop_lower == "font" {
                                (format!("\"{}\"", s), true)
                            } else {
                                (s.clone(), true)
                            }
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
                            format_operator(&val, "-")
                        }
                        TokenKind::Slash => {
                            self.advance();
                            format_operator(&val, "/")
                        }
                        TokenKind::Plus => {
                            self.advance();
                            format_operator(&val, "+")
                        }
                        TokenKind::Star => {
                            self.advance();
                            format_operator(&val, "*")
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

    pub(crate) fn is_identifier_like_token(tk: &TokenKind) -> bool {
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

    pub(crate) fn parse_component_params(&mut self) -> Result<Vec<ComponentParam>, String> {
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while !self.check(TokenKind::RParen) {
            let name = self.expect_identifier()?;
            let mut ty = None;
            if self.check(TokenKind::Colon) {
                self.advance();
                let next_kind = self
                    .peek_kind()
                    .ok_or_else(|| "Unexpected EOF".to_string())?;
                let type_str = match next_kind {
                    TokenKind::Identifier(t) => {
                        self.advance();
                        t
                    }
                    TokenKind::Str => {
                        self.advance();
                        "str".to_string()
                    }
                    TokenKind::Int => {
                        self.advance();
                        "int".to_string()
                    }
                    TokenKind::Float => {
                        self.advance();
                        "float".to_string()
                    }
                    TokenKind::Bool => {
                        self.advance();
                        "bool".to_string()
                    }
                    TokenKind::Email => {
                        self.advance();
                        "email".to_string()
                    }
                    TokenKind::Password => {
                        self.advance();
                        "password".to_string()
                    }
                    TokenKind::DateTime => {
                        self.advance();
                        "datetime".to_string()
                    }
                    TokenKind::Money => {
                        self.advance();
                        "money".to_string()
                    }
                    _ => {
                        return Err(format!(
                            "Expected type name in component parameters, found {:?}",
                            next_kind
                        ));
                    }
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

    pub(crate) fn parse_css_decls_block(&mut self) -> Result<Vec<CssDecl>, String> {
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
                return Err(format!(
                    "Expected ':' in CSS declaration, found {:?}",
                    self.peek_kind()
                ));
            }

            let mut val = String::new();
            let mut prev_was_word = false;
            while !self.check(TokenKind::NewLine)
                && !self.check(TokenKind::Dedent)
                && self.peek_kind().is_some()
            {
                let tk = self.peek_kind().unwrap();
                let (token_str, is_word) = match tk {
                    TokenKind::Not => {
                        self.advance();
                        ("!".to_string(), false)
                    }
                    _ if Self::is_identifier_like_token(&tk) => (self.expect_identifier()?, true),
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
                        format_operator(&val, "-")
                    }
                    TokenKind::Slash => {
                        self.advance();
                        format_operator(&val, "/")
                    }
                    TokenKind::Plus => {
                        self.advance();
                        format_operator(&val, "+")
                    }
                    TokenKind::Star => {
                        self.advance();
                        format_operator(&val, "*")
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
            decls.push(CssDecl {
                property: prop,
                value: val,
            });
            self.consume_newlines();
        }
        Ok(decls)
    }

    pub(crate) fn parse_style_rules_inline(&mut self) -> Result<Vec<StyleRule>, String> {
        let decls = self.parse_css_decls_block()?;
        Ok(vec![StyleRule {
            selector: "&".to_string(),
            declarations: decls,
        }])
    }

    pub(crate) fn parse_slots_rules(&mut self) -> Result<Vec<(String, Vec<StyleRule>)>, String> {
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

    pub(crate) fn parse_responsive_rules(&mut self) -> Result<Vec<ResponsiveRule>, String> {
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

    pub(crate) fn parse_variant_node(&mut self) -> Result<VariantDecl, String> {
        self.expect(TokenKind::Variant)?;
        let line = self.peek_line();
        let column = self.peek_column();
        let target = self.expect_identifier()?;
        self.expect(TokenKind::Dot)?;
        let name = self.expect_identifier()?;
        self.capture_definition(
            LocatedDefinitionKind::Variant,
            format!("{}.{}", target, name),
            line,
            column,
        );
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
                        "Unknown variant section '{}' in variant declaration at line {}",
                        sec_name,
                        self.peek_line()
                    ));
                }
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

    pub(crate) fn parse_tokens_decl(&mut self) -> Result<TokenConfigBlock, String> {
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

fn is_in_math_function(val: &str) -> bool {
    let mut paren_stack = Vec::new();
    let chars = val.chars().collect::<Vec<_>>();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '(' {
            let mut j = i;
            while j > 0 && chars[j - 1].is_alphabetic() {
                j -= 1;
            }
            let word: String = chars[j..i].iter().collect();
            let is_math = matches!(word.as_str(), "calc" | "min" | "max" | "clamp");
            paren_stack.push(is_math);
        } else if chars[i] == ')' {
            paren_stack.pop();
        }
        i += 1;
    }
    paren_stack.last().copied().unwrap_or(false)
}

fn format_operator(val: &str, op: &str) -> (String, bool) {
    let last_char = val.chars().last();
    let is_binary = if let Some(c) = last_char {
        c.is_alphanumeric() || c == ')' || c == '%' || c == '"' || c == '\''
    } else {
        false
    };
    if is_binary && is_in_math_function(val) {
        (format!(" {} ", op), false)
    } else {
        (op.to_string(), false)
    }
}
