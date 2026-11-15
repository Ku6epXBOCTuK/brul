use crate::lexer::{self, Span};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("line {}, column {}: {}", .line, .column, .message)]
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

impl From<lexer::Error> for Error {
    fn from(error: lexer::Error) -> Self {
        Self {
            message: error.message,
            line: error.line,
            column: error.column,
        }
    }
}
