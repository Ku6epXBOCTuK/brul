use crate::error::Error;
use crate::style::ast::{
    Border, ClassRule, Color, Gradient, Property, State, Stylesheet, Theme, Value,
};

pub fn parse_file(source: &str) -> Result<Stylesheet, Error> {
    let mut parser = Parser { source, pos: 0 };
    let mut themes = Vec::new();
    let mut theme_positions = Vec::new();
    let mut rules = Vec::new();
    let mut positions = Vec::new();
    loop {
        parser.skip_trivia();
        if parser.peek().is_none() {
            break;
        }
        if parser.keyword("theme") {
            parser.pos += "theme".len();
            let index = themes.len();
            let (theme, name_at, property_positions) = parser.theme()?;
            themes.push(theme);
            theme_positions.push((index, name_at, property_positions));
        } else {
            let index = rules.len();
            let (rule, property_positions) = parser.class_rule()?;
            rules.push(rule);
            positions.push((index, property_positions));
        }
    }
    validate_themes(source, &themes, &theme_positions)?;
    validate_states(source, &rules, &positions)?;
    Ok(Stylesheet { themes, rules })
}

type PropertyPositions = Vec<(String, usize)>;
type ThemeInfo = (usize, usize, PropertyPositions);

struct Parser<'a> {
    source: &'a str,
    pos: usize,
}

