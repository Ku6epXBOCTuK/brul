mod common;

use brul_syntax::style::parse_file;

const STYLE: &str = include_str!("../../../docs/lang/style.md");

#[test]
fn document_examples_parse() {
    common::assert_document_examples_parse(STYLE, "css", parse_file, |example, sheet| {
        assert!(
            !sheet.rules.is_empty() || !sheet.themes.is_empty(),
            "example at line {} is empty",
            example.line
        );
    });
}

#[test]
fn opt_out_and_invalid_markers_are_honored() {
    common::assert_markers_are_honored(
        "css",
        "this is not style at all",
        ".a { width 4px; }",
        ".a { width: 4px; }",
        parse_file,
    );
}

#[test]
fn malformed_examples_are_rejected() {
    let sources = [
        (".a { width 4px; }", "expected ':'"),
        (".a { width: 4em; }", "expected px or % for 'width'"),
        (".a:hovered { }", "unknown state"),
        (".a { widht: 4px; }", "unknown property 'widht'"),
        (
            ".a { opacity: 2; }",
            "expected number from 0 to 1 for 'opacity'",
        ),
    ];
    for (source, message) in sources {
        let error = parse_file(source).unwrap_err();
        assert!(
            error.message().contains(message),
            "{source}: expected message containing {message}, got {}",
            error.message()
        );
    }
}
