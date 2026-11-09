use crate::error::Error;
use crate::lexer::{self, Punct, Span, Token, TokenKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    File,
    Use,
    Ident,
    String,
}

impl Rule {
    pub const ALL: &[Rule] = &[Rule::File, Rule::Use, Rule::Ident, Rule::String];

    pub fn name(self) -> &'static str {
        match self {
            Rule::File => "File",
            Rule::Use => "Use",
            Rule::Ident => "Ident",
            Rule::String => "String",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    pub name: String,
    pub path: String,
}

pub fn parse(source: &str) -> Result<Vec<Use>, Error> {
    let tokens = lexer::tokenize(source)?;
    let mut parser = Parser {
        source,
        tokens,
        at: 0,
    };
    let mut uses = Vec::new();
    while !parser.at_end() {
        uses.push(parser.use_item()?);
    }
    Ok(uses)
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
        let path = self.string()?;
        self.expect_punct(Punct::Semicolon, "expected ';'")?;
        Ok(Use { name, path })
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
            _ => Err(self.error(token.span, "expected import path")),
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
