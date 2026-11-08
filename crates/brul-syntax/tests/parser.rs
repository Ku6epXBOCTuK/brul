use brul_syntax::ast::{AssignOp, BinaryOp, Expr, ExprKind, Position, TemplatePart, UnaryOp};
use brul_syntax::parser::parse_expression;

fn expr(kind: ExprKind) -> Expr {
    Expr::new(kind, Position::default())
}

fn ident(name: &str) -> Expr {
    expr(ExprKind::Ident(name.to_owned()))
}

fn field(target: Expr, name: &str) -> Expr {
    expr(ExprKind::Field {
        target: Box::new(target),
        name: name.to_owned(),
    })
}

fn index(target: Expr, index: Expr) -> Expr {
    expr(ExprKind::Index {
        target: Box::new(target),
        index: Box::new(index),
    })
}

fn call(target: Expr, args: Vec<Expr>) -> Expr {
    expr(ExprKind::Call {
        target: Box::new(target),
        args,
    })
}

fn num(value: f64) -> Expr {
    expr(ExprKind::Number(value))
}

fn bin(op: BinaryOp, left: Expr, right: Expr) -> Expr {
    expr(ExprKind::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn unary(op: UnaryOp, operand: Expr) -> Expr {
    expr(ExprKind::Unary {
        op,
        operand: Box::new(operand),
    })
}

fn ternary(condition: Expr, then_branch: Expr, else_branch: Expr) -> Expr {
    expr(ExprKind::Ternary {
        condition: Box::new(condition),
        then_branch: Box::new(then_branch),
        else_branch: Box::new(else_branch),
    })
}

fn assign(op: AssignOp, target: Expr, value: Expr) -> Expr {
    expr(ExprKind::Assign {
        target: Box::new(target),
        op,
        value: Box::new(value),
    })
}

fn array(items: Vec<Expr>) -> Expr {
    expr(ExprKind::Array(items))
}

fn object(fields: Vec<(&str, Expr)>) -> Expr {
    expr(ExprKind::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    ))
}

#[test]
fn expressions_carry_their_source_position() {
    let expression = parse_expression("1 + 2 * 3").unwrap();
    assert_eq!(expression.position, Position { line: 1, column: 1 });
    let ExprKind::Binary { right, .. } = &expression.kind else {
        panic!("expected a sum");
    };
    assert_eq!(right.position, Position { line: 1, column: 5 });
    let ExprKind::Binary { right, .. } = &right.kind else {
        panic!("expected a product");
    };
    assert_eq!(right.position, Position { line: 1, column: 9 });
}

#[test]
fn literals_and_identifiers_parse() {
    assert_eq!(parse_expression("42").unwrap(), num(42.0));
    assert_eq!(parse_expression("2.5").unwrap(), num(2.5));
    assert_eq!(
        parse_expression("\"hi\"").unwrap(),
        expr(ExprKind::String("hi".to_owned()))
    );
    assert_eq!(
        parse_expression("true").unwrap(),
        expr(ExprKind::Bool(true))
    );
    assert_eq!(
        parse_expression("false").unwrap(),
        expr(ExprKind::Bool(false))
    );
    assert_eq!(parse_expression("null").unwrap(), expr(ExprKind::Null));
    assert_eq!(parse_expression("camelCase").unwrap(), ident("camelCase"));
}

#[test]
fn entry_skips_whitespace_and_comments() {
    assert_eq!(
        parse_expression("  // note\n true ").unwrap(),
        expr(ExprKind::Bool(true))
    );
}

#[test]
fn trailing_tokens_are_errors() {
    let error = parse_expression("1 2").unwrap_err();
    assert_eq!(error.message, "expected end of expression");
    assert_eq!((error.line, error.column), (1, 3));
}

#[test]
fn empty_input_is_an_error() {
    let error = parse_expression("").unwrap_err();
    assert_eq!(error.message, "expected expression");
    assert_eq!((error.line, error.column), (1, 1));
}

fn template(parts: Vec<TemplatePart>) -> Expr {
    expr(ExprKind::Template(parts))
}

fn text(value: &str) -> TemplatePart {
    TemplatePart::Text(value.to_owned())
}

fn part(expr: Expr) -> TemplatePart {
    TemplatePart::Expr(Box::new(expr))
}

#[test]
fn templates_parse_text_and_interpolation() {
    assert_eq!(parse_expression("``").unwrap(), template(vec![]));
    assert_eq!(
        parse_expression("`hi`").unwrap(),
        template(vec![text("hi")])
    );
    assert_eq!(
        parse_expression("`a${b}c`").unwrap(),
        template(vec![text("a"), part(ident("b")), text("c")])
    );
    assert_eq!(
        parse_expression("`x${`y${z}`}`").unwrap(),
        template(vec![
            text("x"),
            part(template(vec![text("y"), part(ident("z"))])),
        ])
    );
    assert_eq!(
        parse_expression("`sum ${1 + 2}`").unwrap(),
        template(vec![
            text("sum "),
            part(bin(BinaryOp::Add, num(1.0), num(2.0))),
        ])
    );
}

#[test]
fn template_errors_point_into_the_expression() {
    let error = parse_expression("`a${1 +}`").unwrap_err();
    assert_eq!(error.message, "expected expression");
    assert_eq!((error.line, error.column), (1, 8));

    let error = parse_expression("`a${1 + 2").unwrap_err();
    assert_eq!(error.message, "unterminated template");
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
        index(ident("items"), num(1.0))
    );
    assert_eq!(parse_expression("f()").unwrap(), call(ident("f"), vec![]));
    assert_eq!(
        parse_expression("f(1, x)").unwrap(),
        call(ident("f"), vec![num(1.0), ident("x")])
    );
    assert_eq!(
        parse_expression("a.b[0](c).d").unwrap(),
        field(
            call(index(field(ident("a"), "b"), num(0.0)), vec![ident("c")]),
            "d"
        )
    );
}

