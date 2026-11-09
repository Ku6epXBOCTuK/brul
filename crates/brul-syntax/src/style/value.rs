use crate::error::Error;
use crate::style::ast::{Border, BorderColor, Color, Gradient, Value};
use crate::style::cursor::Cursor;

#[derive(Clone, Copy)]
pub enum Type {
    ThemeKey,
    Number,
    Px,
    Length,
    Color,
    Paint,
    Border,
    String,
    Keywords(&'static [&'static str]),
    NumberRange(f64, f64),
    NumberIn(&'static [f64]),
}

impl Type {
    pub fn parse(self, cursor: &mut Cursor, name: &str) -> Result<Value, Error> {
        match self {
            Type::ThemeKey => theme_key(cursor, name),
            Type::Number => number(cursor, &expected(name, "number")).map(Value::Number),
            Type::Px => px(cursor, name).map(Value::Px),
            Type::Length => length(cursor, name),
            Type::Color => color(cursor, name).map(Value::Color),
            Type::Paint => paint(cursor, name),
            Type::Border => border(cursor, name),
            Type::String => string(cursor, name).map(Value::String),
            Type::Keywords(names) => keyword(cursor, name, names),
            Type::NumberRange(min, max) => {
                let at = cursor.value_start();
                let message = format!("expected number from {min} to {max} for '{name}'");
                let value = cursor
                    .number_literal()
                    .map_err(|_| Error::at(cursor.source(), at, message.as_str()))?;
                if (min..=max).contains(&value) {
                    Ok(Value::Number(value))
                } else {
                    Err(Error::at(cursor.source(), at, message))
                }
            }
            Type::NumberIn(values) => {
                let at = cursor.value_start();
                let message = format!("expected one of {} for '{name}'", list(values));
                let value = cursor
                    .number_literal()
                    .map_err(|_| Error::at(cursor.source(), at, message.as_str()))?;
                if values.contains(&value) {
                    Ok(Value::Number(value))
                } else {
                    Err(Error::at(cursor.source(), at, message))
                }
            }
        }
    }
}

fn expected(name: &str, what: &str) -> String {
    format!("expected {what} for '{name}'")
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

fn border(cursor: &mut Cursor, name: &str) -> Result<Value, Error> {
    let width = px(cursor, name)?;
    let color = color_or_theme(cursor, name)?;
    Ok(Value::Border(Border { width, color }))
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

fn color_or_theme(cursor: &mut Cursor, name: &str) -> Result<BorderColor, Error> {
    let at = cursor.value_start();
    match cursor.peek() {
        Some('#') => Ok(BorderColor::Color(hex_color(cursor)?)),
        Some(ch) if ch.is_ascii_alphabetic() => {
            if cursor.eat_keyword("theme") {
                cursor.expect('.')?;
                return Ok(BorderColor::ThemeRef(cursor.ident()?));
            }
            if cursor.eat_keyword("transparent") {
                return Ok(BorderColor::Color(Color::TRANSPARENT));
            }
            if cursor.eat_keyword("white") {
                return Ok(BorderColor::Color(Color::WHITE));
            }
            Err(Error::at(cursor.source(), at, expected(name, "color")))
        }
        _ => Err(Error::at(cursor.source(), at, expected(name, "color"))),
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
