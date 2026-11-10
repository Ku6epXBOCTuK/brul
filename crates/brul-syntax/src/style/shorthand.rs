use crate::cursor::Cursor;
use crate::error::Error;
use crate::style::ast::{Property, Value};
use crate::style::properties::{Kind, Side};
use crate::style::value::{self, Type};

pub struct Shorthand {
    pub name: &'static str,
    pub kinds: &'static [Kind],
    pub parse: fn(&mut Cursor, &str) -> Result<Vec<Value>, Error>,
}

pub static SHORTHANDS: &[Shorthand] = &[Shorthand {
    name: "padding",
    kinds: &[
        Kind::PaddingSide(Side::Top),
        Kind::PaddingSide(Side::Right),
        Kind::PaddingSide(Side::Bottom),
        Kind::PaddingSide(Side::Left),
    ],
    parse: box_values,
}];

pub fn get(name: &str) -> Option<&'static Shorthand> {
    SHORTHANDS.iter().find(|shorthand| shorthand.name == name)
}

impl Shorthand {
    pub fn properties(&self, values: Vec<Value>) -> Vec<Property> {
        self.kinds
            .iter()
            .copied()
            .zip(values)
            .map(|(kind, value)| Property { kind, value })
            .collect()
    }
}

fn box_values(cursor: &mut Cursor, name: &str) -> Result<Vec<Value>, Error> {
    let mut values = vec![value::value(cursor, true, Type::Px, name)?];
    loop {
        cursor.skip_trivia();
        if !cursor.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            break;
        }
        if values.len() == 4 {
            return Err(cursor.error(format!("expected at most 4 values for '{name}'")));
        }
        values.push(value::value(cursor, true, Type::Px, name)?);
    }
    let top = values[0].clone();
    let (right, bottom, left) = match values.len() {
        1 => (top.clone(), top.clone(), top.clone()),
        2 => (values[1].clone(), top.clone(), values[1].clone()),
        3 => (values[1].clone(), values[2].clone(), values[1].clone()),
        _ => (values[1].clone(), values[2].clone(), values[3].clone()),
    };
    Ok(vec![top, right, bottom, left])
}
