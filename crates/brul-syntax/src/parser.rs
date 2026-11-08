use std::fmt;

use crate::ast::Expr;
use crate::lexer::{self, Span, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "строка {}, колонка {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for Error {}

impl From<lexer::Error> for Error {
    fn from(error: lexer::Error) -> Self {
        Self {
            message: error.message,
            line: error.line,
            column: error.column,
        }
    }
}

pub struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    at: usize,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Result<Self, Error> {
        let tokens = lexer::tokenize(source)?;
        Ok(Self {
            source,
            tokens,
            at: 0,
        })
    }

    pub fn parse_expression(&mut self) -> Result<Expr, Error> {
        self.primary()
    }

    pub fn expect_end(&mut self) -> Result<(), Error> {
        let token = &self.tokens[self.at];
        if token.kind == TokenKind::Eof {
            Ok(())
        } else {
            Err(self.error(token.span, "ожидался конец выражения"))
        }
    }

    fn primary(&mut self) -> Result<Expr, Error> {
        let token = &self.tokens[self.at];
        let kind = token.kind.clone();
        let span = token.span;
        match kind {
            TokenKind::Number(value) => {
                self.at += 1;
                Ok(Expr::Number(value))
            }
            TokenKind::String(value) => {
                self.at += 1;
                Ok(Expr::String(value))
            }
            TokenKind::Ident(name) => {
                self.at += 1;
                Ok(match name.as_str() {
                    "true" => Expr::Bool(true),
                    "false" => Expr::Bool(false),
                    "null" => Expr::Null,
                    _ => Expr::Ident(name),
                })
            }
            _ => Err(self.error(span, "ожидалось выражение")),
        }
    }

    fn error(&self, span: Span, message: &str) -> Error {
        let (line, column) = span.line_col(self.source);
        Error {
            message: message.to_owned(),
            line,
            column,
        }
    }
}

pub fn parse_expression(source: &str) -> Result<Expr, Error> {
    let mut parser = Parser::new(source)?;
    let expr = parser.parse_expression()?;
    parser.expect_end()?;
    Ok(expr)
}
