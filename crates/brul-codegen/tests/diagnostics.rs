use brul_codegen::sfc_error;
use brul_syntax::sfc;

#[test]
fn syntax_error_keeps_the_message_and_the_position() {
    let error = sfc_error(sfc::Error::Syntax {
        message: String::from("unexpected token"),
        line: 4,
        column: 7,
    });

    assert_eq!(error.message, "unexpected token");
    assert_eq!((error.line, error.column), (4, 7));
}

#[test]
fn duplicate_block_reports_its_name() {
    let error = sfc_error(sfc::Error::Duplicate {
        name: "style",
        line: 9,
        column: 2,
    });

    assert!(error.message.contains("style"));
    assert_eq!((error.line, error.column), (9, 2));
}

#[test]
fn missing_markup_points_at_the_first_line() {
    let error = sfc_error(sfc::Error::MissingMarkup);

    assert!(error.message.contains("markup"));
    assert_eq!((error.line, error.column), (1, 1));
}
