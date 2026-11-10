const MARKUP: &str = include_str!("../../../docs/lang/markup.md");
const STYLE: &str = include_str!("../../../docs/lang/style.md");
const SCRIPT: &str = include_str!("../../../docs/lang/script.md");
const LOCK: &str = include_str!("fixtures/spec_examples.lock");

const EXAMPLES_HEADING: &str =
    "## \u{422}\u{435}\u{441}\u{442}-\u{43f}\u{440}\u{438}\u{43c}\u{435}\u{440}\u{44b}";

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Parse,
    Ignore,
    Invalid,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Ignore => "ignore",
            Self::Invalid => "invalid",
        }
    }
}

fn examples(document: &str, language: &str) -> Vec<(Kind, String)> {
    let mut out = Vec::new();
    let mut inside = false;
    let mut open: Option<(Kind, String)> = None;
    for text in document.lines() {
        if !inside {
            inside = text.starts_with(EXAMPLES_HEADING);
            continue;
        }
        if text.starts_with("## ") {
            break;
        }
        if let Some((kind, body)) = &mut open {
            if text.starts_with("```") {
                out.push((*kind, std::mem::take(body)));
                open = None;
            } else {
                body.push_str(text);
                body.push('\n');
            }
        } else if let Some(info) = text.strip_prefix("```") {
            let Some(rest) = info.trim().strip_prefix(language) else {
                continue;
            };
            let kind = match rest {
                "" => Kind::Parse,
                ",ignore" => Kind::Ignore,
                ",invalid" => Kind::Invalid,
                _ => continue,
            };
            open = Some((kind, String::new()));
        }
    }
    out
}

fn hash(kind: Kind, body: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325;
    for byte in kind.name().bytes().chain([0]).chain(body.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn frozen() -> Vec<String> {
    let documents = [
        ("markup.md", MARKUP, "brul"),
        ("style.md", STYLE, "css"),
        ("script.md", SCRIPT, "script"),
    ];
    let mut out = Vec::new();
    for (file, document, language) in documents {
        for (kind, body) in examples(document, language) {
            out.push(format!("{file} {} {:016x}", kind.name(), hash(kind, &body)));
        }
    }
    out
}

#[test]
fn spec_examples_are_frozen() {
    let actual = frozen();
    let expected: Vec<String> = LOCK
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    assert!(
        expected == actual,
        "spec examples changed; update crates/brul-syntax/tests/fixtures/spec_examples.lock:\n{}",
        actual.join("\n")
    );
}
