use std::fmt::{Debug, Display};

pub const EXAMPLES_HEADING: &str = "## Test examples";

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Parse,
    Ignore,
    Invalid,
    Other,
}

pub struct Example {
    pub kind: Kind,
    pub line: usize,
    pub body: String,
}

pub fn examples(document: &str, language: &str) -> Vec<Example> {
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
                let mut parts = info.trim().split(',');
                if parts.next() == Some(language) {
                    kind = match parts.next() {
                        None => Kind::Parse,
                        Some("ignore") => Kind::Ignore,
                        Some("invalid") => Kind::Invalid,
                        _ => Kind::Other,
                    };
                    line = number;
                    body.clear();
                }
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

pub fn assert_document_examples_parse<T, E, F, C>(
    document: &str,
    language: &str,
    parse: F,
    check: C,
) where
    F: Fn(&str) -> Result<T, E>,
    E: Debug + Display,
    C: Fn(&Example, &T),
{
    let examples = examples(document, language);
    let parsed: Vec<&Example> = examples
        .iter()
        .filter(|example| example.kind == Kind::Parse)
        .collect();
    assert!(
        !parsed.is_empty(),
        "no '{language}' examples under {EXAMPLES_HEADING}"
    );
    for example in parsed {
        let value = parse(&example.body)
            .unwrap_or_else(|error| panic!("example at line {}: {error}", example.line));
        check(example, &value);
    }
}

pub fn assert_markers_are_honored<T, E, F>(
    language: &str,
    ignore_body: &str,
    invalid_body: &str,
    parse_body: &str,
    parse: F,
) where
    F: Fn(&str) -> Result<T, E>,
{
    let document = format!(
        "{EXAMPLES_HEADING}\n\n\
         ```{language},ignore\n\
         {ignore_body}\n\
         ```\n\n\
         ```{language},invalid\n\
         {invalid_body}\n\
         ```\n\n\
         ```{language}\n\
         {parse_body}\n\
         ```\n"
    );
    let examples = examples(&document, language);
    assert_eq!(examples.len(), 3);
    assert_eq!(examples[0].kind, Kind::Ignore);
    assert_eq!(examples[1].kind, Kind::Invalid);
    assert_eq!(examples[2].kind, Kind::Parse);
    assert!(parse(&examples[1].body).is_err());
    assert!(parse(&examples[2].body).is_ok());
}
