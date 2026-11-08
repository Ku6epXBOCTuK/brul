use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Undefined(String),
    Redeclared(String),
    Constant(String),
    Mismatch {
        operation: &'static str,
        left: &'static str,
        right: &'static str,
    },
    Expected {
        operation: &'static str,
        expected: &'static str,
        found: &'static str,
    },
    DivisionByZero,
    IndexNotInteger,
    OutOfBounds {
        index: usize,
        length: usize,
    },
    MissingField(String),
    Unsupported(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Undefined(name) => write!(f, "variable '{name}' is not defined"),
            Error::Redeclared(name) => write!(f, "variable '{name}' is already declared"),
            Error::Constant(name) => write!(f, "cannot assign to constant '{name}'"),
            Error::Mismatch {
                operation,
                left,
                right,
            } => write!(
                f,
                "operator '{operation}' is not defined for {left} and {right}"
            ),
            Error::Expected {
                operation,
                expected,
                found,
            } => write!(f, "operator '{operation}' expects {expected}, got {found}"),
            Error::DivisionByZero => write!(f, "division by zero"),
            Error::IndexNotInteger => write!(f, "index must be a non-negative integer"),
            Error::OutOfBounds { index, length } => {
                write!(f, "index {index} is out of bounds for length {length}")
            }
            Error::MissingField(name) => write!(f, "object has no field '{name}'"),
            Error::Unsupported(what) => write!(f, "{what} is not supported yet"),
        }
    }
}

impl std::error::Error for Error {}
