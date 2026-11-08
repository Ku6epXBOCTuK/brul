use std::fmt;

use crate::ast::{AssignOp, BinaryOp, Expr, UnaryOp};
use crate::lexer::{self, Punct, Span, Token, TokenKind};

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
        self.assignment()
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

    fn assignment(&mut self) -> Result<Expr, Error> {
        let target = self.ternary()?;
        let op = match &self.tokens[self.at].kind {
            TokenKind::Punct(Punct::Assign) => AssignOp::Set,
            TokenKind::Punct(Punct::PlusAssign) => AssignOp::Add,
            TokenKind::Punct(Punct::MinusAssign) => AssignOp::Sub,
            _ => return Ok(target),
        };
        self.at += 1;
        let value = self.assignment()?;
        Ok(Expr::Assign {
            target: Box::new(target),
            op,
            value: Box::new(value),
        })
    }

    fn ternary(&mut self) -> Result<Expr, Error> {
        let condition = self.logic_or()?;
        if self.tokens[self.at].kind != TokenKind::Punct(Punct::Question) {
            return Ok(condition);
        }
        self.at += 1;
        let then_branch = self.parse_expression()?;
        self.expect_punct(Punct::Colon, "ожидался ':'")?;
        let else_branch = self.ternary()?;
        Ok(Expr::Ternary {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        })
    }

    fn logic_or(&mut self) -> Result<Expr, Error> {
        self.binary(Self::logic_and, &[(Punct::Or, BinaryOp::Or)])
    }

    fn logic_and(&mut self) -> Result<Expr, Error> {
        self.binary(Self::equality, &[(Punct::And, BinaryOp::And)])
    }

    fn equality(&mut self) -> Result<Expr, Error> {
        self.binary(
            Self::comparison,
            &[(Punct::Eq, BinaryOp::Eq), (Punct::NotEq, BinaryOp::NotEq)],
        )
    }

    fn comparison(&mut self) -> Result<Expr, Error> {
        self.binary(
            Self::term,
            &[
                (Punct::Lt, BinaryOp::Lt),
                (Punct::Gt, BinaryOp::Gt),
                (Punct::Le, BinaryOp::Le),
                (Punct::Ge, BinaryOp::Ge),
            ],
        )
    }

    fn term(&mut self) -> Result<Expr, Error> {
        self.binary(
            Self::factor,
            &[(Punct::Plus, BinaryOp::Add), (Punct::Minus, BinaryOp::Sub)],
        )
    }

    fn factor(&mut self) -> Result<Expr, Error> {
        self.binary(
            Self::unary,
            &[
                (Punct::Star, BinaryOp::Mul),
                (Punct::Slash, BinaryOp::Div),
                (Punct::Percent, BinaryOp::Rem),
            ],
        )
    }

    fn binary(
        &mut self,
        next: fn(&mut Self) -> Result<Expr, Error>,
        ops: &[(Punct, BinaryOp)],
    ) -> Result<Expr, Error> {
        let mut left = next(self)?;
        'operators: while let TokenKind::Punct(punct) = &self.tokens[self.at].kind {
            let punct = *punct;
            for &(token, op) in ops {
                if punct == token {
                    self.at += 1;
                    let right = next(self)?;
                    left = Expr::Binary {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                    continue 'operators;
                }
            }
            break;
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, Error> {
        let op = match &self.tokens[self.at].kind {
            TokenKind::Punct(Punct::Bang) => Some(UnaryOp::Not),
            TokenKind::Punct(Punct::Minus) => Some(UnaryOp::Neg),
            _ => None,
        };
        let Some(op) = op else {
            return self.postfix();
        };
        self.at += 1;
        let operand = self.postfix()?;
        Ok(Expr::Unary {
            op,
            operand: Box::new(operand),
        })
    }

    fn postfix(&mut self) -> Result<Expr, Error> {
        let mut expr = self.primary()?;
        while let TokenKind::Punct(punct) = &self.tokens[self.at].kind {
            let punct = *punct;
            match punct {
                Punct::Dot => {
                    self.at += 1;
                    let name = self.expect_ident()?;
                    expr = Expr::Field {
                        target: Box::new(expr),
                        name,
                    };
                }
                Punct::LBracket => {
                    self.at += 1;
                    let index = self.parse_expression()?;
                    self.expect_punct(Punct::RBracket, "ожидался ']'")?;
                    expr = Expr::Index {
                        target: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                Punct::LParen => {
                    self.at += 1;
                    let args = self.arg_list()?;
                    self.expect_punct(Punct::RParen, "ожидался ')'")?;
                    expr = Expr::Call {
                        target: Box::new(expr),
                        args,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn arg_list(&mut self) -> Result<Vec<Expr>, Error> {
        let mut args = Vec::new();
        if self.tokens[self.at].kind == TokenKind::Punct(Punct::RParen) {
            return Ok(args);
        }
        loop {
            args.push(self.parse_expression()?);
            if self.tokens[self.at].kind != TokenKind::Punct(Punct::Comma) {
                break;
            }
            self.at += 1;
        }
        Ok(args)
    }

    fn expect_ident(&mut self) -> Result<String, Error> {
        let token = &self.tokens[self.at];
        match &token.kind {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.at += 1;
                Ok(name)
            }
            _ => Err(self.error(token.span, "ожидалось имя поля")),
        }
    }

    fn expect_punct(&mut self, punct: Punct, message: &str) -> Result<(), Error> {
        let token = &self.tokens[self.at];
        if token.kind == TokenKind::Punct(punct) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.error(token.span, message))
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
