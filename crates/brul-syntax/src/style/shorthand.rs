use brul_style::{Kind, Property, Side, Type, Value};

use crate::cursor::Cursor;
use crate::error::Error;
use crate::style::value;

pub struct Shorthand {
    pub name: &'static str,
    pub kinds: &'static [Kind],
    pub parse: fn(&mut Cursor, &str) -> Result<Vec<Value>, Error>,
}

include!(concat!(env!("OUT_DIR"), "/style_shorthands.rs"));

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

fn border_values(cursor: &mut Cursor, name: &str) -> Result<Vec<Value>, Error> {
    let width = value::value(cursor, true, Type::Px, name)?;
    let color = value::value(cursor, true, Type::Color, name)?;
    let mut values = expand_four(vec![width]);
    values.extend(expand_four(vec![color]));
    Ok(values)
}

fn color_values(cursor: &mut Cursor, name: &str) -> Result<Vec<Value>, Error> {
    let mut values = vec![value::value(cursor, true, Type::Color, name)?];
    loop {
        cursor.skip_trivia();
        if matches!(cursor.peek(), None | Some(';') | Some('}')) {
            break;
        }
        if values.len() == 4 {
            return Err(cursor.error(Error::AtMost {
                max: 4,
                name: name.to_owned(),
            }));
        }
        values.push(value::value(cursor, true, Type::Color, name)?);
    }
    Ok(expand_four(values))
}

fn expand_four(values: Vec<Value>) -> Vec<Value> {
    let top = values[0].clone();
    let (right, bottom, left) = match values.len() {
        1 => (top.clone(), top.clone(), top.clone()),
        2 => (values[1].clone(), top.clone(), values[1].clone()),
        3 => (values[1].clone(), values[2].clone(), values[1].clone()),
        _ => (values[1].clone(), values[2].clone(), values[3].clone()),
    };
    vec![top, right, bottom, left]
}

fn shadow_values(cursor: &mut Cursor, name: &str) -> Result<Vec<Value>, Error> {
    let offset_x = value::value(cursor, true, Type::SignedPx, name)?;
    let offset_y = value::value(cursor, true, Type::SignedPx, name)?;
    let blur = value::value(cursor, true, Type::Px, name)?;
    let color = value::value(cursor, true, Type::Color, name)?;
    Ok(vec![offset_x, offset_y, blur, color])
}

fn box_values(cursor: &mut Cursor, name: &str) -> Result<Vec<Value>, Error> {
    let mut values = vec![value::value(cursor, true, Type::Px, name)?];
    loop {
        cursor.skip_trivia();
        if !cursor.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            break;
        }
        if values.len() == 4 {
            return Err(cursor.error(Error::AtMost {
                max: 4,
                name: name.to_owned(),
            }));
        }
        values.push(value::value(cursor, true, Type::Px, name)?);
    }
    Ok(expand_four(values))
}
