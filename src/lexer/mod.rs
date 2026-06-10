// src/lexer/mod.rs

/// Represents the various categories of lexical tokens supported by the Amana language.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    // Keywords
    App,
    Model,
    Route,
    View,
    Component,
    Protected,
    Server,
    Client,
    Render,
    State,
    Form,
    If,
    Else,
    For,
    In,
    Permit,
    Fetch,
    Style,
    Variant,
    Slot,
    Optional,
    Tokens,
    // Types
    Str,
    Int,
    Float,
    Bool,
    Email,
    Password,
    DateTime,
    Money,
    // Literals
    Identifier(String),
    StringLiteral(String),
    Number(f64),
    Boolean(bool),
    Null,
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    EqEq,
    Neq,
    Gt,
    Lt,
    Gte,
    Lte,
    Assign,
    And,
    Or,
    Not,
    Question,
    Colon,
    Dot,
    Comma,
    Arrow,
    Percent,
    // Brackets
    LParen,
    RParen,
    LBracket,
    RBracket,
    // Indentation
    Indent,
    Dedent,
    NewLine,
}

/// A token structure containing the kind and its exact source position (line and column).
#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

/// Lexical analyzer (Lexer) responsible for translating raw source code strings into a vector of Tokens.
pub struct Lexer {
    source: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    indent_stack: Vec<usize>,
}

