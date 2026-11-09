use brul_syntax::ast::{AssignOp, BinaryOp, Expr, ExprKind, Position};
use brul_syntax::script::{Function, Program, Statement, parse};

fn expr(kind: ExprKind) -> Expr {
    Expr::new(kind, Position::default())
}

fn ident(name: &str) -> Expr {
    expr(ExprKind::Ident(name.to_owned()))
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

fn call(target: Expr, args: Vec<Expr>) -> Expr {
    expr(ExprKind::Call {
        target: Box::new(target),
        args,
    })
}

fn index(target: Expr, index: Expr) -> Expr {
    expr(ExprKind::Index {
        target: Box::new(target),
        index: Box::new(index),
    })
}

#[test]
fn empty_script_is_an_empty_program() {
    assert_eq!(
        parse("").unwrap(),
        Program {
            functions: vec![],
            statements: vec![],
        }
    );
}

#[test]
fn let_parses_name_and_value() {
    let program = parse("let browsers = [];").unwrap();
    assert_eq!(
        program.statements,
        vec![Statement::Let {
            constant: false,
            name: "browsers".to_owned(),
            value: Some(expr(ExprKind::Array(vec![]))),
        }]
    );
}

#[test]
fn let_parses_without_value() {
    let program = parse("let limit;").unwrap();
    assert_eq!(
        program.statements,
        vec![Statement::Let {
            constant: false,
            name: "limit".to_owned(),
            value: None,
        }]
    );
}

#[test]
fn const_parses_name_and_value() {
    let program = parse("const limit = 10;").unwrap();
    assert_eq!(
        program.statements,
        vec![Statement::Let {
            constant: true,
            name: "limit".to_owned(),
            value: Some(num(10.0)),
        }]
    );
}

#[test]
fn const_without_value_is_rejected() {
    let error = parse("const limit;").unwrap_err();
    assert_eq!(error.message, "const requires an initializer");
    assert_eq!((error.line, error.column), (1, 12));
}

#[test]
fn assignment_is_an_expression_statement() {
    let program = parse("count = count + 1;").unwrap();
    assert_eq!(
        program.statements,
        vec![Statement::Expr(expr(ExprKind::Assign {
            target: Box::new(ident("count")),
            op: AssignOp::Set,
            value: Box::new(bin(BinaryOp::Add, ident("count"), num(1.0))),
        }))]
    );
}

#[test]
fn expression_statements_keep_their_position() {
    let program = parse("\n  count = count + 1;").unwrap();
    let [Statement::Expr(expression)] = program.statements.as_slice() else {
        panic!("expected an expression statement");
    };
    assert_eq!(expression.position, Position { line: 2, column: 3 });
}

#[test]
fn block_holds_parenthesised_statement_list() {
    let program = parse("{ let a = 1; a; }").unwrap();
    assert_eq!(
        program.statements,
        vec![Statement::Block(vec![
            Statement::Let {
                constant: false,
                name: "a".to_owned(),
                value: Some(num(1.0)),
            },
            Statement::Expr(ident("a")),
        ])]
    );
}

#[test]
fn empty_braces_at_statement_position_are_a_block() {
    let program = parse("{}").unwrap();
    assert_eq!(program.statements, vec![Statement::Block(vec![])]);
}

#[test]
fn object_literal_at_statement_position_is_rejected() {
    let error = parse(r#"{ "a": 1 };"#).unwrap_err();
    assert_eq!(error.message, "expected ';'");
    assert_eq!((error.line, error.column), (1, 6));
}

#[test]
fn let_without_semicolon_points_at_the_end() {
    let error = parse("let a = 1").unwrap_err();
    assert_eq!(error.message, "expected ';'");
    assert_eq!((error.line, error.column), (1, 10));
}

#[test]
fn let_without_name_is_rejected() {
    let error = parse("let = 1;").unwrap_err();
    assert_eq!(error.message, "expected variable name");
    assert_eq!((error.line, error.column), (1, 5));
}

#[test]
fn unterminated_block_points_at_the_end() {
    let error = parse("{ let a = 1;").unwrap_err();
    assert_eq!(error.message, "expected '}'");
    assert_eq!((error.line, error.column), (1, 13));
}

#[test]
fn if_parses_condition_and_then_branch() {
    let program = parse("if (ready) { launch(); }").unwrap();
    assert_eq!(
        program.statements,
        vec![Statement::If {
            condition: ident("ready"),
            then_branch: vec![Statement::Expr(call(ident("launch"), vec![]))],
            else_branch: None,
        }]
    );
}

#[test]
fn if_condition_is_an_expression() {
    let program = parse("if (count > 0) { count = 0; }").unwrap();
    assert!(matches!(
        program.statements.as_slice(),
        [Statement::If {
            condition: Expr {
                kind: ExprKind::Binary {
                    op: BinaryOp::Gt,
                    ..
                },
                ..
            },
            ..
        }]
    ));
}

#[test]
fn if_with_else_parses_both_branches() {
    let program = parse("if (ready) { launch(); } else { wait(); }").unwrap();
    assert_eq!(
        program.statements,
        vec![Statement::If {
            condition: ident("ready"),
            then_branch: vec![Statement::Expr(call(ident("launch"), vec![]))],
            else_branch: Some(Box::new(Statement::Block(vec![Statement::Expr(call(
                ident("wait"),
                vec![]
            ))]))),
        }]
    );
}

#[test]
fn else_if_becomes_a_nested_if() {
    let program = parse("if (a) { } else if (b) { } else { }").unwrap();
    let [Statement::If { else_branch, .. }] = program.statements.as_slice() else {
        panic!("expected an if statement");
    };
    let Some(Statement::If { else_branch, .. }) = else_branch.as_deref() else {
        panic!("expected a nested if statement");
    };
    assert_eq!(else_branch.as_deref(), Some(&Statement::Block(vec![])));
}

#[test]
fn if_without_parentheses_is_rejected() {
    let error = parse("if ready { }").unwrap_err();
    assert_eq!(error.message, "expected '('");
    assert_eq!((error.line, error.column), (1, 4));
}

#[test]
fn if_without_block_is_rejected() {
    let error = parse("if (ready) launch();").unwrap_err();
    assert_eq!(error.message, "expected '{'");
    assert_eq!((error.line, error.column), (1, 12));
}

#[test]
fn function_parses_params_and_body() {
    let program = parse("function launch(i) { let browser = browsers[i]; }").unwrap();
    assert_eq!(
        program.functions,
        vec![Function {
            name: "launch".to_owned(),
            params: vec!["i".to_owned()],
            body: vec![Statement::Let {
                constant: false,
                name: "browser".to_owned(),
                value: Some(index(ident("browsers"), ident("i"))),
            }],
        }]
    );
}

#[test]
fn function_without_params_parses_empty_list() {
    let program = parse("function exit() { quit(); }").unwrap();
    assert_eq!(
        program.functions,
        vec![Function {
            name: "exit".to_owned(),
            params: vec![],
            body: vec![Statement::Expr(call(ident("quit"), vec![]))],
        }]
    );
}

#[test]
fn function_parses_several_params() {
    let program = parse("function move(x, y) { }").unwrap();
    assert_eq!(program.functions[0].params, vec!["x", "y"]);
}

#[test]
fn function_without_name_is_rejected() {
    let error = parse("function () { }").unwrap_err();
    assert_eq!(error.message, "expected function name");
    assert_eq!((error.line, error.column), (1, 10));
}

#[test]
fn function_without_body_is_rejected() {
    let error = parse("function f() launch();").unwrap_err();
    assert_eq!(error.message, "expected '{'");
    assert_eq!((error.line, error.column), (1, 14));
}

#[test]
fn function_param_with_trailing_comma_is_rejected() {
    let error = parse("function f(a,) { }").unwrap_err();
    assert_eq!(error.message, "expected parameter name");
    assert_eq!((error.line, error.column), (1, 14));
}

#[test]
fn nested_function_is_rejected() {
    let error = parse("function f() { function g() { } }").unwrap_err();
    assert_eq!(error.message, "functions are only allowed at the top level");
    assert_eq!((error.line, error.column), (1, 16));
}

#[test]
fn function_between_statements_is_a_top_level_item() {
    let program = parse("let a = 1; function f() { } let b = 2;").unwrap();
    assert_eq!(program.statements.len(), 2);
    assert_eq!(program.functions.len(), 1);
}
