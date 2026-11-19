use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Bool,
    Number,
    String,
    Image,
    Null,
    Object,
    Void,
    Array(Box<Type>),
}

impl Type {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "Bool" => Self::Bool,
            "Number" => Self::Number,
            "String" => Self::String,
            "Image" => Self::Image,
            "Null" => Self::Null,
            "Object" => Self::Object,
            "Void" => Self::Void,
            _ => return None,
        })
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Array(item) => write!(f, "PropType::Array(Box::new({item}))"),
            other => write!(f, "PropType::{}", other.name()),
        }
    }
}

impl Type {
    fn name(&self) -> &'static str {
        match self {
            Self::Bool => "Bool",
            Self::Number => "Number",
            Self::String => "String",
            Self::Image => "Image",
            Self::Null => "Null",
            Self::Object => "Object",
            Self::Void => "Void",
            Self::Array(_) => "Array",
        }
    }
}
