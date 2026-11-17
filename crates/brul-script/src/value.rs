use std::sync::Arc;

use crate::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(Arc<str>),
    Bool(bool),
    Null,
    Array(Arc<Vec<Value>>),
    Object(Arc<Vec<(String, Value)>>),
    Image(Arc<str>),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "Number",
            Value::String(_) => "String",
            Value::Bool(_) => "Bool",
            Value::Null => "Null",
            Value::Array(_) => "Array",
            Value::Object(_) => "Object",
            Value::Image(_) => "Image",
        }
    }

    pub fn as_image(&self) -> Option<&str> {
        match self {
            Value::Image(path) => Some(path.as_ref()),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::String(content) => Some(content.as_ref()),
            _ => None,
        }
    }

    pub fn as_flag(&self) -> Option<bool> {
        match self {
            Value::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_pixels(&self) -> Option<i32> {
        match self {
            Value::Number(value) if value.is_finite() => {
                let pixels = value.round();
                (pixels >= f64::from(i32::MIN) && pixels <= f64::from(i32::MAX))
                    .then_some(pixels as i32)
            }
            _ => None,
        }
    }
}

impl From<&str> for Value {
    fn from(text: &str) -> Self {
        Self::String(Arc::from(text))
    }
}

impl From<String> for Value {
    fn from(text: String) -> Self {
        Self::String(Arc::from(text))
    }
}

impl From<f64> for Value {
    fn from(number: f64) -> Self {
        Self::Number(number)
    }
}

impl From<bool> for Value {
    fn from(flag: bool) -> Self {
        Self::Bool(flag)
    }
}

impl<'a> TryFrom<&'a Value> for &'a str {
    type Error = Error;

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(text) => Ok(text),
            other => Err(Error::conversion("String", other)),
        }
    }
}

impl TryFrom<&Value> for String {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(text) => Ok(text.to_string()),
            other => Err(Error::conversion("String", other)),
        }
    }
}

impl TryFrom<&Value> for f64 {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(number) => Ok(*number),
            other => Err(Error::conversion("Number", other)),
        }
    }
}

impl TryFrom<&Value> for bool {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bool(flag) => Ok(*flag),
            other => Err(Error::conversion("Bool", other)),
        }
    }
}

impl TryFrom<&Value> for usize {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(number) if number.fract() == 0.0 && *number >= 0.0 => Ok(*number as Self),
            other => Err(Error::conversion("a non-negative integer", other)),
        }
    }
}

pub struct Fields<'a> {
    fields: &'a [(String, Value)],
}

impl<'a> Fields<'a> {
    pub fn new(value: &'a Value) -> Result<Self, Error> {
        match value {
            Value::Object(fields) => Ok(Self { fields }),
            other => Err(Error::conversion("Object", other)),
        }
    }

    pub fn take<T: TryFrom<&'a Value, Error = Error>>(&mut self, name: &str) -> Result<T, Error> {
        let fields = self.fields;
        let value = fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
            .ok_or_else(|| Error::MissingField(name.to_owned()))?;
        T::try_from(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_pixels_rounds_to_the_nearest_pixel() {
        for value in [0.0, -3.0, 4.0, f64::from(i32::MIN), f64::from(i32::MAX)] {
            assert_eq!(Value::Number(value).as_pixels(), Some(value as i32));
        }
        for (value, pixels) in [
            (1.5, 2),
            (-1.5, -2),
            (2.4999, 2),
            (0.1 + 0.2, 0),
            (99.9999999, 100),
        ] {
            assert_eq!(Value::Number(value).as_pixels(), Some(pixels), "{value}");
        }
        for value in [
            f64::NAN,
            f64::INFINITY,
            f64::from(i32::MIN) - 1.0,
            f64::from(i32::MAX) + 1.0,
        ] {
            assert_eq!(Value::Number(value).as_pixels(), None, "{value}");
        }
    }

    #[test]
    fn a_scalar_becomes_a_value() {
        assert_eq!(Value::from("a"), Value::String("a".into()));
        assert_eq!(Value::from("a".to_owned()), Value::String("a".into()));
        assert_eq!(Value::from(1.5), Value::Number(1.5));
        assert_eq!(Value::from(true), Value::Bool(true));
    }

    #[test]
    fn only_the_matching_variant_converts() {
        assert_eq!(Value::String("a".into()).as_text(), Some("a"));
        assert_eq!(Value::Bool(true).as_flag(), Some(true));
        assert_eq!(Value::Number(1.0).as_pixels(), Some(1));
        assert_eq!(Value::Image("a.png".into()).as_image(), Some("a.png"));
        for value in [
            Value::Null,
            Value::Number(1.0),
            Value::Array(Vec::new().into()),
            Value::Object(Vec::new().into()),
            Value::Image("a.png".into()),
        ] {
            assert_eq!(value.as_text(), None, "{value:?}");
            assert_eq!(value.as_flag(), None, "{value:?}");
        }
        for value in [
            Value::Null,
            Value::String("1".into()),
            Value::Bool(true),
            Value::Array(Vec::new().into()),
            Value::Object(Vec::new().into()),
            Value::Image("a.png".into()),
        ] {
            assert_eq!(value.as_pixels(), None, "{value:?}");
        }
    }
}
