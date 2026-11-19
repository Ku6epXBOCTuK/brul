mod common;

use brul_syntax::script::parse;
use common::Kind;

const SCRIPT: &str = include_str!("../../../docs/lang/script.md");

#[test]
fn document_examples_parse() {
    common::assert_document_examples_parse(SCRIPT, "script", parse, |example, program| {
        assert!(
            !program.statements.is_empty() || !program.functions.is_empty(),
            "example at line {} is empty",
            example.line
        );
    });
    let ignored = common::examples(SCRIPT, "script")
        .iter()
        .filter(|example| example.kind == Kind::Ignore)
        .count();
    assert_eq!(
        ignored, 4,
        "expected annotated and markup examples to be ignored"
    );
}

#[test]
fn opt_out_and_invalid_markers_are_honored() {
    common::assert_markers_are_honored(
        "script",
        "this is not script at all",
        "let = 1;",
        "let value: Number = 1;",
        parse,
    );
}

#[test]
fn malformed_examples_are_rejected() {
    let sources = [
        ("let value: Number = ;", "expected expression"),
        ("let value: Number = 1", "expected ';'"),
        ("const limit: Number;", "const requires an initializer"),
        ("{ name: \"Chrome\" };", "expected ';'"),
        (
            "function run(): Void { function inner(): Void { } }",
            "only allowed at the top level",
        ),
    ];
    for (source, message) in sources {
        let error = parse(source).unwrap_err();
        assert!(
            error.message().contains(message),
            "{source}: expected message containing {message}, got {}",
            error.message()
        );
    }
}
