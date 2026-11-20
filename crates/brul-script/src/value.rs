use std::hash::{Hash, Hasher};
use std::sync::Arc;

use brul_syntax::TypeName;

use crate::{Error, Items};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(Arc<str>),
    Bool(bool),
    Null,
    Array(Arc<Vec<Value>>),
    Object(Arc<Vec<(String, Value)>>),
    Image(Arc<str>),
    ImageBytes(Arc<[u8]>),
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Number(number) => number.to_bits().hash(state),
            Value::String(text) => text.as_ref().hash(state),
            Value::Bool(flag) => flag.hash(state),
            Value::Null => {}
            Value::Array(items) => items.hash(state),
            Value::Object(entries) => entries.hash(state),
            Value::Image(path) => path.as_ref().hash(state),
            Value::ImageBytes(bytes) => bytes.as_ref().hash(state),
        }
    }
}

impl TypeName for Value {
    fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "Number",
            Value::String(_) => "String",
            Value::Bool(_) => "Bool",
            Value::Null => "Null",
            Value::Array(_) => "Array",
            Value::Object(_) => "Object",
            Value::Image(_) | Value::ImageBytes(_) => "Image",
        }
    }
}

impl Value {
    pub fn image_bytes(bytes: &[u8]) -> Self {
        Self::ImageBytes(Arc::from(bytes))
    }

    fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(number) => Some(*number),
            _ => None,
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
        let number = self.as_number()?;
        if !number.is_finite() {
            return None;
        }
        let pixels = number.round();
        (pixels >= f64::from(i32::MIN) && pixels <= f64::from(i32::MAX)).then_some(pixels as i32)
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

pub fn items(value: &Value) -> Items {
    match value {
        Value::Array(items) => items.iter().cloned().map(crate::Dyn::from).collect(),
        Value::Object(fields) => fields
            .iter()
            .map(|(_, value)| crate::Dyn::from(value.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

impl From<Vec<Value>> for Value {
    fn from(items: Vec<Value>) -> Self {
        Self::Array(items.into())
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
        value
            .as_text()
            .ok_or_else(|| Error::conversion("String", value))
    }
}

impl TryFrom<&Value> for String {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value
            .as_text()
            .map(str::to_owned)
            .ok_or_else(|| Error::conversion("String", value))
    }
}

impl TryFrom<&Value> for f64 {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value
            .as_number()
            .ok_or_else(|| Error::conversion("Number", value))
    }
}

impl TryFrom<&Value> for bool {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value
            .as_flag()
            .ok_or_else(|| Error::conversion("Bool", value))
    }
}

impl TryFrom<&Value> for () {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Null => Ok(()),
            other => Err(Error::conversion("Null", other)),
        }
    }
}

impl TryFrom<&Value> for usize {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value.as_number() {
            Some(number) if number.fract() == 0.0 && number >= 0.0 => Ok(number as Self),
            _ => Err(Error::conversion("a non-negative integer", value)),
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
    use std::hash::DefaultHasher;

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

    #[test]
    fn try_from_uses_the_same_conversions_as_the_accessors() {
        let text = Value::String("a".into());
        assert_eq!(<&str>::try_from(&text), Ok("a"));
        assert_eq!(String::try_from(&text), Ok("a".to_owned()));

        let flag = Value::Bool(true);
        assert_eq!(bool::try_from(&flag), Ok(true));

        let number = Value::Number(2.0);
        assert_eq!(f64::try_from(&number), Ok(2.0));
        assert_eq!(usize::try_from(&number), Ok(2));
        assert_eq!(number.as_pixels(), Some(2));

        for value in [Value::Null, Value::Number(1.5), Value::Bool(true)] {
            assert!(<&str>::try_from(&value).is_err(), "{value:?}");
        }
        for value in [Value::Null, Value::String("1".into()), Value::Number(1.5)] {
            assert!(bool::try_from(&value).is_err(), "{value:?}");
        }
        assert!(usize::try_from(&Value::Number(-1.0)).is_err());
        assert!(usize::try_from(&Value::Number(1.5)).is_err());
    }

    #[test]
    fn hashing_covers_the_variant_and_the_whole_shape() {
        let fingerprint = |value: &Value| {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        };

        assert_eq!(
            fingerprint(&Value::Number(1.0)),
            fingerprint(&Value::Number(1.0))
        );
        assert_ne!(
            fingerprint(&Value::Number(1.0)),
            fingerprint(&Value::String("1".into()))
        );
        assert_ne!(
            fingerprint(&Value::Array(vec![Value::Number(1.0)].into())),
            fingerprint(&Value::Array(vec![Value::String("1".into())].into()))
        );
        assert_ne!(
            fingerprint(&Value::Object(vec![("a".into(), Value::Null)].into())),
            fingerprint(&Value::Object(vec![("b".into(), Value::Null)].into()))
        );
    }
}
