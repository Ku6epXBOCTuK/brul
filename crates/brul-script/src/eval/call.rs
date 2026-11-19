use crate::component::Component;
use crate::error::Error;
use crate::value::Value;

use super::errors::argument_error;

pub fn call(component: &mut Component, name: &str, values: Vec<Value>) -> Result<Value, Error> {
    match name {
        "String" => call_string(values),
        "Number" => call_number(values),
        "len" => call_len(values),
        _ if component.has_function(name) => component.call_function(name, values),
        _ if component.has_dispatch() => match component.call_function(name, values.clone()) {
            Err(Error::UnknownFunction(_)) => component.call(name, &values),
            result => result,
        },
        _ => component.call(name, &values),
    }
}

fn call_string(arguments: Vec<Value>) -> Result<Value, Error> {
    let value = single_argument("String", arguments)?;
    match value {
        Value::String(text) => Ok(Value::String(text)),
        Value::Number(number) => Ok(Value::String(number.to_string().into())),
        Value::Bool(value) => Ok(Value::String(value.to_string().into())),
        Value::Null => Ok(Value::String("null".into())),
        other => Err(argument_error(
            "String",
            "Number, String, Bool or Null",
            &other,
        )),
    }
}

fn call_number(arguments: Vec<Value>) -> Result<Value, Error> {
    let value = single_argument("Number", arguments)?;
    match value {
        Value::Number(number) => Ok(Value::Number(number)),
        Value::String(text) => match text.parse() {
            Ok(number) => Ok(Value::Number(number)),
            Err(_) => Err(Error::InvalidNumber(text.to_string())),
        },
        other => Err(argument_error("Number", "String or Number", &other)),
    }
}

fn call_len(arguments: Vec<Value>) -> Result<Value, Error> {
    let value = single_argument("len", arguments)?;
    match value {
        Value::Array(items) => Ok(Value::Number(items.len() as f64)),
        other => Err(argument_error("len", "Array", &other)),
    }
}

fn single_argument(name: &'static str, mut arguments: Vec<Value>) -> Result<Value, Error> {
    if arguments.len() != 1 {
        return Err(Error::Arity {
            name: name.to_owned(),
            expected: "one",
        });
    }
    Ok(arguments.remove(0))
}
