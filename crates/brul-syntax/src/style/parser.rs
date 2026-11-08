use crate::error::Error;
use crate::style::ast::{ClassRule, Property, Stylesheet, Value};

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
        let mut properties = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                Some('}') => {
                    self.bump();
                    return Ok(ClassRule { class, properties });
                }
                None => return Err(self.error("expected '}'")),
                _ => properties.push(self.property()?),
            }
        }
    }

    fn property(&mut self) -> Result<Property, Error> {
        let name = self.ident()?;
        self.expect(':')?;
        let value = self.value()?;
        self.expect(';')?;
        Ok(Property { name, value })
    }

    fn value(&mut self) -> Result<Value, Error> {
        self.skip_trivia();
        match self.peek() {
            Some(ch) if ch.is_ascii_digit() => self.number(),
            Some('"') => self.string(),
            Some(ch) if ch.is_ascii_alphabetic() => Ok(Value::Keyword(self.ident()?)),
            _ => Err(self.error("expected value")),
        }
    }

    fn number(&mut self) -> Result<Value, Error> {
        let start = self.pos;
        while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            self.bump();
        }
        if self.peek() == Some('.')
            && self.source[self.pos + 1..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_digit())
        {
            self.bump();
            while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                self.bump();
            }
        }
        let value: f64 = self.source[start..self.pos]
            .parse()
            .expect("parser accepts only numbers");
        self.skip_trivia();
        if self.source[self.pos..].starts_with("px") && !self.is_ident_char(2) {
            self.pos += 2;
            return Ok(Value::Px(value));
        }
        if self.peek() == Some('%') {
            self.bump();
            return Ok(Value::Percent(value));
        }
        Ok(Value::Number(value))
    }

    fn string(&mut self) -> Result<Value, Error> {
        let start = self.pos;
        self.bump();
        let value_start = self.pos;
        loop {
            match self.peek() {
                Some('"') => {
                    let value = self.source[value_start..self.pos].to_owned();
                    self.bump();
                    return Ok(Value::String(value));
                }
                Some('\n') | None => {
                    return Err(Error::at(self.source, start, "unterminated string"));
                }
                Some(_) => self.bump(),
            }
        }
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
        while self
            .peek()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        {
            self.bump();
        }
        Ok(self.source[start..self.pos].to_owned())
    }

    fn is_ident_char(&self, offset: usize) -> bool {
        self.source[self.pos + offset..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '-')
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
