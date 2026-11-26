use brul_syntax::TypeName;
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
    error
}

pub fn error_at(position: Position, message: impl Into<String>) -> Error {
    Error::at_position(position, Error::custom(message))
}

pub fn arg_mismatch<E: TypeName + ?Sized, F: TypeName + ?Sized>(
    position: Position,
    expected: &E,
    found: &F,
) -> Error {
    Error::at_position(
        position,
        Error::custom(format!(
            "argument expects {}, found {}",
            expected.type_name(),
            found.type_name()
        )),
    )
}
