use brul_syntax::TypeName;
use brul_syntax::ast::Position;

use crate::Value;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("{0}: {1}")]
    At(Position, #[source] Box<Error>),
    #[error("variable '{0}' is not defined")]
    Undefined(String),
    #[error("variable '{0}' is already declared")]
    Redeclared(String),
    #[error("function '{0}' is already declared")]
    FunctionRedeclared(String),
    #[error("function '{}' expects {} arguments, got {}", .name, .expected, .found)]
    FunctionArity {
        name: String,
        expected: usize,
        found: usize,
    },
    #[error("cannot assign to constant '{0}'")]
    Constant(String),
    #[error("operator '{}' is not defined for {} and {}", .operation, .left, .right)]
    Mismatch {
        operation: &'static str,
        left: &'static str,
        right: &'static str,
    },
    #[error("operator '{}' expects {}, got {}", .operation, .expected, .found)]
    Expected {
        operation: &'static str,
        expected: &'static str,
        found: &'static str,
    },
    #[error("division by zero")]
    DivisionByZero,
    #[error("index must be a non-negative integer")]
    IndexNotInteger,
    #[error("index {} is out of bounds for length {}", .index, .length)]
    OutOfBounds { index: usize, length: usize },
    #[error("object has no field '{0}'")]
    MissingField(String),
    #[error("function '{0}' is not defined")]
    UnknownFunction(String),
    #[error("window '{0}' is not declared")]
    UnknownWindow(String),
    #[error("function '{}' expects {} argument", .name, .expected)]
    Arity {
        name: String,
        expected: &'static str,
    },
    #[error("function '{}' expects {}, got {}", .name, .expected, .found)]
    Argument {
        name: &'static str,
        expected: &'static str,
        found: &'static str,
    },
    #[error("event '{}' expects {}, got {}", .name, .expected, .found)]
    EventArgument {
        name: String,
        expected: &'static str,
        found: &'static str,
    },
    #[error("string '{0}' cannot be converted to Number")]
    InvalidNumber(String),
    #[error("expected {}, got {}", .expected, .found)]
    Conversion {
        expected: &'static str,
        found: &'static str,
    },
    #[error("{0} is not supported yet")]
    Unsupported(&'static str),
}

impl Error {
    pub fn conversion(expected: &'static str, found: &Value) -> Self {
        Error::Conversion {
            expected,
            found: found.type_name(),
        }
    }

    pub fn event_argument<E: TypeName + ?Sized, F: TypeName + ?Sized>(
        name: impl Into<String>,
        expected: &E,
        found: &F,
    ) -> Self {
        Error::EventArgument {
            name: name.into(),
            expected: expected.type_name(),
            found: found.type_name(),
        }
    }

    pub fn at_command(self, command: &'static str) -> Self {
        match self {
            Error::Conversion { expected, found } => Error::Argument {
                name: command,
                expected,
                found,
            },
            error => error,
        }
    }
    pub fn at(self, position: Position) -> Self {
        match self {
            Error::At(..) => self,
            error => Error::At(position, Box::new(error)),
        }
    }

    pub fn position(&self) -> Option<Position> {
        match self {
            Error::At(position, _) => Some(*position),
            _ => None,
        }
    }

    pub fn reason(&self) -> String {
        match self {
            Error::At(_, error) => error.reason(),
            error => error.to_string(),
        }
    }
}
