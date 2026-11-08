use crate::ast::Expr;
use crate::error::Error;
use crate::lexer::{self, Punct, Span, Token, TokenKind};
use crate::script::ast::{Import, Program, Statement};

pub fn parse(source: &str) -> Result<Program, Error> {
    let tokens = lexer::tokenize(source)?;
    let mut parser = Parser {
        source,
        tokens,
        at: 0,
    };
    let mut imports = Vec::new();
    while parser.at_keyword("import") {
        imports.push(parser.import()?);
    }
    let mut statements = Vec::new();
    while !parser.at_end() {
        statements.push(parser.statement()?);
    }
    Ok(Program {
        imports,
        statements,
    })
}

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    at: usize,
}

impl Parser<'_> {
    fn import(&mut self) -> Result<Import, Error> {
        self.expect_keyword("import")?;
        let name = self.ident("expected import name")?;
        self.expect_keyword("from")?;
        let path = self.string()?;
        self.expect_punct(Punct::Semicolon, "expected ';'")?;
        Ok(Import { name, path })
    }

    fn statement(&mut self) -> Result<Statement, Error> {
        if self.at_keyword("let") {
            return self.let_statement(false);
        }
        if self.at_keyword("const") {
            return self.let_statement(true);
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

    fn block(&mut self) -> Result<Statement, Error> {
        self.at += 1;
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
        Ok(Statement::Block(statements))
    }

    fn expression(&mut self) -> Result<Expr, Error> {
        let start = self.tokens[self.at].span.start;
        let (expr, end) = crate::parser::parse_expression_at(self.source, start)?;
        while self.tokens[self.at].span.start < end {
            self.at += 1;
        }
        Ok(expr)
    }

    fn at_keyword(&self, keyword: &str) -> bool {
        matches!(&self.tokens[self.at].kind, TokenKind::Ident(name) if name == keyword)
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<(), Error> {
        let token = &self.tokens[self.at];
        match &token.kind {
            TokenKind::Ident(name) if name == keyword => {
                self.at += 1;
                Ok(())
            }
            _ => Err(self.error(token.span, format!("expected '{keyword}'"))),
        }
    }

    fn ident(&mut self, message: &str) -> Result<String, Error> {
        let token = &self.tokens[self.at];
        match &token.kind {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.at += 1;
                Ok(name)
            }
            _ => Err(self.error(token.span, message)),
        }
    }

    fn string(&mut self) -> Result<String, Error> {
        let token = &self.tokens[self.at];
        match &token.kind {
            TokenKind::String(value) => {
                let value = value.clone();
                self.at += 1;
                Ok(value)
            }
            _ => Err(self.error(token.span, "expected module path")),
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

    fn at_end(&self) -> bool {
        self.tokens[self.at].kind == TokenKind::Eof
    }

    fn error(&self, span: Span, message: impl Into<String>) -> Error {
        let (line, column) = span.line_col(self.source);
        Error {
            message: message.into(),
            line,
            column,
        }
    }
}
