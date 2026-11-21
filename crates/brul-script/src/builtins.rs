use brul_syntax::TypeName;

use crate::{Error, Value};

pub fn string(value: impl Into<Value>) -> Result<String, Error> {
    match value.into() {
        Value::String(text) => Ok(text.to_string()),
        Value::Number(number) => Ok(number.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Null => Ok("null".to_owned()),
        other => Err(Error::Argument {
            name: "String",
            expected: "Number, String, Bool or Null",
            found: other.type_name(),
        }),
    }
}

pub fn number(value: impl Into<Value>) -> Result<f64, Error> {
    match value.into() {
        Value::Number(number) => Ok(number),
        Value::String(text) => text
            .parse()
            .map_err(|_| Error::InvalidNumber(text.to_string())),
        other => Err(Error::Argument {
            name: "Number",
            expected: "String or Number",
            found: other.type_name(),
        }),
    }
}

pub fn len(value: impl Into<Value>) -> Result<f64, Error> {
    match value.into() {
        Value::Array(items) => Ok(items.len() as f64),
        other => Err(Error::Argument {
            name: "len",
            expected: "Array",
            found: other.type_name(),
        }),
    }
}

pub fn string_value(mut arguments: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::String(
        string(single_argument("String", &mut arguments)?)?.into(),
    ))
}

pub fn number_value(mut arguments: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Number(number(single_argument(
        "Number",
        &mut arguments,
    )?)?))
}

pub fn len_value(mut arguments: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Number(len(single_argument("len", &mut arguments)?)?))
}

fn single_argument(name: &'static str, arguments: &mut Vec<Value>) -> Result<Value, Error> {
    if arguments.len() != 1 {
        return Err(Error::Arity {
            name: name.to_owned(),
            expected: "one",
        });
    }
    Ok(arguments.remove(0))
}
