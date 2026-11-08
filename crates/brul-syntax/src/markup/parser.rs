use crate::error::Error;
use crate::markup::ast::{Arg, Component, Declaration, File, Window};

pub fn parse_file(source: &str) -> Result<File, Error> {
    let mut parser = Parser { source, pos: 0 };
    let mut declarations = Vec::new();
    loop {
        parser.skip_trivia();
        if parser.peek().is_none() {
            break;
        }
        declarations.push(parser.declaration()?);
    }
    Ok(File { declarations })
}

struct Parser<'a> {
    source: &'a str,
    pos: usize,
}

impl Parser<'_> {
    fn declaration(&mut self) -> Result<Declaration, Error> {
        self.skip_trivia();
        let start = self.pos;
        let keyword = self.ident()?;
        match keyword.as_str() {
            "component" => self.component(),
            "window" => self.window(),
            _ => Err(Error::at(
                self.source,
                start,
                "expected 'component' or 'window'",
            )),
        }
    }

    fn component(&mut self) -> Result<Declaration, Error> {
        let name = self.ident()?;
        self.empty_block()?;
        Ok(Declaration::Component(Component { name }))
    }

    fn window(&mut self) -> Result<Declaration, Error> {
        let name = self.ident()?;
        self.expect('(')?;
        let mut args = Vec::new();
        loop {
            args.push(Arg::Class(self.ident()?));
            self.skip_trivia();
            match self.peek() {
                Some(',') => self.bump(),
                Some(')') => {
                    self.bump();
                    break;
                }
                _ => return Err(self.error("expected ',' or ')'")),
            }
        }
        self.empty_block()?;
        Ok(Declaration::Window(Window { name, args }))
    }

    fn empty_block(&mut self) -> Result<(), Error> {
        self.expect('{')?;
        self.expect('}')
    }

    fn expect(&mut self, ch: char) -> Result<(), Error> {
        self.skip_trivia();
        if self.peek() == Some(ch) {
            self.bump();
            Ok(())
        } else {
            Err(self.error(format!("expected '{ch}'")))
        }
    }

    fn ident(&mut self) -> Result<String, Error> {
        self.skip_trivia();
        let start = self.pos;
        match self.peek() {
            Some(ch) if ch.is_ascii_alphabetic() => self.bump(),
            _ => return Err(self.error("expected name")),
        }
        while let Some(ch) = self.peek() {
            if !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') {
                break;
            }
            self.bump();
        }
        Ok(self.source[start..self.pos].to_owned())
    }

    fn skip_trivia(&mut self) {
        loop {
            let Some(ch) = self.peek() else { return };
            if ch.is_whitespace() {
                self.bump();
            } else if self.source[self.pos..].starts_with("//") {
                self.pos = self.source[self.pos..]
                    .find('\n')
                    .map_or(self.source.len(), |offset| self.pos + offset);
            } else {
                return;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn bump(&mut self) {
        if let Some(ch) = self.peek() {
            self.pos += ch.len_utf8();
        }
    }

    fn error(&self, message: impl Into<String>) -> Error {
        Error::at(self.source, self.pos, message)
    }
}
