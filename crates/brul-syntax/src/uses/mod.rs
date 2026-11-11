mod rule;

use crate::error::Error;
use crate::lexer::{self, Punct, Span, Token, TokenKind};

pub use rule::Rule;

#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resource {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Block {
    pub uses: Vec<Use>,
    pub resources: Vec<Resource>,
}

pub fn parse(source: &str) -> Result<Block, Error> {
    let tokens = lexer::tokenize(source)?;
    let mut parser = Parser {
        source,
        tokens,
        at: 0,
    };
    let mut block = Block::default();
    while !parser.at_end() {
        if parser.at_keyword("resource") {
            block.resources.push(parser.resource_item()?);
        } else {
            block.uses.push(parser.use_item()?);
        }
    }
    Ok(block)
}

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    at: usize,
}

impl Parser<'_> {
    fn use_item(&mut self) -> Result<Use, Error> {
        self.expect_keyword("import")?;
        let name = self.ident("expected import name")?;
        self.expect_keyword("from")?;
        let path = self.string("expected import path")?;
        self.expect_punct(Punct::Semicolon, "expected ';'")?;
        Ok(Use { name, path })
    }

    fn resource_item(&mut self) -> Result<Resource, Error> {
        self.expect_keyword("resource")?;
        let name = self.ident("expected resource name")?;
        self.expect_keyword("from")?;
        let path = self.string("expected resource path")?;
        self.expect_punct(Punct::Semicolon, "expected ';'")?;
        Ok(Resource { name, path })
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

    fn string(&mut self, message: &str) -> Result<String, Error> {
        let token = &self.tokens[self.at];
        match &token.kind {
            TokenKind::String(value) => {
                let value = value.clone();
                self.at += 1;
                Ok(value)
            }
            _ => Err(self.error(token.span, message)),
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
