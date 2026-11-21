use brul_script::{Error, TypeName, Value};

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

include!(concat!(env!("OUT_DIR"), "/lang_builtin_markers.rs"));
