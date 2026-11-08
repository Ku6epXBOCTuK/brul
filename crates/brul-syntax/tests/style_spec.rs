use brul_syntax::style::parse_file;

const STYLE: &str = include_str!("../../../docs/lang/style.md");

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
                    "css" => Kind::Parse,
                    "css,ignore" => Kind::Ignore,
                    "css,invalid" => Kind::Invalid,
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
    let examples = examples(STYLE);
    let parsed: Vec<&Example> = examples
        .iter()
        .filter(|example| example.kind == Kind::Parse)
        .collect();
    assert_eq!(parsed.len(), 4, "expected four style examples");
    for example in parsed {
        let sheet = parse_file(&example.body)
            .unwrap_or_else(|error| panic!("example at line {}: {error}", example.line));
        assert!(
            !sheet.rules.is_empty() || !sheet.themes.is_empty(),
            "example at line {} is empty",
            example.line
        );
    }
}

#[test]
fn opt_out_and_invalid_markers_are_honored() {
    let document = format!(
        "{EXAMPLES_HEADING}\n\n\
         ```css,ignore\n\
         this is not style at all\n\
         ```\n\n\
         ```css,invalid\n\
         .a {{ width 4px; }}\n\
         ```\n\n\
         ```css\n\
         .a {{ width: 4px; }}\n\
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
            error.message.contains(message),
            "{source}: expected message containing {message}, got {}",
            error.message
        );
    }
}