#[test]
fn postfix_errors_point_at_the_offending_token() {
    let error = parse_expression("user.").unwrap_err();
    assert_eq!(error.message, "expected field name");
    assert_eq!((error.line, error.column), (1, 6));

    let error = parse_expression("items[0").unwrap_err();
    assert_eq!(error.message, "expected ']'");
    assert_eq!((error.line, error.column), (1, 8));

    let error = parse_expression("f(1").unwrap_err();
    assert_eq!(error.message, "expected ')'");
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
    assert_eq!(error.message, "expected expression");
    assert_eq!((error.line, error.column), (1, 4));
}

#[test]
fn lexer_errors_reach_the_entry() {
    let error = parse_expression("\"abc").unwrap_err();
    assert_eq!(error.message, "unterminated string");
    assert_eq!((error.line, error.column), (1, 1));
}

#[test]
fn ternary_is_right_associative() {
    assert_eq!(
        parse_expression("a ? b : c ? d : e").unwrap(),
        ternary(
            ident("a"),
            ident("b"),
            ternary(ident("c"), ident("d"), ident("e"))
        )
    );
    assert_eq!(
        parse_expression("a ? b ? c : d : e").unwrap(),
        ternary(
            ident("a"),
            ternary(ident("b"), ident("c"), ident("d")),
            ident("e")
        )
    );
}

#[test]
fn assignment_binds_looser_than_ternary_and_right() {
    assert_eq!(
        parse_expression("a = b ? c : d").unwrap(),
        assign(
            AssignOp::Set,
            ident("a"),
            ternary(ident("b"), ident("c"), ident("d"))
        )
    );
    assert_eq!(
        parse_expression("a = b += 1").unwrap(),
        assign(
            AssignOp::Set,
            ident("a"),
            assign(AssignOp::Add, ident("b"), num(1.0))
        )
    );
    assert_eq!(
        parse_expression("obj.x -= 1").unwrap(),
        assign(AssignOp::Sub, field(ident("obj"), "x"), num(1.0))
    );
}

#[test]
fn missing_colon_after_then_branch_is_an_error() {
    let error = parse_expression("a ? b").unwrap_err();
    assert_eq!(error.message, "expected ':'");
    assert_eq!((error.line, error.column), (1, 6));
}

#[test]
fn arrays_and_objects_parse() {
    assert_eq!(parse_expression("[]").unwrap(), array(vec![]));
    assert_eq!(
        parse_expression("[1, x]").unwrap(),
        array(vec![num(1.0), ident("x")])
    );
    assert_eq!(parse_expression("{}").unwrap(), object(vec![]));
    assert_eq!(
        parse_expression("{a: 1, b: \"x\"}").unwrap(),
        object(vec![
            ("a", num(1.0)),
            ("b", expr(ExprKind::String("x".to_owned())))
        ])
    );
    assert_eq!(
        parse_expression("[{a: [1]}]").unwrap(),
        array(vec![object(vec![("a", array(vec![num(1.0)]))])])
    );
}

#[test]
fn parentheses_group_expressions() {
    assert_eq!(
        parse_expression("(1 + 2) * 3").unwrap(),
        bin(
            BinaryOp::Mul,
            bin(BinaryOp::Add, num(1.0), num(2.0)),
            num(3.0)
        )
    );
    assert_eq!(parse_expression("(a)").unwrap(), ident("a"));
}

#[test]
fn container_errors_point_at_the_offending_token() {
    let error = parse_expression("[1").unwrap_err();
    assert_eq!(error.message, "expected ']'");
    assert_eq!((error.line, error.column), (1, 3));

    let error = parse_expression("{1: 2}").unwrap_err();
    assert_eq!(error.message, "expected field name");
    assert_eq!((error.line, error.column), (1, 2));

    let error = parse_expression("{a 1}").unwrap_err();
    assert_eq!(error.message, "expected ':'");
    assert_eq!((error.line, error.column), (1, 4));

    let error = parse_expression("{a: 1").unwrap_err();
    assert_eq!(error.message, "expected '}'");
    assert_eq!((error.line, error.column), (1, 6));

    let error = parse_expression("[1,").unwrap_err();
    assert_eq!(error.message, "expected expression");
    assert_eq!((error.line, error.column), (1, 4));
}
