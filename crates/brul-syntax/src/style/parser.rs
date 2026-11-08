use crate::error::Error;
use crate::style::ast::{ClassRule, Stylesheet};

pub fn parse_file(source: &str) -> Result<Stylesheet, Error> {
    let mut parser = Parser { source, pos: 0 };
    let mut rules = Vec::new();
    loop {
        parser.skip_trivia();
        if parser.peek().is_none() {
            break;
        }
        rules.push(parser.class_rule()?);
    }
    Ok(Stylesheet { rules })
}

struct Parser<'a> {
    source: &'a str,
    pos: usize,
}

impl Parser<'_> {
    fn class_rule(&mut self) -> Result<ClassRule, Error> {
        self.expect('.')?;
        let class = self.ident()?;
        self.expect('{')?;
        self.expect('}')?;
        Ok(ClassRule { class })
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
            if !(ch.is_ascii_alphanumeric() || ch == '-') {
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
