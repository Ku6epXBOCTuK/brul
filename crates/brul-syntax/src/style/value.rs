#[path = "value/color.rs"]
mod color;
use brul_style::{
    AreaTemplate, Color, ColorExpr, ColorValue, Gradient, GridLine, LinePlacement, RepeatCount,
    TrackSize, Type, Value,
};
use color::{color, grid_area, keyword, list, paint, string, theme_key};

use crate::cursor::Cursor;
use crate::error::{Error, Unterminated};

const NESTING: usize = 8;

pub fn value(cursor: &mut Cursor, theme: bool, ty: Type, name: &str) -> Result<Value, Error> {
    if theme && cursor.eat_keyword("theme") {
        cursor.expect('.')?;
        return Ok(Value::ThemeRef(cursor.ident()?));
    }
    parse(cursor, theme, ty, name)
}

fn parse(cursor: &mut Cursor, theme: bool, ty: Type, name: &str) -> Result<Value, Error> {
    match ty {
        Type::Number | Type::Px | Type::SignedPx | Type::Length => number_value(ty, cursor, name),
        Type::Color | Type::Paint => paint_value(ty, cursor, theme, name),
        Type::Bool => boolean(cursor, name).map(Value::Bool),
        Type::ThemeKey => theme_key(cursor, name),
        Type::String => string(cursor, name).map(Value::String),
        Type::Keywords(names) => keyword(cursor, name, names),
        Type::TrackList => track_list(cursor, name),
        Type::GridLine => grid_line(cursor, name),
        Type::AreaTemplate => area_template(cursor, name),
        Type::GridArea => grid_area(cursor, name),
        Type::NumberRange(min, max) => number_in(
            cursor,
            Error::NumberRange {
                min,
                max,
                name: name.to_owned(),
            },
            |value| (min..=max).contains(&value),
        ),
        Type::NumberIn(values) => number_in(
            cursor,
            Error::OneOf {
                options: format!("one of {}", list(values)),
                name: name.to_owned(),
            },
            |value| values.contains(&value),
        ),
    }
}

fn number_value(ty: Type, cursor: &mut Cursor, name: &str) -> Result<Value, Error> {
    match ty {
        Type::Number => number(cursor, &expected(name, "number")).map(Value::Number),
        Type::Px => px(cursor, name).map(Value::Px),
        Type::SignedPx => signed_px(cursor, name).map(Value::Px),
        Type::Length => length(cursor, name),
        _ => unreachable!("parse sends only number types here"),
    }
}

fn paint_value(ty: Type, cursor: &mut Cursor, theme: bool, name: &str) -> Result<Value, Error> {
    match ty {
        Type::Color => color(cursor, theme, name),
        Type::Paint => paint(cursor, theme, name),
        _ => unreachable!("parse sends only paint types here"),
    }
}

fn number_in(
    cursor: &mut Cursor,
    reason: Error,
    accept: impl Fn(f64) -> bool,
) -> Result<Value, Error> {
    let at = cursor.value_start();
    let value = number(cursor, &reason)?;
    if accept(value) {
        Ok(Value::Number(value))
    } else {
        Err(Error::at(cursor.source(), at, reason))
    }
}

