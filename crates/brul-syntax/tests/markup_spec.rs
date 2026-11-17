mod common;

use brul_syntax::markup::{Widget, parse_file};

const MARKUP: &str = include_str!("../../../docs/lang/markup.md");

const WIDGETS_HEADING: &str =
    "## \u{412}\u{441}\u{442}\u{440}\u{43e}\u{435}\u{43d}\u{43d}\u{44b}\u{435}";

const EXPRESSION: &str = "\u{432}\u{44b}\u{440}\u{430}\u{436}\u{435}\u{43d}\u{438}\u{435}";

#[test]
fn document_examples_parse() {
    common::assert_document_examples_parse(MARKUP, "brul", parse_file, |example, file| {
        assert!(
            !file.declarations.is_empty(),
            "example at line {} has no declarations",
            example.line
        );
    });
}

#[test]
fn opt_out_and_invalid_markers_are_honored() {
    common::assert_markers_are_honored(
        "brul",
        "this is not markup at all",
        "component { }",
        "component A { }",
        parse_file,
    );
}

#[test]
fn widget_table_matches_the_catalog() {
    let mut listed = Vec::new();
    let mut inside = false;
    for line in MARKUP.lines() {
        if line.starts_with("## ") {
            inside = line.starts_with(WIDGETS_HEADING);
            continue;
        }
        if !inside || !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .collect();
        let [names, children, ..] = cells.as_slice() else {
            continue;
        };
        for name in names.split('`').skip(1).step_by(2) {
            let Some(widget) = Widget::from_name(name) else {
                panic!("markup.md lists widget `{name}`, which the catalog lacks");
            };
            assert_eq!(
                widget.is_leaf(),
                *children == EXPRESSION,
                "markup.md and `Widget::is_leaf` disagree about `{name}`"
            );
            listed.push(widget.name());
        }
    }
    let mut expected: Vec<&str> = Widget::ALL.iter().map(|widget| widget.name()).collect();
    expected.sort_unstable();
    listed.sort_unstable();
    assert_eq!(
        listed, expected,
        "markup.md and the catalog list different widgets"
    );
}

#[test]
fn malformed_examples_are_rejected() {
    let sources = [
        ("component A { Text { View { } } }", "leaf widget"),
        ("component A { View (x) { }", "expected '}'"),
        (
            "component A { Row { @each (items from item) { } } }",
            "expected 'as'",
        ),
        (
            "window Picker (title: \"x\") { Row { Row } }",
            "expected '{'",
        ),
    ];
    for (source, message) in sources {
        let error = parse_file(source).unwrap_err();
        assert!(
            error.message.contains(message),
            "{source}: expected message containing {message}, got {}",
            error.message
        );
    }
}
