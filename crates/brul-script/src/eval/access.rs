use std::sync::Arc;

use crate::error::Error;
use crate::value::Value;

use super::errors::expected;

pub fn read_field(target: &Value, name: &str) -> Result<Value, Error> {
    match target {
        Value::Object(object) => object
            .get(name)
            .cloned()
            .ok_or_else(|| Error::MissingField(name.to_owned())),
        other => Err(expected(".", "Object", other)),
    }
}

pub fn assign_field(target: &mut Value, name: &str, value: Value) -> Result<(), Error> {
    match target {
        Value::Object(object) => {
            let field = object
                .get_mut(name)
                .ok_or_else(|| Error::MissingField(name.to_owned()))?;
            *field = value;
            Ok(())
        }
        other => Err(expected(".", "Object", other)),
    }
}

pub fn read_index(target: &Value, index: &Value) -> Result<Value, Error> {
    match target {
        Value::Array(items) => {
            let Value::Number(_) = index else {
                return Err(expected("[]", "Number", index));
            };
            let index = array_index(items.len(), index)?;
            Ok(items[index].clone())
        }
        Value::Object(object) => {
            let Value::String(key) = index else {
                return Err(expected("[]", "String", index));
            };
            object
                .get(key.as_ref())
                .cloned()
                .ok_or_else(|| Error::MissingField(key.to_string()))
        }
        other => Err(expected("[]", "Array or Object", other)),
    }
}

pub fn assign_index(target: &mut Value, index: &Value, value: Value) -> Result<(), Error> {
    match target {
        Value::Array(items) => {
            let Value::Number(_) = index else {
                return Err(expected("[]", "Number", index));
            };
            let index = array_index(items.len(), index)?;
            Arc::make_mut(items)[index] = value;
            Ok(())
        }
        Value::Object(object) => {
            let Value::String(key) = index else {
                return Err(expected("[]", "String", index));
            };
            let field = object
                .get_mut(key.as_ref())
                .ok_or_else(|| Error::MissingField(key.to_string()))?;
            *field = value;
            Ok(())
        }
        other => Err(expected("[]", "Array or Object", other)),
    }
}

fn array_index(length: usize, index: &Value) -> Result<usize, Error> {
    let Value::Number(index) = index else {
        return Err(Error::IndexNotInteger);
    };
    if !index.is_finite() || *index < 0.0 || index.fract() != 0.0 {
        return Err(Error::IndexNotInteger);
    }
    let index = *index as usize;
    if index >= length {
        return Err(Error::OutOfBounds { index, length });
    }
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Object;

    #[test]
    fn fields_and_indexes_are_read_and_written() {
        assert_eq!(
            read_field(
                &Value::Object(Object::builder().field("a", Value::Number(1.0)).into()),
                "a"
            ),
            Ok(Value::Number(1.0))
        );
        assert_eq!(
            read_index(
                &Value::Array(vec![Value::Bool(true)].into()),
                &Value::Number(0.0)
            ),
            Ok(Value::Bool(true))
        );
        let mut object = Value::Object(Object::builder().field("a", Value::Null).into());
        assert_eq!(assign_field(&mut object, "a", Value::Bool(true)), Ok(()));
        assert_eq!(read_field(&object, "a"), Ok(Value::Bool(true)));
    }
}
