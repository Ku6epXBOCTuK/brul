use crate::cursor::Cursor;
use crate::error::Error;
use crate::style::ast::{ClassRule, Property, State, Stylesheet, Theme, ThemeEntry};
use crate::style::properties::Kind;
use crate::style::shorthand;
use crate::style::validate::{self, PropertyAt, RuleAt, ThemeAt, ThemeKeyAt};
use crate::style::value::{self, Type};

pub fn parse_file(source: &str) -> Result<Stylesheet, Error> {
    let mut parser = Parser::new(source);
    let mut themes = Vec::new();
    let mut theme_positions = Vec::new();
    let mut rules = Vec::new();
    let mut positions = Vec::new();
    loop {
        parser.cursor.skip_trivia();
        if parser.cursor.peek().is_none() {
            break;
        }
        if parser.cursor.eat_keyword("theme") {
            let (theme, position) = parser.theme()?;
            themes.push(theme);
            theme_positions.push(position);
        } else {
            let (rule, position) = parser.class_rule()?;
            rules.push(rule);
            positions.push(position);
        }
    }
    validate::themes(source, &themes, &theme_positions)?;
    validate::refs(source, &themes, &rules, &positions)?;
    validate::states(source, &rules, &positions)?;
    Ok(Stylesheet { themes, rules })
}

struct Parser<'a> {
    cursor: Cursor<'a>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            cursor: Cursor::new(source),
        }
    }

    fn class_rule(&mut self) -> Result<(ClassRule, RuleAt), Error> {
        self.cursor.expect('.')?;
        let class_at = self.cursor.value_start();
        let class = self.cursor.ident()?;
        let state = self.state()?;
        self.cursor.expect('{')?;
        let mut properties = Vec::new();
        let mut positions = Vec::new();
        loop {
            self.cursor.skip_trivia();
            match self.cursor.peek() {
                Some('}') => {
                    self.cursor.bump();
                    return Ok((
                        ClassRule {
                            class,
                            state,
                            properties,
                        },
                        RuleAt {
                            class_at,
                            properties: positions,
                        },
                    ));
                }
                None => return Err(self.cursor.error("expected '}'")),
                _ => {
                    for (property, at) in self.property()? {
                        properties.push(property);
                        positions.push(at);
                    }
                }
            }
        }
    }

    fn state(&mut self) -> Result<Option<State>, Error> {
        if !self.cursor.eat(':') {
            return Ok(None);
        }
        let at = self.cursor.value_start();
        let name = self.cursor.ident()?;
        let state = match name.as_str() {
            "hover" => State::Hover,
            "pressed" => State::Pressed,
            "checked" => State::Checked,
            "selected" => State::Selected,
            "disabled" => State::Disabled,
            _ => return Err(Error::at(self.cursor.source(), at, "unknown state")),
        };
        Ok(Some(state))
    }

    fn theme(&mut self) -> Result<(Theme, ThemeAt), Error> {
        let name_at = self.cursor.value_start();
        let name = self.cursor.ident()?;
        self.cursor.expect('{')?;
        let mut properties = Vec::new();
        let mut positions = Vec::new();
        loop {
            self.cursor.skip_trivia();
            match self.cursor.peek() {
                Some('}') => {
                    self.cursor.bump();
                    return Ok((
                        Theme { name, properties },
                        ThemeAt {
                            name_at,
                            properties: positions,
                        },
                    ));
                }
                None => return Err(self.cursor.error("expected '}'")),
                _ => {
                    let (property, at) = self.theme_property()?;
                    properties.push(property);
                    positions.push(at);
                }
            }
        }
    }

    fn property(&mut self) -> Result<Vec<(Property, PropertyAt)>, Error> {
        let at = self.cursor.value_start();
        let name = self.cursor.ident()?;
        if let Some(shorthand) = shorthand::get(&name) {
            self.cursor.expect(':')?;
            let values = (shorthand.parse)(&mut self.cursor, &name)?;
            self.cursor.expect(';')?;
            return Ok(shorthand
                .properties(values)
                .into_iter()
                .map(|property| (property, PropertyAt { at }))
                .collect());
        }
        let kind = Kind::from_name(&name).ok_or_else(|| {
            Error::at(
                self.cursor.source(),
                at,
                format!("unknown property '{name}'"),
            )
        })?;
        let spec = kind.spec();
        self.cursor.expect(':')?;
        let value = value::value(
            &mut self.cursor,
            spec.theme_type().is_some(),
            spec.ty,
            &name,
        )?;
        self.cursor.expect(';')?;
        Ok(vec![(Property { kind, value }, PropertyAt { at })])
    }

    fn theme_property(&mut self) -> Result<(ThemeEntry, ThemeKeyAt), Error> {
        let at = self.cursor.value_start();
        let key = self.cursor.ident()?;
        self.cursor.expect(':')?;
        let value = Type::ThemeKey.parse(&mut self.cursor, &key)?;
        self.cursor.expect(';')?;
        Ok((
            ThemeEntry {
                key: key.clone(),
                value,
            },
            ThemeKeyAt { key, at },
        ))
    }
}
