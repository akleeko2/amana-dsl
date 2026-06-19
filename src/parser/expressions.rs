// src/parser/expressions.rs
use super::Parser;
use crate::ast::*;
use crate::lexer::TokenKind;

impl Parser {
    pub fn parse_expression(&mut self, precedence: u8) -> Result<Expression, String> {
        let mut left = self.parse_primary()?;

        while let Some(op) = self.peek_kind() {
            let op_precedence = self.get_precedence(&op);
            if op_precedence < precedence {
                break;
            }

            self.advance();
            left = match op {
                TokenKind::Dot => {
                    let property = self.expect_identifier()?;
                    Expression::MemberAccess {
                        object: Box::new(left),
                        property,
                    }
                }
                TokenKind::LParen => {
                    let args = self.parse_call_args()?;
                    Expression::Call {
                        callee: Box::new(left),
                        args,
                    }
                }
                TokenKind::Question => {
                    let then_branch = self.parse_expression(1)?;
                    self.expect(TokenKind::Colon)?;
                    let else_branch = self.parse_expression(op_precedence)?;
                    Expression::Ternary {
                        cond: Box::new(left),
                        then_branch: Box::new(then_branch),
                        else_branch: Box::new(else_branch),
                    }
                }
                TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::EqEq
                | TokenKind::Neq
                | TokenKind::Gt
                | TokenKind::Lt
                | TokenKind::Gte
                | TokenKind::Lte
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::Assign => {
                    let right = if op == TokenKind::Assign {
                        self.parse_expression(op_precedence)?
                    } else {
                        self.parse_expression(op_precedence + 1)?
                    };
                    Expression::Binary {
                        left: Box::new(left),
                        op: self.op_to_str(&op),
                        right: Box::new(right),
                    }
                }
                _ => return Err("Unexpected operator in expression".to_string()),
            };
        }
        Ok(left)
    }

    pub(crate) fn peek_kind(&self) -> Option<TokenKind> {
        self.tokens.get(self.position).map(|t| t.kind.clone())
    }

    pub(crate) fn peek_line(&self) -> usize {
        self.tokens.get(self.position).map(|t| t.line).unwrap_or(0)
    }

    pub(crate) fn peek_column(&self) -> usize {
        self.tokens
            .get(self.position)
            .map(|t| t.column)
            .unwrap_or(1)
    }

