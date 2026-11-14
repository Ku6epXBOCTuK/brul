use crate::error::Error;
use crate::value::Value;

use super::errors::expected;

pub fn template(values: Vec<Value>) -> Result<String, Error> {
    let mut text = String::new();
    for value in values {
        match value {
            Value::String(chunk) => text.push_str(chunk.as_ref()),
            Value::Number(number) => text.push_str(&number.to_string()),
            other => return Err(expected("${}", "String or Number", &other)),
        }
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_values_are_concatenated() {
        assert_eq!(
            template(vec![Value::String("a".into()), Value::Number(2.0)]),
            Ok("a2".to_owned())
        );
    }
}
