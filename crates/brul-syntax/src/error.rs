use std::fmt;

use crate::lexer::{self, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl Error {
    pub fn at(source: &str, pos: usize, message: impl Into<String>) -> Self {
        let (line, column) = Span {
            start: pos,
            end: pos,
        }
        .line_col(source);
        Self {
            message: message.into(),
            line,
            column,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for Error {}

impl From<lexer::Error> for Error {
    fn from(error: lexer::Error) -> Self {
        Self {
            message: error.message,
            line: error.line,
            column: error.column,
        }
    }
}
