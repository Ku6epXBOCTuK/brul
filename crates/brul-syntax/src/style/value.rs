use crate::cursor::Cursor;
use crate::error::Error;
use crate::style::ast::{Color, Gradient, Value};

#[derive(Clone, Copy)]
pub enum Type {
    ThemeKey,
    Number,
    Px,
    SignedPx,
    Length,
    Color,
    Paint,
    String,
    Bool,
    Keywords(&'static [&'static str]),
    NumberRange(f64, f64),
    NumberIn(&'static [f64]),
}

pub fn value(cursor: &mut Cursor, theme: bool, ty: Type, name: &str) -> Result<Value, Error> {
    if theme && cursor.eat_keyword("theme") {
        cursor.expect('.')?;
        return Ok(Value::ThemeRef(cursor.ident()?));
    }
    ty.parse(cursor, name)
}

impl Type {
    pub fn parse(self, cursor: &mut Cursor, name: &str) -> Result<Value, Error> {
        match self {
            Type::Number | Type::Px | Type::SignedPx | Type::Length => {
                self.number_value(cursor, name)
            }
            Type::Color | Type::Paint => self.paint_value(cursor, name),
            Type::Bool => boolean(cursor, name).map(Value::Bool),
            Type::ThemeKey => theme_key(cursor, name),
            Type::String => string(cursor, name).map(Value::String),
            Type::Keywords(names) => keyword(cursor, name, names),
            Type::NumberRange(min, max) => number_in(
                cursor,
                format!("expected number from {min} to {max} for '{name}'"),
                |value| (min..=max).contains(&value),
            ),
            Type::NumberIn(values) => number_in(
                cursor,
                format!("expected one of {} for '{name}'", list(values)),
                |value| values.contains(&value),
            ),
        }
    }

    fn number_value(self, cursor: &mut Cursor, name: &str) -> Result<Value, Error> {
        match self {
            Type::Number => number(cursor, &expected(name, "number")).map(Value::Number),
            Type::Px => px(cursor, name).map(Value::Px),
            Type::SignedPx => signed_px(cursor, name).map(Value::Px),
            Type::Length => length(cursor, name),
            _ => unreachable!("parse sends only number types here"),
        }
    }

    fn paint_value(self, cursor: &mut Cursor, name: &str) -> Result<Value, Error> {
        match self {
            Type::Color => color(cursor, name).map(Value::Color),
            Type::Paint => paint(cursor, name),
            _ => unreachable!("parse sends only paint types here"),
        }
    }
}

fn number_in(
    cursor: &mut Cursor,
    message: String,
    accept: impl Fn(f64) -> bool,
) -> Result<Value, Error> {
    let at = cursor.value_start();
    let value = number(cursor, &message)?;
    if accept(value) {
        Ok(Value::Number(value))
    } else {
        Err(Error::at(cursor.source(), at, &message))
    }
}

fn expected(name: &str, what: &str) -> String {
    format!("expected {what} for '{name}'")
}

fn boolean(cursor: &mut Cursor, name: &str) -> Result<bool, Error> {
    let at = cursor.value_start();
    if cursor.eat_keyword("true") {
        Ok(true)
    } else if cursor.eat_keyword("false") {
        Ok(false)
    } else {
        Err(Error::at(
            cursor.source(),
            at,
            expected(name, "true or false"),
        ))
    }
}

fn number(cursor: &mut Cursor, message: &str) -> Result<f64, Error> {
    let at = cursor.value_start();
    cursor
        .number_literal()
        .map_err(|_| Error::at(cursor.source(), at, message))
}

fn px(cursor: &mut Cursor, name: &str) -> Result<f64, Error> {
    let value = number(cursor, &expected(name, "px"))?;
    if cursor.eat_unit("px") {
        Ok(value)
    } else {
        Err(Error::at(
            cursor.source(),
            cursor.pos(),
            expected(name, "px"),
        ))
    }
}

fn signed_px(cursor: &mut Cursor, name: &str) -> Result<f64, Error> {
    let negative = cursor.eat('-');
    let value = px(cursor, name)?;
    Ok(if negative { -value } else { value })
}

fn length(cursor: &mut Cursor, name: &str) -> Result<Value, Error> {
    let value = number(cursor, &expected(name, "px or %"))?;
    if cursor.eat_unit("px") {
        return Ok(Value::Px(value));
    }
    if cursor.eat('%') {
        return Ok(Value::Percent(value));
    }
    Err(Error::at(
        cursor.source(),
        cursor.pos(),
        expected(name, "px or %"),
    ))
}