impl Lexer {
    /// Constructs a new Lexer instance for the given source code text.
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            indent_stack: vec![0],
        }
    }

    /// Tokenizes the source code string, tracking indent/dedent levels and returning a Vec of Tokens.
    /// Returns a String error message on failure (e.g. invalid characters or indentation mismatches).
    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        let mut is_at_line_start = true;

        while self.position < self.source.len() {
            // 1. معالجة الإزاحة عند بداية كل سطر
            if is_at_line_start {
                // تحقق مما إذا كان السطر فارغًا أو تعليقًا بالكامل لتخطيه
                let mut temp_pos = self.position;
                let mut is_empty_or_comment = false;
                while temp_pos < self.source.len() {
                    let tc = self.source[temp_pos];
                    if tc == ' ' {
                        temp_pos += 1;
                    } else if tc == '\n' || tc == '\r' || tc == '#' {
                        is_empty_or_comment = true;
                        break;
                    } else {
                        break;
                    }
                }

                if is_empty_or_comment {
                    // تخطي السطر بالكامل
                    while self.position < self.source.len() && self.source[self.position] != '\n' {
                        self.advance();
                    }
                    if self.position < self.source.len() {
                        self.advance(); // consume \n
                        self.line += 1;
                        self.column = 1;
                    }
                    is_at_line_start = true;
                    continue;
                }

                // حساب المسافات البادئة
                let mut spaces = 0;
                while self.position < self.source.len() {
                    let next_c = self.source[self.position];
                    if next_c == ' ' {
                        spaces += 1;
                        self.advance();
                    } else if next_c == '\t' {
                        return Err(format!(
                            "Tab character detected at line {}:{}. Only spaces are allowed.",
                            self.line, self.column
                        ));
                    } else {
                        break;
                    }
                }

                let last_indent = *self.indent_stack.last().unwrap();
                if spaces > last_indent {
                    self.indent_stack.push(spaces);
                    tokens.push(Token {
                        kind: TokenKind::Indent,
                        line: self.line,
                        column: spaces + 1,
                    });
                } else if spaces < last_indent {
                    while let Some(&top) = self.indent_stack.last() {
                        if top > spaces {
                            self.indent_stack.pop();
                            tokens.push(Token {
                                kind: TokenKind::Dedent,
                                line: self.line,
                                column: spaces + 1,
                            });
                        } else {
                            break;
                        }
                    }
                    if *self.indent_stack.last().unwrap_or(&0) != spaces {
                        return Err(format!(
                            "Indentation mismatch at line {}:{}. Expected {} spaces but got {}.",
                            self.line,
                            self.column,
                            self.indent_stack.last().unwrap_or(&0),
                            spaces
                        ));
                    }
                }
                is_at_line_start = false;
            }

            if self.position >= self.source.len() {
                break;
            }
            let curr = self.source[self.position];

            if curr == '\n' {
                tokens.push(Token {
                    kind: TokenKind::NewLine,
                    line: self.line,
                    column: self.column,
                });
                self.advance();
                self.line += 1;
                self.column = 1;
                is_at_line_start = true;
                continue;
            }

            if curr == '\r' {
                self.advance();
                continue;
            }

            if curr.is_whitespace() {
                self.advance();
                continue;
            }

            // التعليقات الملحقة في نهاية السطر
            if curr == '#' {
                while self.position < self.source.len() && self.source[self.position] != '\n' {
                    self.advance();
                }
                continue;
            }

            // التحقق من النص المنسق f"..."
            if curr == 'f'
                && self.position + 1 < self.source.len()
                && self.source[self.position + 1] == '"'
            {
                tokens.push(self.read_formatted_string()?);
                continue;
            }

            // قراءة النصوص متعددة الأسطر أو العادية
            if curr == '"' {
                if self.position + 2 < self.source.len()
                    && self.source[self.position + 1] == '"'
                    && self.source[self.position + 2] == '"'
                {
                    tokens.push(self.read_multiline_string()?);
                } else {
                    tokens.push(self.read_string()?);
                }
                continue;
            }

            // قراءة الأرقام
            if curr.is_ascii_digit() {
                tokens.push(self.read_number());
                continue;
            }

            // قراءة المعرفات والكلمات المفتاحية
            if curr.is_ascii_alphabetic() || curr == '_' {
                tokens.push(self.read_identifier_or_keyword());
                continue;
            }

            // قراءة العمليات والرموز
            if let Some(token) = self.read_operator_or_symbol() {
                tokens.push(token);
                continue;
            }

            return Err(format!(
                "Unexpected character '{}' at line {}:{}",
                curr, self.line, self.column
            ));
        }

        // تفريغ المكدس بالكامل عند نهاية الملف
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            tokens.push(Token {
                kind: TokenKind::Dedent,
                line: self.line,
                column: self.column,
            });
        }

        Ok(tokens)
    }

    fn advance(&mut self) {
        self.position += 1;
        self.column += 1;
    }

    fn read_string(&mut self) -> Result<Token, String> {
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // skip quote
        let mut s = String::new();
        while self.position < self.source.len() && self.source[self.position] != '"' {
            let c = self.source[self.position];
            if c == '\n' {
                return Err(format!(
                    "Unterminated string literal at line {}:{}",
                    start_line, start_col
                ));
            }
            s.push(c);
            self.advance();
        }
        if self.position >= self.source.len() {
            return Err(format!(
                "Unterminated string literal starting at line {}:{}",
                start_line, start_col
            ));
        }
        self.advance(); // skip closing quote
        Ok(Token {
            kind: TokenKind::StringLiteral(s),
            line: start_line,
            column: start_col,
        })
    }

    fn read_multiline_string(&mut self) -> Result<Token, String> {
        let start_line = self.line;
        let start_col = self.column;
        // skip """
        self.advance();
        self.advance();
        self.advance();
        let mut s = String::new();
        while self.position + 2 < self.source.len() {
            if self.source[self.position] == '"'
                && self.source[self.position + 1] == '"'
                && self.source[self.position + 2] == '"'
            {
                break;
            }
            let c = self.source[self.position];
            s.push(c);
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.position += 1;
        }
        if self.position + 2 >= self.source.len() {
            return Err(format!(
                "Unterminated multiline string starting at line {}:{}",
                start_line, start_col
            ));
        }
        // skip closing """
        self.advance();
        self.advance();
        self.advance();
        Ok(Token {
            kind: TokenKind::StringLiteral(s),
            line: start_line,
            column: start_col,
        })
    }

    fn read_formatted_string(&mut self) -> Result<Token, String> {
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // skip 'f'
        self.advance(); // skip '"'
        let mut s = String::new();
        while self.position < self.source.len() && self.source[self.position] != '"' {
            let c = self.source[self.position];
            if c == '\n' {
                return Err(format!(
                    "Unterminated formatted string literal at line {}:{}",
                    start_line, start_col
                ));
            }
            s.push(c);
            self.advance();
        }
        if self.position >= self.source.len() {
            return Err(format!(
                "Unterminated formatted string literal starting at line {}:{}",
                start_line, start_col
            ));
        }
        self.advance(); // skip closing quote
        Ok(Token {
            kind: TokenKind::StringLiteral(format!("f\"{}\"", s)),
            line: start_line,
            column: start_col,
        })
    }

    fn read_number(&mut self) -> Token {
        let start_line = self.line;
        let start_col = self.column;
        let mut s = String::new();
        while self.position < self.source.len()
            && (self.source[self.position].is_ascii_digit() || self.source[self.position] == '.')
        {
            s.push(self.source[self.position]);
            self.advance();
        }
        let val: f64 = s.parse().unwrap_or(0.0);
        Token {
            kind: TokenKind::Number(val),
            line: start_line,
            column: start_col,
        }
    }

    fn read_identifier_or_keyword(&mut self) -> Token {
        let start_line = self.line;
        let start_col = self.column;
        let mut s = String::new();
        while self.position < self.source.len()
            && (self.source[self.position].is_ascii_alphanumeric()
                || self.source[self.position] == '_')
        {
            s.push(self.source[self.position]);
            self.advance();
        }
        let kind = match s.as_str() {
            "app" => TokenKind::App,
            "model" => TokenKind::Model,
            "route" => TokenKind::Route,
            "view" => TokenKind::View,
            "component" => TokenKind::Component,
            "protected" => TokenKind::Protected,
            "server" => TokenKind::Server,
            "client" => TokenKind::Client,
            "render" => TokenKind::Render,
            "state" => TokenKind::State,
            "form" => TokenKind::Form,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "permit" => TokenKind::Permit,
            "fetch" => TokenKind::Fetch,
            "style" => TokenKind::Style,
            "variant" => TokenKind::Variant,
            "slot" => TokenKind::Slot,
            "optional" => TokenKind::Optional,
            "tokens" => TokenKind::Tokens,
            "str" => TokenKind::Str,
            "int" => TokenKind::Int,
            "float" => TokenKind::Float,
            "bool" => TokenKind::Bool,
            "email" => TokenKind::Email,
            "password" => TokenKind::Password,
            "datetime" => TokenKind::DateTime,
            "money" => TokenKind::Money,
            "true" => TokenKind::Boolean(true),
            "false" => TokenKind::Boolean(false),
            "null" => TokenKind::Null,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            _ => TokenKind::Identifier(s),
        };
        Token {
            kind,
            line: start_line,
            column: start_col,
        }
    }

    fn read_operator_or_symbol(&mut self) -> Option<Token> {
        let start_line = self.line;
        let start_col = self.column;
        let c = self.source[self.position];

        let kind = match c {
            '+' => Some(TokenKind::Plus),
            '-' => {
                if self.position + 1 < self.source.len() && self.source[self.position + 1] == '>' {
                    self.advance();
                    Some(TokenKind::Arrow)
                } else {
                    Some(TokenKind::Minus)
                }
            }
            '*' => Some(TokenKind::Star),
            '/' => Some(TokenKind::Slash),
            '?' => Some(TokenKind::Question),
            ':' => Some(TokenKind::Colon),
            '.' => Some(TokenKind::Dot),
            ',' => Some(TokenKind::Comma),
            '%' => Some(TokenKind::Percent),
            '(' => Some(TokenKind::LParen),
            ')' => Some(TokenKind::RParen),
            '[' => Some(TokenKind::LBracket),
            ']' => Some(TokenKind::RBracket),
            '=' => {
                if self.position + 1 < self.source.len() && self.source[self.position + 1] == '=' {
                    self.advance();
                    Some(TokenKind::EqEq)
                } else {
                    Some(TokenKind::Assign)
                }
            }
            '!' => {
                if self.position + 1 < self.source.len() && self.source[self.position + 1] == '=' {
                    self.advance();
                    Some(TokenKind::Neq)
                } else {
                    Some(TokenKind::Not)
                }
            }
            '>' => {
                if self.position + 1 < self.source.len() && self.source[self.position + 1] == '=' {
                    self.advance();
                    Some(TokenKind::Gte)
                } else {
                    Some(TokenKind::Gt)
                }
            }
            '<' => {
                if self.position + 1 < self.source.len() && self.source[self.position + 1] == '=' {
                    self.advance();
                    Some(TokenKind::Lte)
                } else {
                    Some(TokenKind::Lt)
                }
            }
            // دعم السهم البرمجي كرمز يونيكود أيضًا
            '→' => Some(TokenKind::Arrow),
            _ => None,
        };

        if kind.is_some() {
            self.advance();
        }
        kind.map(|k| Token {
            kind: k,
            line: start_line,
            column: start_col,
        })
    }
}
