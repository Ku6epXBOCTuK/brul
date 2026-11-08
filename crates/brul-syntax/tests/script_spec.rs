use brul_syntax::script::parse;

const SCRIPT: &str = include_str!("../../../docs/lang/script.md");

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
                    "script" => Kind::Parse,
                    "script,ignore" => Kind::Ignore,
                    "script,invalid" => Kind::Invalid,
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
    let examples = examples(SCRIPT);
    assert_eq!(examples.len(), 4, "expected four script examples");
    let parsed: Vec<&Example> = examples
        .iter()
        .filter(|example| example.kind == Kind::Parse)
        .collect();
    assert_eq!(parsed.len(), 3, "expected three script programs");
    for example in parsed {
        let program = parse(&example.body)
            .unwrap_or_else(|error| panic!("example at line {}: {error}", example.line));
        assert!(
            !program.statements.is_empty() || !program.functions.is_empty(),
            "example at line {} is empty",
            example.line
        );
    }
    let ignored: Vec<&Example> = examples
        .iter()
        .filter(|example| example.kind == Kind::Ignore)
        .collect();
    assert_eq!(ignored.len(), 1, "expected one markup expression example");
}

#[test]
fn opt_out_and_invalid_markers_are_honored() {
    let document = format!(
        "{EXAMPLES_HEADING}\n\n\
         ```script,ignore\n\
         this is not script at all\n\
         ```\n\n\
         ```script,invalid\n\
         let = 1;\n\
         ```\n\n\
         ```script\n\
         let value = 1;\n\
         ```\n"
    );
    let examples = examples(&document);
    assert_eq!(examples.len(), 3);
    assert_eq!(examples[0].kind, Kind::Ignore);
    assert_eq!(examples[1].kind, Kind::Invalid);
    assert_eq!(examples[2].kind, Kind::Parse);
    assert!(parse(&examples[1].body).is_err());
    assert!(parse(&examples[2].body).is_ok());
}

#[test]
fn malformed_examples_are_rejected() {
    let sources = [
        ("let value = ;", "expected expression"),
        ("let value = 1", "expected ';'"),
        ("const limit;", "const requires an initializer"),
        ("{ name: \"Chrome\" };", "expected ';'"),
        (
            "function run() { function inner() { } }",
            "only allowed at the top level",
        ),
    ];
    for (source, message) in sources {
        let error = parse(source).unwrap_err();
        assert!(
            error.message.contains(message),
            "{source}: expected message containing {message}, got {}",
            error.message
        );
    }
}
