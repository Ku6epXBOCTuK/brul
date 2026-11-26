use super::*;
pub(super) fn grid_area(cursor: &mut Cursor, name: &str) -> Result<Value, Error> {
    let at = cursor.value_start();
    let area = cursor
        .ident()
        .map_err(|_| Error::at(cursor.source(), at, expected(name, "a grid area")))?;
    let area = if area == "auto" || area == "none" {
        String::new()
    } else {
        area
    };
    Ok(Value::GridArea(area))
}

pub(super) fn paint(cursor: &mut Cursor, theme: bool, name: &str) -> Result<Value, Error> {
    cursor.skip_trivia();
    if cursor.eat_keyword("linear-gradient") {
        return gradient(cursor, theme, name).map(Value::Gradient);
    }
    color(cursor, theme, name)
}

pub(super) fn color(cursor: &mut Cursor, theme: bool, name: &str) -> Result<Value, Error> {
    Ok(match color_value(cursor, theme, name, 0)? {
        ColorValue::Color(color) => Value::Color(color),
        ColorValue::Theme(key) => Value::ThemeRef(key),
        ColorValue::Expr(expr) => Value::ColorExpr(*expr),
    })
}

fn color_value(
    cursor: &mut Cursor,
    theme: bool,
    name: &str,
    depth: usize,
) -> Result<ColorValue, Error> {
    let at = cursor.value_start();
    if depth > NESTING {
        return Err(Error::at(
            cursor.source(),
            at,
            Error::NestingDepth {
                what: "color expression",
                max: NESTING,
            },
        ));
    }
    if cursor.eat_keyword("alpha") {
        let color = first_argument(cursor, theme, name, depth)?;
        let amount = percent(cursor, name)?;
        cursor.expect(')')?;
        return Ok(expr(ColorExpr::Alpha { color, amount }));
    }
    if cursor.eat_keyword("lighten") {
        let color = first_argument(cursor, theme, name, depth)?;
        let amount = percent(cursor, name)?;
        cursor.expect(')')?;
        return Ok(expr(ColorExpr::Lighten { color, amount }));
    }
    if cursor.eat_keyword("darken") {
        let color = first_argument(cursor, theme, name, depth)?;
        let amount = percent(cursor, name)?;
        cursor.expect(')')?;
        return Ok(expr(ColorExpr::Darken { color, amount }));
    }
    if cursor.eat_keyword("mix") {
        let from = first_argument(cursor, theme, name, depth)?;
        let to = color_value(cursor, theme, name, depth + 1)?;
        cursor.expect(',')?;
        let amount = percent(cursor, name)?;
        cursor.expect(')')?;
        return Ok(expr(ColorExpr::Mix { from, to, amount }));
    }
    if theme && cursor.eat_keyword("theme") {
        cursor.expect('.')?;
        return Ok(ColorValue::Theme(cursor.ident()?));
    }
    literal(cursor, name, at).map(ColorValue::Color)
}

fn first_argument(
    cursor: &mut Cursor,
    theme: bool,
    name: &str,
    depth: usize,
) -> Result<ColorValue, Error> {
    cursor.expect('(')?;
    let color = color_value(cursor, theme, name, depth + 1)?;
    cursor.expect(',')?;
    Ok(color)
}

fn expr(expr: ColorExpr) -> ColorValue {
    ColorValue::Expr(Box::new(expr))
}

fn percent(cursor: &mut Cursor, name: &str) -> Result<f64, Error> {
    let at = cursor.value_start();
    let reason = Error::PercentRange(name.to_owned());
    let value = cursor
        .number_literal()
        .map_err(|_| Error::at(cursor.source(), at, reason.clone()))?;
    if cursor.eat_unit("%") && (0.0..=100.0).contains(&value) {
        Ok(value)
    } else {
        Err(Error::at(cursor.source(), at, reason))
    }
}

fn literal(cursor: &mut Cursor, name: &str, at: usize) -> Result<Color, Error> {
    let reason = expected(name, "color");
    match cursor.peek() {
        Some('#') => hex_color(cursor),
        Some(ch) if ch.is_ascii_alphabetic() => {
            if cursor.eat_keyword("transparent") {
                return Ok(Color::TRANSPARENT);
            }
            if cursor.eat_keyword("white") {
                return Ok(Color::WHITE);
            }
            Err(Error::at(cursor.source(), at, reason))
        }
        _ => Err(Error::at(cursor.source(), at, reason)),
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
        _ => Err(Error::at(cursor.source(), start, Error::HexDigits)),
    }
}

fn gradient(cursor: &mut Cursor, theme: bool, name: &str) -> Result<Gradient, Error> {
    cursor.expect('(')?;
    let angle = cursor.number_literal()?;
    cursor.eat_keyword("deg");
    cursor.expect(',')?;
    let from = color_value(cursor, theme, name, 0)?;
    cursor.expect(',')?;
    let to = color_value(cursor, theme, name, 0)?;
    cursor.expect(')')?;
    Ok(Gradient { angle, from, to })
}

pub(super) fn string(cursor: &mut Cursor, name: &str) -> Result<String, Error> {
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
                return Err(Error::at(
                    cursor.source(),
                    start,
                    Error::Unterminated(Unterminated::String),
                ));
            }
            Some(_) => cursor.bump(),
        }
    }
}

pub(super) fn keyword(cursor: &mut Cursor, name: &str, names: &[&str]) -> Result<Value, Error> {
    let at = cursor.value_start();
    let options = names
        .iter()
        .map(|option| format!("'{option}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let reason = Error::OneOf {
        options,
        name: name.to_owned(),
    };
    let word = cursor
        .ident()
        .map_err(|_| Error::at(cursor.source(), at, reason.clone()))?;
    if names.contains(&word.as_str()) {
        Ok(Value::Keyword(word))
    } else {
        Err(Error::at(cursor.source(), at, reason))
    }
}

pub(super) fn list(values: &[f64]) -> String {
    values
        .iter()
        .map(|value| format!("{value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn theme_key(cursor: &mut Cursor, name: &str) -> Result<Value, Error> {
    cursor.skip_trivia();
    let at = cursor.pos();
    if starts_color(cursor) {
        return color(cursor, true, name);
    }
    if cursor.eat_keyword("theme") {
        return Err(Error::at(
            cursor.source(),
            at,
            Error::ThemeAlias(name.to_owned()),
        ));
    }
    if cursor.peek().is_some_and(|ch| ch.is_ascii_digit()) {
        let value = cursor
            .number_literal()
            .expect("parser accepts only numbers");
        if cursor.eat_unit("px") {
            return Ok(Value::Px(value));
        }
    }
    Err(Error::at(
        cursor.source(),
        at,
        Error::ExpectedThemeKey(name.to_owned()),
    ))
}

pub(super) fn starts_color(cursor: &mut Cursor) -> bool {
    if cursor.peek() == Some('#') {
        return true;
    }
    let start = cursor.pos();
    let found = ["alpha", "mix", "lighten", "darken", "transparent", "white"]
        .iter()
        .any(|word| cursor.eat_keyword(word));
    cursor.set_pos(start);
    found
}

pub(super) fn hex(ch: char) -> u8 {
    ch.to_digit(16).expect("parser accepts only hex digits") as u8
}
