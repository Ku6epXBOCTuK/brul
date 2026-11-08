use crate::error::Error;
use crate::markup::ast::{
    Arg, Component, Declaration, Event, EventParam, File, Member, Prop, PropType, Window,
};

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
        let members = self.members()?;
        Ok(Declaration::Component(Component { name, members }))
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
        let members = self.members()?;
        Ok(Declaration::Window(Window {
            name,
            args,
            members,
        }))
    }

    fn members(&mut self) -> Result<Vec<Member>, Error> {
        self.expect('{')?;
        let mut members = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                Some('}') => {
                    self.bump();
                    return Ok(members);
                }
                None => return Err(self.error("expected '}'")),
                _ => members.push(self.member()?),
            }
        }
    }

    fn member(&mut self) -> Result<Member, Error> {
        self.skip_trivia();
        let start = self.pos;
        let keyword = self.ident()?;
        match keyword.as_str() {
            "prop" => self.prop(),
            "event" => self.event(),
            _ => Err(Error::at(self.source, start, "expected 'prop' or 'event'")),
        }
    }

    fn prop(&mut self) -> Result<Member, Error> {
        let name = self.ident()?;
        self.expect(':')?;
        let ty = self.prop_type()?;
        self.skip_trivia();
        let default = if self.peek() == Some('=') {
            self.bump();
            let (expr, end) = crate::parser::parse_expression_at(self.source, self.pos)?;
            self.pos = end;
            Some(expr)
        } else {
            None
        };
        Ok(Member::Prop(Prop { name, ty, default }))
    }

    fn event(&mut self) -> Result<Member, Error> {
        let name = self.ident()?;
        let mut params = Vec::new();
        self.skip_trivia();
        if self.peek() == Some('(') {
            self.bump();
            self.skip_trivia();
            if self.peek() == Some(')') {
                self.bump();
            } else {
                loop {
                    let param = self.ident()?;
                    self.expect(':')?;
                    let ty = self.prop_type()?;
                    params.push(EventParam { name: param, ty });
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
            }
        }
        Ok(Member::Event(Event { name, params }))
    }

    fn prop_type(&mut self) -> Result<PropType, Error> {
        self.skip_trivia();
        let start = self.pos;
        let name = self.ident()?;
        match name.as_str() {
            "Bool" => Ok(PropType::Bool),
            "Number" => Ok(PropType::Number),
            "String" => Ok(PropType::String),
            "Image" => Ok(PropType::Image),
            _ => Err(Error::at(self.source, start, "expected property type")),
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
