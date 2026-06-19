// src/parser/design.rs
use super::Parser;
use crate::ast::*;
use crate::lexer::TokenKind;

impl Parser {
    pub(crate) fn is_design_block_name(name: &str) -> bool {
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

    pub(crate) fn parse_design_block_body(&mut self, kind: String) -> Result<DesignBlock, String> {
        let settings = self.parse_design_settings(None)?;
        Ok(DesignBlock { kind, settings })
    }

    pub(crate) fn parse_design_settings(
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

    pub(crate) fn parse_design_setting_value(&mut self) -> Result<String, String> {
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

    pub(crate) fn normalize_design_setting_value(value: &str) -> String {
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
}
