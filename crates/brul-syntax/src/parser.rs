use crate::ast::{AssignOp, BinaryOp, Expr, TemplatePart, UnaryOp};
pub use crate::error::Error;
use crate::lexer::{self, Punct, Span, Token, TokenKind};

pub struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    at: usize,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Result<Self, Error> {
        Self::at(source, 0)
    }

    fn at(source: &'a str, offset: usize) -> Result<Self, Error> {
        let mut tokens =
            lexer::tokenize(&source[offset..]).map_err(|error| rebase(source, offset, error))?;
        for token in &mut tokens {
            token.span.start += offset;
            token.span.end += offset;
        }
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
            Err(self.error(token.span, "expected end of expression"))
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
            TokenKind::Punct(Punct::LBracket) => self.array(),
            TokenKind::TemplateStart => self.template(),
            TokenKind::Punct(Punct::LBrace) => self.object(),
            TokenKind::Punct(Punct::LParen) => {
                self.at += 1;
                let expr = self.parse_expression()?;
                self.expect_punct(Punct::RParen, "expected ')'")?;
                Ok(expr)
            }
            _ => Err(self.error(span, "expected expression")),
        }
    }

    fn array(&mut self) -> Result<Expr, Error> {
        self.at += 1;
        let mut items = Vec::new();
        if self.tokens[self.at].kind == TokenKind::Punct(Punct::RBracket) {
            self.at += 1;
            return Ok(Expr::Array(items));
        }
        loop {
            items.push(self.parse_expression()?);
            if self.tokens[self.at].kind != TokenKind::Punct(Punct::Comma) {
                break;
            }
            self.at += 1;
        }
        self.expect_punct(Punct::RBracket, "expected ']'")?;
        Ok(Expr::Array(items))
    }

    fn object(&mut self) -> Result<Expr, Error> {
        self.at += 1;
        let mut fields = Vec::new();
        if self.tokens[self.at].kind == TokenKind::Punct(Punct::RBrace) {
            self.at += 1;
            return Ok(Expr::Object(fields));
        }
        loop {
            let name = self.expect_ident()?;
            self.expect_punct(Punct::Colon, "expected ':'")?;
            fields.push((name, self.parse_expression()?));
            if self.tokens[self.at].kind != TokenKind::Punct(Punct::Comma) {
                break;
            }
            self.at += 1;
        }
        self.expect_punct(Punct::RBrace, "expected '}'")?;
        Ok(Expr::Object(fields))
    }

    fn template(&mut self) -> Result<Expr, Error> {
        self.at += 1;
        let mut parts = Vec::new();
        loop {
            match self.tokens[self.at].kind.clone() {
                TokenKind::TemplateChunk(text) => {
                    self.at += 1;
                    parts.push(TemplatePart::Text(text));
                }
                TokenKind::TemplateInterpStart => {
                    self.at += 1;
                    let expr = self.parse_expression()?;
                    if self.tokens[self.at].kind != TokenKind::TemplateInterpEnd {
                        let span = self.tokens[self.at].span;
                        return Err(self.error(span, "expected '}'"));
                    }
                    self.at += 1;
                    parts.push(TemplatePart::Expr(Box::new(expr)));
                }
                TokenKind::TemplateEnd => {
                    self.at += 1;
                    break;
                }
                _ => {
                    let span = self.tokens[self.at].span;
                    return Err(self.error(span, "expected end of template"));
                }
            }
        }
        Ok(Expr::Template(parts))
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
        self.expect_punct(Punct::Colon, "expected ':'")?;
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
                    self.expect_punct(Punct::RBracket, "expected ']'")?;
                    expr = Expr::Index {
                        target: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                Punct::LParen => {
                    self.at += 1;
                    let args = self.arg_list()?;
                    self.expect_punct(Punct::RParen, "expected ')'")?;
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
            _ => Err(self.error(token.span, "expected field name")),
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

pub fn parse_expression_at(source: &str, offset: usize) -> Result<(Expr, usize), Error> {
    let mut parser = Parser::at(source, offset)?;
    let expr = parser.parse_expression()?;
    let end = parser.tokens[parser.at - 1].span.end;
    Ok((expr, end))
}

fn rebase(source: &str, offset: usize, error: lexer::Error) -> Error {
    let (base_line, base_column) = Span {
        start: offset,
        end: offset,
    }
    .line_col(source);
    let (line, column) = if error.line == 1 {
        (base_line, base_column + error.column - 1)
    } else {
        (base_line + error.line - 1, error.column)
    };
    Error {
        message: error.message,
        line,
        column,
    }
}