fn expected(name: &str, what: &'static str) -> Error {
    Error::ExpectedFor {
        expected: what,
        name: name.to_owned(),
    }
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

fn number(cursor: &mut Cursor, reason: &Error) -> Result<f64, Error> {
    let at = cursor.value_start();
    cursor
        .number_literal()
        .map_err(|_| Error::at(cursor.source(), at, reason.clone()))
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

pub fn duration(cursor: &mut Cursor, name: &str) -> Result<f64, Error> {
    let reason = Error::ExpectedFor {
        expected: "ms",
        name: format!("animate {name}"),
    };
    let value = number(cursor, &reason)?;
    if cursor.eat_unit("ms") {
        Ok(value)
    } else {
        Err(Error::at(cursor.source(), cursor.pos(), reason))
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

fn track_list(cursor: &mut Cursor, name: &str) -> Result<Value, Error> {
    let mut tracks = vec![track_size(cursor, name, 0)?];
    while starts_track_size(cursor) {
        tracks.push(track_size(cursor, name, 0)?);
    }
    Ok(Value::TrackList(tracks))
}

fn starts_track_size(cursor: &mut Cursor) -> bool {
    cursor.skip_trivia();
    cursor.peek().is_some_and(|ch| ch.is_ascii_digit())
        || cursor.starts_keyword("minmax")
        || cursor.starts_keyword("repeat")
}

fn track_size(cursor: &mut Cursor, name: &str, depth: usize) -> Result<TrackSize, Error> {
    let at = cursor.value_start();
    if depth > NESTING {
        return Err(Error::at(
            cursor.source(),
            at,
            Error::NestingDepth {
                what: "grid track",
                max: NESTING,
            },
        ));
    }
    if cursor.eat_keyword("repeat") {
        cursor.expect('(')?;
        let count = repeat_count(cursor, name)?;
        cursor.expect(',')?;
        let mut tracks = vec![track_size(cursor, name, depth + 1)?];
        while starts_track_size(cursor) {
            tracks.push(track_size(cursor, name, depth + 1)?);
        }
        cursor.expect(')')?;
        return Ok(TrackSize::Repeat(count, tracks));
    }
    if cursor.eat_keyword("minmax") {
        cursor.expect('(')?;
        let min = track_size(cursor, name, depth + 1)?;
        cursor.expect(',')?;
        let max = track_size(cursor, name, depth + 1)?;
        cursor.expect(')')?;
        return Ok(TrackSize::Minmax(Box::new(min), Box::new(max)));
    }
    let value = number(cursor, &expected(name, "a track size"))?;
    if cursor.eat_unit("fr") {
        return Ok(TrackSize::Fr(value));
    }
    if cursor.eat_unit("px") {
        return Ok(TrackSize::Px(value));
    }
    if cursor.eat('%') {
        return Ok(TrackSize::Percent(value));
    }
    Err(Error::at(
        cursor.source(),
        at,
        expected(name, "fr, px or %"),
    ))
}

fn repeat_count(cursor: &mut Cursor, name: &str) -> Result<RepeatCount, Error> {
    if cursor.eat_keyword("auto-fill") {
        return Ok(RepeatCount::AutoFill);
    }
    if cursor.eat_keyword("auto-fit") {
        return Ok(RepeatCount::AutoFit);
    }
    let count = grid_integer(cursor, name, 1.0, u16::MAX as f64)?;
    Ok(RepeatCount::Count(count as u16))
}

fn grid_line(cursor: &mut Cursor, name: &str) -> Result<Value, Error> {
    let start = line_placement(cursor, name)?;
    let end = if cursor.eat('/') {
        Some(line_placement(cursor, name)?)
    } else {
        None
    };
    Ok(Value::GridLine(GridLine { start, end }))
}

fn line_placement(cursor: &mut Cursor, name: &str) -> Result<LinePlacement, Error> {
    let at = cursor.value_start();
    if cursor.eat_keyword("auto") {
        return Ok(LinePlacement::Auto);
    }
    if cursor.eat_keyword("span") {
        let count = grid_integer(cursor, name, 0.0, u16::MAX as f64)?;
        return Ok(LinePlacement::Span(count as u16));
    }
    let negative = cursor.eat('-');
    if !cursor.peek().is_some_and(|ch| ch.is_ascii_digit()) {
        return Err(Error::at(
            cursor.source(),
            at,
            expected(name, "a line placement"),
        ));
    }
    let value = grid_integer(cursor, name, 0.0, i32::MAX as f64)? as i32;
    Ok(LinePlacement::Line(if negative { -value } else { value }))
}

fn grid_integer(cursor: &mut Cursor, name: &str, min: f64, max: f64) -> Result<f64, Error> {
    let at = cursor.value_start();
    let reason = Error::NumberRange {
        min,
        max,
        name: name.to_owned(),
    };
    let value = cursor
        .number_literal()
        .map_err(|_| Error::at(cursor.source(), at, reason.clone()))?;
    if value.fract() == 0.0 && (min..=max).contains(&value) {
        Ok(value)
    } else {
        Err(Error::at(cursor.source(), at, reason))
    }
}

fn area_template(cursor: &mut Cursor, name: &str) -> Result<Value, Error> {
    let at = cursor.value_start();
    let mut rows = Vec::new();
    loop {
        cursor.skip_trivia();
        if cursor.peek() != Some('"') {
            break;
        }
        rows.push(string(cursor, name)?);
    }
    AreaTemplate::parse(&rows)
        .map(Value::AreaTemplate)
        .map_err(|_| Error::at(cursor.source(), at, expected(name, "an area template")))
}
