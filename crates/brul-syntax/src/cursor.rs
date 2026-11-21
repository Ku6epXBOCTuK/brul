use crate::error::{Error, Expected};
use crate::lexer::Punct;

pub struct Cursor<'a> {
    source: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source, pos: 0 }
    }

    pub fn source(&self) -> &'a str {
        self.source
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn value_start(&mut self) -> usize {
        self.skip_trivia();
        self.pos
    }

    pub fn skip_trivia(&mut self) {
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

    pub fn expect(&mut self, ch: char) -> Result<(), Error> {
        self.skip_trivia();
        if self.peek() == Some(ch) {
            self.bump();
            Ok(())
        } else {
            Err(self.error(Expected::Char(ch)))
        }
    }

    pub fn eat(&mut self, ch: char) -> bool {
        self.skip_trivia();
        if self.peek() == Some(ch) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub fn braced_items<T>(
        &mut self,
        mut parse: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        self.expect('{')?;
        let mut items = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                Some('}') => {
                    self.bump();
                    return Ok(items);
                }
                None => return Err(self.error(Expected::Punct(Punct::RBrace))),
                _ => items.push(parse(self)?),
            }
        }
    }

    pub fn ident(&mut self) -> Result<String, Error> {
        self.skip_trivia();
        let start = self.pos;
        match self.peek() {
            Some(ch) if ch.is_ascii_alphabetic() => self.bump(),
            _ => return Err(self.error(Expected::Word("name"))),
        }
        while self.peek().is_some_and(is_ident_char) {
            self.bump();
        }
        Ok(self.source[start..self.pos].to_owned())
    }

    pub fn number_literal(&mut self) -> Result<f64, Error> {
        self.skip_trivia();
        let start = self.pos;
        if !self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            return Err(self.error(Expected::Word("number")));
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

    pub fn eat_unit(&mut self, unit: &str) -> bool {
        self.skip_trivia();
        if self.source[self.pos..].starts_with(unit) && !self.is_ident_char(unit.len()) {
            self.pos += unit.len();
            true
        } else {
            false
        }
    }

    pub fn starts_keyword(&self, word: &str) -> bool {
        self.source[self.pos..].starts_with(word) && !self.is_ident_char(word.len())
    }

    pub fn eat_keyword(&mut self, word: &str) -> bool {
        self.skip_trivia();
        if self.starts_keyword(word) {
            self.pos += word.len();
            true
        } else {
            false
        }
    }

    pub fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    pub fn bump(&mut self) {
        if let Some(ch) = self.peek() {
            self.pos += ch.len_utf8();
        }
    }

    pub fn error(&self, reason: impl Into<Error>) -> Error {
        self.error_at(self.pos, reason)
    }

    pub fn error_at(&self, pos: usize, reason: impl Into<Error>) -> Error {
        Error::at(self.source, pos, reason)
    }

    fn is_ident_char(&self, offset: usize) -> bool {
        self.source[self.pos + offset..]
            .chars()
            .next()
            .is_some_and(is_ident_char)
    }
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'
}
