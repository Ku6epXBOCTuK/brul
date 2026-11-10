#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
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
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::String(content) => Some(content),
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
    fn only_the_matching_variant_converts() {
        assert_eq!(Value::String("a".to_owned()).as_text(), Some("a"));
        assert_eq!(Value::Bool(true).as_flag(), Some(true));
        assert_eq!(Value::Number(1.0).as_pixels(), Some(1));
        for value in [
            Value::Null,
            Value::Number(1.0),
            Value::Array(Vec::new()),
            Value::Object(Vec::new()),
        ] {
            assert_eq!(value.as_text(), None, "{value:?}");
            assert_eq!(value.as_flag(), None, "{value:?}");
        }
        for value in [
            Value::Null,
            Value::String("1".to_owned()),
            Value::Bool(true),
            Value::Array(Vec::new()),
            Value::Object(Vec::new()),
        ] {
            assert_eq!(value.as_pixels(), None, "{value:?}");
        }
    }
}