    pub(crate) fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    pub(crate) fn expect(&mut self, kind: TokenKind) -> Result<(), String> {
        if self.peek_kind() == Some(kind.clone()) {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "Expected token {:?} at line {}:{}, found {:?}",
                kind,
                self.peek_line(),
                self.peek_column(),
                self.peek_kind()
            ))
        }
    }

    pub(crate) fn expect_identifier(&mut self) -> Result<String, String> {
        if let Some(tk) = self.peek_kind() {
            match tk {
                TokenKind::Identifier(id) => {
                    self.advance();
                    Ok(id)
                }
                TokenKind::Email => {
                    self.advance();
                    Ok("email".to_string())
                }
                TokenKind::Password => {
                    self.advance();
                    Ok("password".to_string())
                }
                TokenKind::Str => {
                    self.advance();
                    Ok("str".to_string())
                }
                TokenKind::Int => {
                    self.advance();
                    Ok("int".to_string())
                }
                TokenKind::Float => {
                    self.advance();
                    Ok("float".to_string())
                }
                TokenKind::Bool => {
                    self.advance();
                    Ok("bool".to_string())
                }
                TokenKind::DateTime => {
                    self.advance();
                    Ok("datetime".to_string())
                }
                TokenKind::Money => {
                    self.advance();
                    Ok("money".to_string())
                }
                TokenKind::State => {
                    self.advance();
                    Ok("state".to_string())
                }
                TokenKind::App => {
                    self.advance();
                    Ok("app".to_string())
                }
                TokenKind::Model => {
                    self.advance();
                    Ok("model".to_string())
                }
                TokenKind::Route => {
                    self.advance();
                    Ok("route".to_string())
                }
                TokenKind::View => {
                    self.advance();
                    Ok("view".to_string())
                }
                TokenKind::Component => {
                    self.advance();
                    Ok("component".to_string())
                }
                TokenKind::Protected => {
                    self.advance();
                    Ok("protected".to_string())
                }
                TokenKind::Server => {
                    self.advance();
                    Ok("server".to_string())
                }
                TokenKind::Client => {
                    self.advance();
                    Ok("client".to_string())
                }
                TokenKind::Render => {
                    self.advance();
                    Ok("render".to_string())
                }
                TokenKind::Form => {
                    self.advance();
                    Ok("form".to_string())
                }
                TokenKind::If => {
                    self.advance();
                    Ok("if".to_string())
                }
                TokenKind::Else => {
                    self.advance();
                    Ok("else".to_string())
                }
                TokenKind::For => {
                    self.advance();
                    Ok("for".to_string())
                }
                TokenKind::In => {
                    self.advance();
                    Ok("in".to_string())
                }
                TokenKind::Permit => {
                    self.advance();
                    Ok("permit".to_string())
                }
                TokenKind::Fetch => {
                    self.advance();
                    Ok("fetch".to_string())
                }
                TokenKind::Style => {
                    self.advance();
                    Ok("style".to_string())
                }
                TokenKind::And => {
                    self.advance();
                    Ok("and".to_string())
                }
                TokenKind::Or => {
                    self.advance();
                    Ok("or".to_string())
                }
                TokenKind::Not => {
                    self.advance();
                    Ok("not".to_string())
                }
                TokenKind::Variant => {
                    self.advance();
                    Ok("variant".to_string())
                }
                TokenKind::Slot => {
                    self.advance();
                    Ok("slot".to_string())
                }
                TokenKind::Optional => {
                    self.advance();
                    Ok("optional".to_string())
                }
                TokenKind::Tokens => {
                    self.advance();
                    Ok("tokens".to_string())
                }
                _ => Err(format!("Expected identifier at line {}", self.peek_line())),
            }
        } else {
            Err(format!("Expected identifier at line {}", self.peek_line()))
        }
    }

    pub(crate) fn check(&self, kind: TokenKind) -> bool {
        self.peek_kind() == Some(kind)
    }

    pub(crate) fn check_has_block_children(&self) -> bool {
        let mut pos = self.position;
        while pos < self.tokens.len() && self.tokens[pos].kind == TokenKind::NewLine {
            pos += 1;
        }
        if pos < self.tokens.len() {
            self.tokens[pos].kind == TokenKind::Indent
        } else {
            false
        }
    }

    pub(crate) fn consume_newlines(&mut self) {
        while self.check(TokenKind::NewLine) {
            self.advance();
        }
    }

    pub(crate) fn parse_indented_block<F, T>(&mut self, parse_fn: F) -> Result<Vec<T>, String>
    where
        F: Fn(&mut Self) -> Result<T, String>,
    {
        self.expect(TokenKind::Indent)?;
        let mut items = Vec::new();
        while !self.check(TokenKind::Dedent) {
            items.push(parse_fn(self)?);
            self.consume_newlines();
        }
        self.expect(TokenKind::Dedent)?;
        Ok(items)
    }

    pub(crate) fn parse_primary(&mut self) -> Result<Expression, String> {
        match self.peek_kind() {
            Some(TokenKind::Not) => {
                self.advance();
                let expr = self.parse_expression(9)?;
                Ok(Expression::Unary {
                    op: "not".to_string(),
                    expr: Box::new(expr),
                })
            }
            Some(TokenKind::Minus) => {
                self.advance();
                let expr = self.parse_expression(9)?;
                Ok(Expression::Unary {
                    op: "-".to_string(),
                    expr: Box::new(expr),
                })
            }
            Some(TokenKind::Identifier(id)) => {
                self.advance();
                Ok(Expression::Identifier(id))
            }
            Some(TokenKind::Number(n)) => {
                self.advance();
                Ok(Expression::Number(n))
            }
            Some(TokenKind::StringLiteral(s)) => {
                self.advance();
                Ok(Expression::StringLiteral(s))
            }
            Some(TokenKind::Boolean(b)) => {
                self.advance();
                Ok(Expression::Boolean(b))
            }
            Some(TokenKind::Null) => {
                self.advance();
                Ok(Expression::Null)
            }
            Some(TokenKind::LParen) => {
                self.advance();
                let expr = self.parse_expression(1)?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            _ => Err(format!(
                "Expected primary expression at line {}",
                self.peek_line()
            )),
        }
    }

    pub(crate) fn get_precedence(&self, op: &TokenKind) -> u8 {
        match op {
            TokenKind::Dot | TokenKind::LParen => 10,
            TokenKind::Star | TokenKind::Slash => 8,
            TokenKind::Plus | TokenKind::Minus => 7,
            TokenKind::EqEq
            | TokenKind::Neq
            | TokenKind::Gt
            | TokenKind::Lt
            | TokenKind::Gte
            | TokenKind::Lte => 6,
            TokenKind::And | TokenKind::Or => 5,
            TokenKind::Question => 3,
            TokenKind::Assign => 2,
            _ => 0,
        }
    }

    pub(crate) fn op_to_str(&self, op: &TokenKind) -> String {
        match op {
            TokenKind::Plus => "+".to_string(),
            TokenKind::Minus => "-".to_string(),
            TokenKind::Star => "*".to_string(),
            TokenKind::Slash => "/".to_string(),
            TokenKind::EqEq => "==".to_string(),
            TokenKind::Neq => "!=".to_string(),
            TokenKind::Gt => ">".to_string(),
            TokenKind::Lt => "<".to_string(),
            TokenKind::Gte => ">=".to_string(),
            TokenKind::Lte => "<=".to_string(),
            TokenKind::And => "and".to_string(),
            TokenKind::Or => "or".to_string(),
            TokenKind::Assign => "=".to_string(),
            _ => "".to_string(),
        }
    }

    pub(crate) fn parse_call_args(&mut self) -> Result<Vec<Expression>, String> {
        let mut args = Vec::new();
        if !self.check(TokenKind::RParen) {
            args.push(self.parse_expression(1)?);
            while self.check(TokenKind::Comma) {
                self.advance();
                args.push(self.parse_expression(1)?);
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(args)
    }

    pub(crate) fn check_identifier(&self) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Identifier(_)))
    }

    pub(crate) fn peek_token_kind(&self) -> Option<TokenKind> {
        self.tokens.get(self.position).map(|t| t.kind.clone())
    }
}