fn paint(cursor: &mut Cursor, name: &str) -> Result<Value, Error> {
    cursor.skip_trivia();
    if cursor.eat_keyword("linear-gradient") {
        return gradient(cursor, name).map(Value::Gradient);
    }
    Ok(Value::Color(color(cursor, name)?))
}

fn color(cursor: &mut Cursor, name: &str) -> Result<Color, Error> {
    let at = cursor.value_start();
    let message = expected(name, "color");
    match cursor.peek() {
        Some('#') => hex_color(cursor),
        Some(ch) if ch.is_ascii_alphabetic() => {
            if cursor.eat_keyword("transparent") {
                return Ok(Color::TRANSPARENT);
            }
            if cursor.eat_keyword("white") {
                return Ok(Color::WHITE);
            }
            Err(Error::at(cursor.source(), at, message))
        }
        _ => Err(Error::at(cursor.source(), at, message)),
    }
}

fn hex_color(cursor: &mut Cursor) -> Result<Color, Error> {
    let start = cursor.pos();
    cursor.bump();
    let digits_start = cursor.pos();
    while cursor.peek().is_some_and(|ch| ch.is_ascii_hexdigit()) {
        cursor.bump();
    }
    let digits = &cursor.source()[digits_start..cursor.pos()];
    let mut channels = [0u8; 4];
    match digits.len() {
        3 => {
            for (index, ch) in digits.chars().enumerate() {
                channels[index] = hex(ch) * 17;
            }
            Ok(Color::rgb(channels[0], channels[1], channels[2]))
        }
        6 | 8 => {
            for (index, pair) in digits.as_bytes().chunks(2).enumerate() {
                channels[index] = hex(pair[0] as char) * 16 + hex(pair[1] as char);
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
            cursor.source(),
            start,
            "expected 3, 6 or 8 hex digits",
        )),
    }
}

fn gradient(cursor: &mut Cursor, name: &str) -> Result<Gradient, Error> {
    cursor.expect('(')?;
    let angle = cursor.number_literal()?;
    cursor.eat_keyword("deg");
    cursor.expect(',')?;
    let from = color(cursor, name)?;
    cursor.expect(',')?;
    let to = color(cursor, name)?;
    cursor.expect(')')?;
    Ok(Gradient { angle, from, to })
}

fn string(cursor: &mut Cursor, name: &str) -> Result<String, Error> {
    let at = cursor.value_start();
    if cursor.peek() != Some('"') {
        return Err(Error::at(cursor.source(), at, expected(name, "string")));
    }
    let start = cursor.pos();
    cursor.bump();
    let from = cursor.pos();
    loop {
        match cursor.peek() {
            Some('"') => {
                let value = cursor.source()[from..cursor.pos()].to_owned();
                cursor.bump();
                return Ok(value);
            }
            Some('\n') | None => {
                return Err(Error::at(cursor.source(), start, "unterminated string"));
            }
            Some(_) => cursor.bump(),
        }
    }
}

fn keyword(cursor: &mut Cursor, name: &str, names: &[&str]) -> Result<Value, Error> {
    let at = cursor.value_start();
    let options = names
        .iter()
        .map(|option| format!("'{option}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let message = format!("expected {options} for '{name}'");
    let word = cursor
        .ident()
        .map_err(|_| Error::at(cursor.source(), at, message.as_str()))?;
    if names.contains(&word.as_str()) {
        Ok(Value::Keyword(word))
    } else {
        Err(Error::at(cursor.source(), at, message))
    }
}

fn list(values: &[f64]) -> String {
    values
        .iter()
        .map(|value| format!("{value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn theme_key(cursor: &mut Cursor, name: &str) -> Result<Value, Error> {
    cursor.skip_trivia();
    let at = cursor.pos();
    let message = || format!("expected color or px for theme key '{name}'");
    match cursor.peek() {
        Some('#') => hex_color(cursor).map(Value::Color),
        Some(ch) if ch.is_ascii_alphabetic() => {
            if cursor.eat_keyword("transparent") {
                return Ok(Value::Color(Color::TRANSPARENT));
            }
            if cursor.eat_keyword("white") {
                return Ok(Value::Color(Color::WHITE));
            }
            Err(Error::at(cursor.source(), at, message()))
        }
        Some(ch) if ch.is_ascii_digit() => {
            let value = cursor
                .number_literal()
                .expect("parser accepts only numbers");
            if cursor.eat_unit("px") {
                Ok(Value::Px(value))
            } else {
                Err(Error::at(cursor.source(), at, message()))
            }
        }
        _ => Err(Error::at(cursor.source(), at, message())),
    }
}

fn hex(ch: char) -> u8 {
    ch.to_digit(16).expect("parser accepts only hex digits") as u8
}
