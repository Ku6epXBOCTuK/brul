use brul_syntax::markup::parse_file;

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
    assert_eq!(parsed.len(), 4, "expected four markup examples");
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
fn malformed_examples_are_rejected() {
    let sources = [
        ("component A { Text { View { } } }", "leaf widget"),
        ("component A { View (x) { }", "expected '}'"),
        (
            "component A { Row { each (items from item) { } } }",
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
