use brul_syntax::ast::Expr;
use brul_syntax::parser::parse_expression;

#[test]
fn literals_and_identifiers_parse() {
    assert_eq!(parse_expression("42").unwrap(), Expr::Number(42.0));
    assert_eq!(parse_expression("2.5").unwrap(), Expr::Number(2.5));
    assert_eq!(
        parse_expression("\"hi\"").unwrap(),
        Expr::String("hi".to_owned())
    );
    assert_eq!(parse_expression("true").unwrap(), Expr::Bool(true));
    assert_eq!(parse_expression("false").unwrap(), Expr::Bool(false));
    assert_eq!(parse_expression("null").unwrap(), Expr::Null);
    assert_eq!(
        parse_expression("camelCase").unwrap(),
        Expr::Ident("camelCase".to_owned())
    );
}

#[test]
fn entry_skips_whitespace_and_comments() {
    assert_eq!(
        parse_expression("  // note\n true ").unwrap(),
        Expr::Bool(true)
    );
}

#[test]
fn trailing_tokens_are_errors() {
    let error = parse_expression("1 2").unwrap_err();
    assert_eq!(error.message, "ожидался конец выражения");
    assert_eq!((error.line, error.column), (1, 3));
}

#[test]
fn empty_input_is_an_error() {
    let error = parse_expression("").unwrap_err();
    assert_eq!(error.message, "ожидалось выражение");
    assert_eq!((error.line, error.column), (1, 1));
}

#[test]
fn templates_are_not_literals_yet() {
    let error = parse_expression("`x`").unwrap_err();
    assert_eq!(error.message, "ожидалось выражение");
    assert_eq!((error.line, error.column), (1, 1));
}

#[test]
fn lexer_errors_reach_the_entry() {
    let error = parse_expression("\"abc").unwrap_err();
    assert_eq!(error.message, "незакрытая строка");
    assert_eq!((error.line, error.column), (1, 1));
}
