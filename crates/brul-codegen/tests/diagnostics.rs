use brul_codegen::sfc_error;
use brul_syntax::ast::Position;
use brul_syntax::sfc;

#[test]
fn syntax_error_keeps_the_message_and_the_position() {
    let error = sfc_error(sfc::Error::at_position(
        Position { line: 4, column: 7 },
        sfc::Error::custom("unexpected token"),
    ));

    assert_eq!(error.message(), "unexpected token");
    assert_eq!((error.line(), error.column()), (4, 7));
}

#[test]
fn duplicate_block_reports_its_name() {
    let error = sfc_error(sfc::Error::at_position(
        Position { line: 9, column: 2 },
        sfc::Error::DuplicateBlock("style"),
    ));

    assert!(error.message().contains("style"));
    assert_eq!((error.line(), error.column()), (9, 2));
}

#[test]
fn missing_markup_points_at_the_first_line() {
    let error = sfc_error(sfc::Error::at_position(
        Position { line: 1, column: 1 },
        sfc::Error::MissingMarkup,
    ));

    assert!(error.message().contains("markup"));
    assert_eq!((error.line(), error.column()), (1, 1));
}