impl Parser<'_> {
    fn class_rule(&mut self) -> Result<(ClassRule, Vec<(String, usize)>), Error> {
        self.expect('.')?;
        let class = self.ident()?;
        let state = self.state()?;
        self.expect('{')?;
        let mut properties = Vec::new();
        let mut positions = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                Some('}') => {
                    self.bump();
                    return Ok((
                        ClassRule {
                            class,
                            state,
                            properties,
                        },
                        positions,
                    ));
                }
                None => return Err(self.error("expected '}'")),
                _ => {
                    let (property, start) = self.property_with_start()?;
                    validate(self.source, start, &property)?;
                    positions.push((property.name.clone(), start));
                    properties.push(property);
                }
            }
        }
    }

    fn state(&mut self) -> Result<Option<State>, Error> {
        self.skip_trivia();
        if self.peek() != Some(':') {
            return Ok(None);
        }
        self.bump();
        self.skip_trivia();
        let start = self.pos;
        let name = self.ident()?;
        let state = match name.as_str() {
            "hover" => State::Hover,
            "pressed" => State::Pressed,
            "checked" => State::Checked,
            "selected" => State::Selected,
            "disabled" => State::Disabled,
            _ => return Err(Error::at(self.source, start, "unknown state")),
        };
        Ok(Some(state))
    }

    fn theme(&mut self) -> Result<(Theme, usize, PropertyPositions), Error> {
        self.skip_trivia();
        let name_at = self.pos;
        let name = self.ident()?;
        self.expect('{')?;
        let mut properties = Vec::new();
        let mut positions = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                Some('}') => {
                    self.bump();
                    return Ok((Theme { name, properties }, name_at, positions));
                }
                None => return Err(self.error("expected '}'")),
                _ => {
                    let (property, start) = self.property_with_start()?;
                    positions.push((property.name.clone(), start));
                    properties.push(property);
                }
            }
        }
    }

    fn property_with_start(&mut self) -> Result<(Property, usize), Error> {
        self.skip_trivia();
        let start = self.pos;
        let name = self.ident()?;
        self.expect(':')?;
        let value = self.value()?;
        self.expect(';')?;
        Ok((Property { name, value }, start))
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

fn validate(source: &str, at: usize, property: &Property) -> Result<(), Error> {
    let value = &property.value;
    let ok = match property.name.as_str() {
        "width" | "height" => {
            matches!(value, Value::Px(_) | Value::Percent(_) | Value::ThemeRef(_))
        }
        "stretch" => matches!(value, Value::Number(_) | Value::ThemeRef(_)),
        "padding" | "padding-left" | "padding-right" | "padding-top" | "padding-bottom"
        | "spacing" | "border-width" | "radius" | "font-size" => {
            matches!(value, Value::Px(_) | Value::ThemeRef(_))
        }
        "justify-content" => keyword_in(value, &["start", "center", "end", "space-between"]),
        "align-items" => keyword_in(value, &["start", "center", "end", "stretch"]),
        "background" => {
            matches!(
                value,
                Value::Color(_) | Value::Gradient(_) | Value::ThemeRef(_)
            )
        }
        "border" => matches!(value, Value::Border(_)),
        "border-color" | "color" | "tint" => {
            matches!(value, Value::Color(_) | Value::ThemeRef(_))
        }
        "font-family" => matches!(value, Value::String(_)),
        "text-align" => keyword_in(value, &["left", "center", "right"]),
        "vertical-align" => keyword_in(value, &["top", "center", "bottom"]),
        "font-weight" => {
            matches!(value, Value::Number(weight) if matches!(weight, 400.0 | 500.0 | 600.0 | 700.0))
        }
        "opacity" => matches!(value, Value::Number(opacity) if (0.0..=1.0).contains(opacity)),
        _ => {
            return Err(Error::at(
                source,
                at,
                format!("unknown property '{}'", property.name),
            ));
        }
    };
    if ok {
        Ok(())
    } else {
        Err(Error::at(
            source,
            at,
            format!("unexpected value for '{}'", property.name),
        ))
    }
}

fn keyword_in(value: &Value, names: &[&str]) -> bool {
    matches!(value, Value::Keyword(keyword) if names.contains(&keyword.as_str()))
}

fn same_type(left: &Value, right: &Value) -> bool {
    matches!(
        (left, right),
        (Value::Number(_), Value::Number(_))
            | (Value::Px(_), Value::Px(_))
            | (Value::Percent(_), Value::Percent(_))
            | (Value::String(_), Value::String(_))
            | (Value::Keyword(_), Value::Keyword(_))
            | (Value::Color(_), Value::Color(_))
            | (Value::ThemeRef(_), Value::ThemeRef(_))
            | (Value::Gradient(_), Value::Gradient(_))
            | (Value::Border(_), Value::Border(_))
    )
}

fn validate_themes(source: &str, themes: &[Theme], positions: &[ThemeInfo]) -> Result<(), Error> {
    let Some((first_index, _, _)) = positions.first() else {
        return Ok(());
    };
    let first = &themes[*first_index];
    for (index, name_at, properties) in positions.iter().skip(1) {
        let theme = &themes[*index];
        if themes[..*index]
            .iter()
            .any(|other| other.name == theme.name)
        {
            return Err(Error::at(
                source,
                *name_at,
                format!("duplicate theme '{}'", theme.name),
            ));
        }
        for ((name, at), property) in properties.iter().zip(&theme.properties) {
            match first.properties.iter().find(|item| item.name == *name) {
                None => {
                    return Err(Error::at(
                        source,
                        *at,
                        format!("theme '{}' has unknown key '{}'", theme.name, name),
                    ));
                }
                Some(base) if !same_type(&base.value, &property.value) => {
                    return Err(Error::at(
                        source,
                        *at,
                        format!("theme '{}' has a different type for '{}'", theme.name, name),
                    ));
                }
                Some(_) => {}
            }
        }
        for base in &first.properties {
            if !properties.iter().any(|(name, _)| *name == base.name) {
                return Err(Error::at(
                    source,
                    *name_at,
                    format!("theme '{}' is missing key '{}'", theme.name, base.name),
                ));
            }
        }
    }
    Ok(())
}

const VISUAL: &[&str] = &[
    "background",
    "border",
    "border-color",
    "color",
    "text-align",
    "vertical-align",
    "tint",
    "opacity",
];

fn validate_states(
    source: &str,
    rules: &[ClassRule],
    positions: &[(usize, Vec<(String, usize)>)],
) -> Result<(), Error> {
    for (index, properties) in positions {
        let rule = &rules[*index];
        if rule.state.is_none() {
            continue;
        }
        for (name, at) in properties {
            if name == "font-weight" {
                let fixed = rules.iter().any(|base| {
                    base.state.is_none()
                        && base.class == rule.class
                        && base.properties.iter().any(|item| item.name == "width")
                        && base.properties.iter().any(|item| item.name == "height")
                });
                if !fixed {
                    return Err(Error::at(
                        source,
                        *at,
                        "font-weight in a state needs width and height in the base class",
                    ));
                }
                continue;
            }
            if !VISUAL.contains(&name.as_str()) {
                return Err(Error::at(
                    source,
                    *at,
                    format!("property '{name}' cannot change in a state"),
                ));
            }
        }
    }
    Ok(())
}
