use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::sfc;

pub fn padded(source: &str, block: sfc::Block<'_>) -> String {
    let mut text = String::with_capacity(block.offset + block.content.len());
    for character in source[..block.offset].chars() {
        text.push(if character == '\n' { '\n' } else { ' ' });
    }
    text.push_str(block.content);
    text
}

pub fn sfc_error(error: sfc::Error) -> Error {
    match error {
        sfc::Error::Syntax {
            message,
            line,
            column,
        } => Error {
            message,
            line,
            column,
        },
        sfc::Error::Duplicate { name, line, column } => Error {
            message: format!("duplicate <{name}> block"),
            line,
            column,
        },
        sfc::Error::MissingMarkup => {
            error_at(Position { line: 1, column: 1 }, "missing <markup> block")
        }
    }
}

pub fn error_at(position: Position, message: impl Into<String>) -> Error {
    Error {
        message: message.into(),
        line: position.line,
        column: position.column,
    }
}
