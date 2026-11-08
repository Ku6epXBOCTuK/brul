use crate::error::Error;
use crate::style::ast::{Border, ClassRule, Color, Gradient, Property, Stylesheet, Theme, Value};

pub fn parse_file(source: &str) -> Result<Stylesheet, Error> {
    let mut parser = Parser { source, pos: 0 };
    let mut themes = Vec::new();
    let mut rules = Vec::new();
    loop {
        parser.skip_trivia();
        if parser.peek().is_none() {
            break;
        }
        if parser.keyword("theme") {
            parser.pos += "theme".len();
            themes.push(parser.theme()?);
        } else {
            rules.push(parser.class_rule()?);
        }
    }
    Ok(Stylesheet { themes, rules })
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

    fn theme(&mut self) -> Result<Theme, Error> {
        let name = self.ident()?;
        self.expect('{')?;
        let mut properties = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                Some('}') => {
                    self.bump();
                    return Ok(Theme { name, properties });
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
            Some(ch) if ch.is_ascii_digit() => self.number_or_border(),
            Some('"') => self.string(),
            Some('#') => Ok(Value::Color(self.color()?)),
            Some(ch) if ch.is_ascii_alphabetic() => {
                let name = self.ident()?;
                match name.as_str() {
                    "theme" => {
                        self.expect('.')?;
                        Ok(Value::ThemeRef(self.ident()?))
                    }
                    "linear-gradient" => self.gradient(),
                    "transparent" => Ok(Value::Color(Color::TRANSPARENT)),
                    "white" => Ok(Value::Color(Color::WHITE)),
                    _ => Ok(Value::Keyword(name)),
                }
            }
            _ => Err(self.error("expected value")),
        }
    }

    fn gradient(&mut self) -> Result<Value, Error> {
        self.expect('(')?;
        let angle = self.number_literal()?;
        self.skip_trivia();
        if self.keyword("deg") {
            self.pos += "deg".len();
        }
        self.expect(',')?;
        let from = self.color()?;
        self.expect(',')?;
        let to = self.color()?;
        self.expect(')')?;
        Ok(Value::Gradient(Gradient { angle, from, to }))
    }

    fn color(&mut self) -> Result<Color, Error> {
        self.skip_trivia();
        let start = self.pos;
        match self.peek() {
            Some('#') => self.hex_color(),
            Some(ch) if ch.is_ascii_alphabetic() => {
                let name = self.ident()?;
                match name.as_str() {
                    "transparent" => Ok(Color::TRANSPARENT),
                    "white" => Ok(Color::WHITE),
                    _ => Err(Error::at(self.source, start, "expected color")),
                }
            }
            _ => Err(Error::at(self.source, start, "expected color")),
        }
    }

    fn hex_color(&mut self) -> Result<Color, Error> {
        let start = self.pos;
        self.bump();
        let digits_start = self.pos;
        while self.peek().is_some_and(|ch| ch.is_ascii_hexdigit()) {
            self.bump();
        }
        let digits = &self.source[digits_start..self.pos];
        let mut channels = [0u8; 4];
        match digits.len() {
            3 => {
                for (index, ch) in digits.chars().enumerate() {
                    let value = hex(ch);
                    channels[index] = value * 17;
                }
                Ok(Color::rgb(channels[0], channels[1], channels[2]))
            }
            6 | 8 => {
                for (index, pair) in digits.as_bytes().chunks(2).enumerate() {
                    let value = hex(pair[0] as char) * 16 + hex(pair[1] as char);
                    channels[index] = value;
                }
                if digits.len() == 6 {
                    Ok(Color::rgb(channels[0], channels[1], channels[2]))
                } else {
                    Ok(Color::rgba(
                        channels[0],
                        channels[1],
                        channels[2],
                        channels[3],
                    ))
                }
            }
            _ => Err(Error::at(
                self.source,
                start,
                "expected 3, 6 or 8 hex digits",
            )),
        }
    }

    fn number_or_border(&mut self) -> Result<Value, Error> {
        match self.number()? {
            Value::Px(width) => {
                self.skip_trivia();
                if self.peek() == Some('#')
                    || self.peek().is_some_and(|ch| ch.is_ascii_alphabetic())
                {
                    let color = self.color()?;
                    return Ok(Value::Border(Border { width, color }));
                }
                Ok(Value::Px(width))
            }
            other => Ok(other),
        }
    }

    fn number(&mut self) -> Result<Value, Error> {
        let value = self.number_literal()?;
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

    fn number_literal(&mut self) -> Result<f64, Error> {
        self.skip_trivia();
        let start = self.pos;
        if !self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            return Err(self.error("expected number"));
        }
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
        Ok(self.source[start..self.pos]
            .parse()
            .expect("parser accepts only numbers"))
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

    fn keyword(&mut self, word: &str) -> bool {
        self.skip_trivia();
        let rest = &self.source[self.pos..];
        if !rest.starts_with(word) {
            return false;
        }
        rest[word.len()..]
            .chars()
            .next()
            .is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '-'))
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

fn hex(ch: char) -> u8 {
    ch.to_digit(16).expect("parser accepts only hex digits") as u8
}
