use brul_syntax::ast::{BinaryOp, Expr, UnaryOp};
use brul_syntax::parser::parse_expression;

fn ident(name: &str) -> Expr {
    Expr::Ident(name.to_owned())
}

fn field(target: Expr, name: &str) -> Expr {
    Expr::Field {
        target: Box::new(target),
        name: name.to_owned(),
    }
}

fn index(target: Expr, index: Expr) -> Expr {
    Expr::Index {
        target: Box::new(target),
        index: Box::new(index),
    }
}

fn call(target: Expr, args: Vec<Expr>) -> Expr {
    Expr::Call {
        target: Box::new(target),
        args,
    }
}

fn num(value: f64) -> Expr {
    Expr::Number(value)
}

fn bin(op: BinaryOp, left: Expr, right: Expr) -> Expr {
    Expr::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn unary(op: UnaryOp, operand: Expr) -> Expr {
    Expr::Unary {
        op,
        operand: Box::new(operand),
    }
}

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
fn postfix_chains_fields_indexes_and_calls() {
    assert_eq!(
        parse_expression("user.name").unwrap(),
        field(ident("user"), "name")
    );
    assert_eq!(
        parse_expression("items[1]").unwrap(),
        index(ident("items"), Expr::Number(1.0))
    );
    assert_eq!(parse_expression("f()").unwrap(), call(ident("f"), vec![]));
    assert_eq!(
        parse_expression("f(1, x)").unwrap(),
        call(ident("f"), vec![Expr::Number(1.0), ident("x")])
    );
    assert_eq!(
        parse_expression("a.b[0](c).d").unwrap(),
        field(
            call(
                index(field(ident("a"), "b"), Expr::Number(0.0)),
                vec![ident("c")]
            ),
            "d"
        )
    );
}

#[test]
fn postfix_errors_point_at_the_offending_token() {
    let error = parse_expression("user.").unwrap_err();
    assert_eq!(error.message, "ожидалось имя поля");
    assert_eq!((error.line, error.column), (1, 6));

    let error = parse_expression("items[0").unwrap_err();
    assert_eq!(error.message, "ожидался ']'");
    assert_eq!((error.line, error.column), (1, 8));

    let error = parse_expression("f(1").unwrap_err();
    assert_eq!(error.message, "ожидался ')'");
    assert_eq!((error.line, error.column), (1, 4));
}

#[test]
fn precedence_and_associativity() {
    assert_eq!(
        parse_expression("1 + 2 * 3").unwrap(),
        bin(
            BinaryOp::Add,
            num(1.0),
            bin(BinaryOp::Mul, num(2.0), num(3.0))
        )
    );
    assert_eq!(
        parse_expression("1 - 2 - 3").unwrap(),
        bin(
            BinaryOp::Sub,
            bin(BinaryOp::Sub, num(1.0), num(2.0)),
            num(3.0)
        )
    );
    assert_eq!(
        parse_expression("a || b && c").unwrap(),
        bin(
            BinaryOp::Or,
            ident("a"),
            bin(BinaryOp::And, ident("b"), ident("c"))
        )
    );
    assert_eq!(
        parse_expression("!a.b").unwrap(),
        unary(UnaryOp::Not, field(ident("a"), "b"))
    );
    assert_eq!(
        parse_expression("a < b == c").unwrap(),
        bin(
            BinaryOp::Eq,
            bin(BinaryOp::Lt, ident("a"), ident("b")),
            ident("c")
        )
    );
}

#[test]
fn dangling_operator_is_an_error() {
    let error = parse_expression("1 +").unwrap_err();
    assert_eq!(error.message, "ожидалось выражение");
    assert_eq!((error.line, error.column), (1, 4));
}

#[test]
fn lexer_errors_reach_the_entry() {
    let error = parse_expression("\"abc").unwrap_err();
    assert_eq!(error.message, "незакрытая строка");
    assert_eq!((error.line, error.column), (1, 1));
}
