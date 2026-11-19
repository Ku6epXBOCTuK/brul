use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use crate::ast::Expr;
use crate::error::Error;
use crate::lexer::{self, Punct, TokenKind, TokenStream};
use crate::script::ast::{Function, Program, Statement};

pub fn parse(source: &str) -> Result<Program, Error> {
    let tokens = lexer::tokenize(source)?;
    let mut parser = Parser {
        stream: TokenStream::new(source, tokens),
    };
    let mut functions = Vec::new();
    let mut statements = Vec::new();
    while !parser.at_end() {
        if parser.at_keyword("function") {
            functions.push(Arc::new(parser.function()?));
        } else {
            statements.push(parser.statement()?);
        }
    }
    Ok(Program {
        functions,
        statements,
    })
}

struct Parser<'a> {
    stream: TokenStream<'a>,
}

impl<'a> Deref for Parser<'a> {
    type Target = TokenStream<'a>;

    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

impl DerefMut for Parser<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stream
    }
}

impl Parser<'_> {
    fn statement(&mut self) -> Result<Statement, Error> {
        if self.at_keyword("let") {
            return self.let_statement(false);
        }
        if self.at_keyword("const") {
            return self.let_statement(true);
        }
        if self.at_keyword("if") {
            return self.if_statement();
        }
        if self.at_keyword("function") {
            return Err(self.error(
                self.tokens[self.at].span,
                "functions are only allowed at the top level",
            ));
        }
        if self.tokens[self.at].kind == TokenKind::Punct(Punct::LBrace) {
            return self.block();
        }
        let expr = self.expression()?;
        self.expect_punct(Punct::Semicolon, "expected ';'")?;
        Ok(Statement::Expr(expr))
    }

    fn let_statement(&mut self, constant: bool) -> Result<Statement, Error> {
        self.at += 1;
        let name = self.ident("expected variable name")?;
        let value = if self.tokens[self.at].kind == TokenKind::Punct(Punct::Assign) {
            self.at += 1;
            Some(self.expression()?)
        } else if constant {
            return Err(self.error(self.tokens[self.at].span, "const requires an initializer"));
        } else {
            None
        };
        self.expect_punct(Punct::Semicolon, "expected ';'")?;
        Ok(Statement::Let {
            constant,
            name,
            value,
        })
    }

    fn function(&mut self) -> Result<Function, Error> {
        self.expect_keyword("function")?;
        let name = self.ident("expected function name")?;
        self.expect_punct(Punct::LParen, "expected '('")?;
        let mut params = Vec::new();
        if self.tokens[self.at].kind != TokenKind::Punct(Punct::RParen) {
            loop {
                params.push(self.ident("expected parameter name")?);
                if self.tokens[self.at].kind != TokenKind::Punct(Punct::Comma) {
                    break;
                }
                self.at += 1;
            }
        }
        self.expect_punct(Punct::RParen, "expected ')'")?;
        let body = self.block_statements()?;
        Ok(Function { name, params, body })
    }

    fn if_statement(&mut self) -> Result<Statement, Error> {
        self.expect_keyword("if")?;
        self.expect_punct(Punct::LParen, "expected '('")?;
        let condition = self.expression()?;
        self.expect_punct(Punct::RParen, "expected ')'")?;
        let then_branch = self.block_statements()?;
        let else_branch = if self.at_keyword("else") {
            self.at += 1;
            if self.at_keyword("if") {
                Some(Box::new(self.if_statement()?))
            } else {
                Some(Box::new(self.block()?))
            }
        } else {
            None
        };
        Ok(Statement::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn block(&mut self) -> Result<Statement, Error> {
        Ok(Statement::Block(self.block_statements()?))
    }

    fn block_statements(&mut self) -> Result<Vec<Statement>, Error> {
        self.expect_punct(Punct::LBrace, "expected '{'")?;
        let mut statements = Vec::new();
        loop {
            let token = &self.tokens[self.at];
            match token.kind {
                TokenKind::Punct(Punct::RBrace) => {
                    self.at += 1;
                    break;
                }
                TokenKind::Eof => return Err(self.error(token.span, "expected '}'")),
                _ => statements.push(self.statement()?),
            }
        }
        Ok(statements)
    }

    fn expression(&mut self) -> Result<Expr, Error> {
        crate::parser::Parser::new(&mut self.stream).parse_expression()
    }
}
