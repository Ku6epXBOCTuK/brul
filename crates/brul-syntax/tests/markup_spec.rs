use brul_syntax::markup::{Widget, parse_file};

const MARKUP: &str = include_str!("../../../docs/lang/markup.md");

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Parse,
    Ignore,
    Invalid,
    Other,
}

struct Example {
    kind: Kind,
    line: usize,
    body: String,
}

const EXAMPLES_HEADING: &str =
    "## \u{422}\u{435}\u{441}\u{442}-\u{43f}\u{440}\u{438}\u{43c}\u{435}\u{440}\u{44b}";

const WIDGETS_HEADING: &str =
    "## \u{412}\u{441}\u{442}\u{440}\u{43e}\u{435}\u{43d}\u{43d}\u{44b}\u{435}";

const EXPRESSION: &str = "\u{432}\u{44b}\u{440}\u{430}\u{436}\u{435}\u{43d}\u{438}\u{435}";

fn examples(document: &str) -> Vec<Example> {
    let mut out = Vec::new();
    let mut in_examples = false;
    let mut kind = Kind::Other;
    let mut line = 0;
    let mut body = String::new();
    for (index, text) in document.lines().enumerate() {
        let number = index + 1;
        if !in_examples {
            if text.starts_with(EXAMPLES_HEADING) {
                in_examples = true;
            }
            continue;
        }
        if text.starts_with("## ") {
            break;
        }
        if kind == Kind::Other {
            if let Some(info) = text.strip_prefix("```") {
                kind = match info.trim() {
                    "brul" => Kind::Parse,
                    "brul,ignore" => Kind::Ignore,
                    "brul,invalid" => Kind::Invalid,
                    _ => Kind::Other,
                };
                line = number;
                body.clear();
            }
            continue;
        }
        if text.starts_with("```") {
            out.push(Example {
                kind,
                line,
                body: body.clone(),
            });
            kind = Kind::Other;
            continue;
        }
        body.push_str(text);
        body.push('\n');
    }
    out
}

#[test]
fn document_examples_parse() {
    let examples = examples(MARKUP);
    let parsed: Vec<&Example> = examples
        .iter()
        .filter(|example| example.kind == Kind::Parse)
        .collect();
    assert!(
        !parsed.is_empty(),
        "the markup spec has no parseable example"
    );
    for example in parsed {
        let file = parse_file(&example.body)
            .unwrap_or_else(|error| panic!("example at line {}: {error}", example.line));
        assert!(
            !file.declarations.is_empty(),
            "example at line {} has no declarations",
            example.line
        );
    }
}

#[test]
fn opt_out_and_invalid_markers_are_honored() {
    let document = format!(
        "{EXAMPLES_HEADING}\n\n\
         ```brul,ignore\n\
         this is not markup at all\n\
         ```\n\n\
         ```brul,invalid\n\
         component {{ }}\n\
         ```\n\n\
         ```brul\n\
         component A {{ }}\n\
         ```\n"
    );
    let examples = examples(&document);
    assert_eq!(examples.len(), 3);
    assert_eq!(examples[0].kind, Kind::Ignore);
    assert_eq!(examples[1].kind, Kind::Invalid);
    assert_eq!(examples[2].kind, Kind::Parse);
    assert!(parse_file(&examples[1].body).is_err());
    assert!(parse_file(&examples[2].body).is_ok());
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
