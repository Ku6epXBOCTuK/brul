use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use crate::ast::Expr;
use crate::error::{Error, Expected};
use crate::lexer::{self, Punct, TokenKind, TokenStream};
use crate::script::ast::{Function, Param, Program, Statement};
use crate::types::Type;

pub fn parse(source: &str) -> Result<Program, Error> {
    let tokens = lexer::tokenize(source)?;
    let mut parser = Parser {
        stream: TokenStream::new(source, tokens),
        in_function: false,
    };
    let mut functions = Vec::new();
    let mut statements = Vec::new();
    while !parser.at_end()? {
        if parser.at_keyword("function")? {
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

pub fn expression_list_in(source: &str) -> Result<(Vec<Expr>, usize), Error> {
    let tokens = lexer::tokenize(source)?;
    let mut parser = Parser {
        stream: TokenStream::new(source, tokens),
        in_function: false,
    };
    let mut values = Vec::new();
    loop {
        let token = parser.peek()?;
        let span = token.span;
        let kind = token.kind.clone();
        if kind == TokenKind::Eof {
            return Ok((values, span.start));
        }
        if kind == TokenKind::Punct(Punct::Semicolon) {
            return Ok((values, span.start));
        }
        values.push(parser.expression()?);
        let token = parser.peek()?;
        let span = token.span;
        let kind = token.kind.clone();
        match kind {
            TokenKind::Punct(Punct::Comma) => parser.bump(),
            TokenKind::Punct(Punct::Semicolon) | TokenKind::Eof => return Ok((values, span.start)),
            _ => return Err(parser.error(span, Expected::Punct(Punct::Comma))),
        }
    }
}

struct Parser<'a> {
    stream: TokenStream<'a>,
    in_function: bool,
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
        if self.at_keyword("let")? {
            return self.let_statement(false);
        }
        if self.at_keyword("const")? {
            return self.let_statement(true);
        }
        if self.at_keyword("if")? {
            return self.if_statement();
        }
        if self.at_keyword("return")? {
            return self.return_statement();
        }
        if self.at_keyword("function")? {
            let span = self.peek()?.span;
            return Err(self.error(span, Error::FunctionNotTopLevel));
        }
        if self.peek()?.kind == TokenKind::Punct(Punct::LBrace) {
            return self.block();
        }
        let expr = self.expression()?;
        self.expect_punct(Punct::Semicolon, Punct::Semicolon)?;
        Ok(Statement::Expr(expr))
    }

    fn let_statement(&mut self, constant: bool) -> Result<Statement, Error> {
        self.bump();
        let name = self.ident(Expected::Word("variable name"))?;
        self.expect_punct(Punct::Colon, Punct::Colon)?;
        let ty = self.type_annotation()?;
        let token = self.peek()?;
        let span = token.span;
        let has_initializer = token.kind == TokenKind::Punct(Punct::Assign);
        let value = if has_initializer {
            self.bump();
            Some(self.expression()?)
        } else if constant {
            return Err(self.error(span, Error::ConstNoInitializer));
        } else {
            None
        };
        self.expect_punct(Punct::Semicolon, Punct::Semicolon)?;
        Ok(Statement::Let {
            constant,
            name,
            ty,
            value,
        })
    }

    fn return_statement(&mut self) -> Result<Statement, Error> {
        let span = self.peek()?.span;
        let position = span.position(self.stream.source);
        if !self.in_function {
            return Err(self.error(span, Error::ReturnOutsideFunction));
        }
        self.bump();
        let value = if self.peek()?.kind == TokenKind::Punct(Punct::Semicolon) {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect_punct(Punct::Semicolon, Punct::Semicolon)?;
        Ok(Statement::Return { value, position })
    }

    fn function(&mut self) -> Result<Function, Error> {
        self.expect_keyword("function")?;
        let name = self.ident(Expected::Word("function name"))?;
        self.expect_punct(Punct::LParen, Punct::LParen)?;
        let mut params = Vec::new();
        let token = self.peek()?;
        if token.kind != TokenKind::Punct(Punct::RParen) {
            loop {
                let name = self.ident(Expected::Word("parameter name"))?;
                self.expect_punct(Punct::Colon, Punct::Colon)?;
                let ty = self.type_annotation()?;
                params.push(Param { name, ty });
                let token = self.peek()?;
                if token.kind != TokenKind::Punct(Punct::Comma) {
                    break;
                }
                self.bump();
            }
        }
        self.expect_punct(Punct::RParen, Punct::RParen)?;
        self.expect_punct(Punct::Colon, Punct::Colon)?;
        let return_type = self.type_annotation()?;
        self.in_function = true;
        let body = self.block_statements()?;
        self.in_function = false;
        Ok(Function {
            name,
            params,
            return_type,
            body,
        })
    }

    fn type_annotation(&mut self) -> Result<Type, Error> {
        let token = self.peek()?;
        let span = token.span;
        let name = match &token.kind {
            TokenKind::Ident(name) => name.clone(),
            _ => return Err(self.error(span, Expected::Word("type"))),
        };
        self.bump();
        if name == "Array" {
            self.expect_punct(Punct::Lt, Punct::Lt)?;
            let item = self.type_annotation()?;
            self.expect_punct(Punct::Gt, Punct::Gt)?;
            return Ok(Type::Array(Box::new(item)));
        }
        Type::from_name(&name).ok_or_else(|| self.error(span, Expected::Word("type")))
    }

    fn if_statement(&mut self) -> Result<Statement, Error> {
        self.expect_keyword("if")?;
        self.expect_punct(Punct::LParen, Punct::LParen)?;
        let condition = self.expression()?;
        self.expect_punct(Punct::RParen, Punct::RParen)?;
        let then_branch = self.block_statements()?;
        let else_branch = if self.at_keyword("else")? {
            self.bump();
            if self.at_keyword("if")? {
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
        self.expect_punct(Punct::LBrace, Punct::LBrace)?;
        let mut statements = Vec::new();
        loop {
            let token = self.peek()?;
            let span = token.span;
            if token.kind == TokenKind::Punct(Punct::RBrace) {
                self.bump();
                break;
            }
            if token.kind == TokenKind::Eof {
                return Err(self.error(span, Expected::Punct(Punct::RBrace)));
            }
            statements.push(self.statement()?);
        }
        Ok(statements)
    }

    fn expression(&mut self) -> Result<Expr, Error> {
        crate::parser::Parser::new(&mut self.stream).parse_expression()
    }
}
