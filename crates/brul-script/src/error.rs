use std::fmt;

use brul_syntax::ast::Position;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    At(Position, Box<Error>),
    Undefined(String),
    Redeclared(String),
    FunctionRedeclared(String),
    FunctionArity {
        name: String,
        expected: usize,
        found: usize,
    },
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
    UnknownFunction(String),
    Reentrant(String),
    Arity {
        name: String,
        expected: &'static str,
    },
    Argument {
        name: &'static str,
        expected: &'static str,
        found: &'static str,
    },
    EventArgument {
        name: String,
        expected: &'static str,
        found: &'static str,
    },
    InvalidNumber(String),
    Unsupported(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::At(position, error) => write!(
                f,
                "line {}, column {}: {error}",
                position.line, position.column
            ),
            Error::Undefined(name) => write!(f, "variable '{name}' is not defined"),
            Error::Redeclared(name) => write!(f, "variable '{name}' is already declared"),
            Error::FunctionRedeclared(name) => write!(f, "function '{name}' is already declared"),
            Error::FunctionArity {
                name,
                expected,
                found,
            } => write!(
                f,
                "function '{name}' expects {expected} arguments, got {found}"
            ),
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
            Error::UnknownFunction(name) => write!(f, "function '{name}' is not defined"),
            Error::Reentrant(name) => {
                write!(
                    f,
                    "event '{name}' cannot run while its component is evaluating"
                )
            }
            Error::Arity { name, expected } => {
                write!(f, "function '{name}' expects {expected} argument")
            }
            Error::Argument {
                name,
                expected,
                found,
            } => write!(f, "function '{name}' expects {expected}, got {found}"),
            Error::EventArgument {
                name,
                expected,
                found,
            } => write!(f, "event '{name}' expects {expected}, got {found}"),
            Error::InvalidNumber(text) => {
                write!(f, "string '{text}' cannot be converted to Number")
            }
            Error::Unsupported(what) => write!(f, "{what} is not supported yet"),
        }
    }
}

impl Error {
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

impl std::error::Error for Error {}
