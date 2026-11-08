use brul_syntax::ast::{AssignOp, BinaryOp, Expr};
use brul_syntax::script::{Function, Import, Program, Statement, parse};

#[test]
fn empty_script_is_an_empty_program() {
    assert_eq!(
        parse("").unwrap(),
        Program {
            imports: vec![],
            functions: vec![],
            statements: vec![],
        }
    );
}

#[test]
fn import_parses_name_and_path() {
    let program = parse(r#"import BrowserItem from "./browser_item";"#).unwrap();
    assert_eq!(
        program.imports,
        vec![Import {
            name: "BrowserItem".to_owned(),
            path: "./browser_item".to_owned(),
        }]
    );
}

#[test]
fn imports_parse_in_order_and_skip_comments() {
    let source = r#"
        // components used by the markup
        import BrowserItem from "./browser_item";

        import Footer from "./footer";
    "#;
    let program = parse(source).unwrap();
    assert_eq!(
        program
            .imports
            .iter()
            .map(|import| import.name.as_str())
            .collect::<Vec<_>>(),
        vec!["BrowserItem", "Footer"]
    );
}

#[test]
fn import_without_name_points_at_the_path() {
    let error = parse(r#"import "./a";"#).unwrap_err();
    assert_eq!(error.message, "expected import name");
    assert_eq!((error.line, error.column), (1, 8));
}

#[test]
fn import_without_from_points_at_the_path() {
    let error = parse(r#"import A "./a";"#).unwrap_err();
    assert_eq!(error.message, "expected 'from'");
    assert_eq!((error.line, error.column), (1, 10));
}

#[test]
fn import_with_non_string_path_is_rejected() {
    let error = parse("import A from x;").unwrap_err();
    assert_eq!(error.message, "expected module path");
    assert_eq!((error.line, error.column), (1, 15));
}

#[test]
fn import_without_semicolon_points_at_the_end() {
    let error = parse(r#"import A from "./a""#).unwrap_err();
    assert_eq!(error.message, "expected ';'");
    assert_eq!((error.line, error.column), (1, 20));
}

#[test]
fn let_parses_name_and_value() {
    let program = parse("let browsers = [];").unwrap();
    assert_eq!(
        program.statements,
        vec![Statement::Let {
            constant: false,
            name: "browsers".to_owned(),
            value: Some(Expr::Array(vec![])),
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
            value: Some(Expr::Number(10.0)),
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
        vec![Statement::Expr(Expr::Assign {
            target: Box::new(Expr::Ident("count".to_owned())),
            op: AssignOp::Set,
            value: Box::new(Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expr::Ident("count".to_owned())),
                right: Box::new(Expr::Number(1.0)),
            }),
        })]
    );
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
                value: Some(Expr::Number(1.0)),
            },
            Statement::Expr(Expr::Ident("a".to_owned())),
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
            condition: Expr::Ident("ready".to_owned()),
            then_branch: vec![Statement::Expr(Expr::Call {
                target: Box::new(Expr::Ident("launch".to_owned())),
                args: vec![],
            })],
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
            condition: Expr::Binary {
                op: BinaryOp::Gt,
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
            condition: Expr::Ident("ready".to_owned()),
            then_branch: vec![Statement::Expr(Expr::Call {
                target: Box::new(Expr::Ident("launch".to_owned())),
                args: vec![],
            })],
            else_branch: Some(Box::new(Statement::Block(vec![Statement::Expr(
                Expr::Call {
                    target: Box::new(Expr::Ident("wait".to_owned())),
                    args: vec![],
                }
            )]))),
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
                value: Some(Expr::Index {
                    target: Box::new(Expr::Ident("browsers".to_owned())),
                    index: Box::new(Expr::Ident("i".to_owned())),
                }),
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
            body: vec![Statement::Expr(Expr::Call {
                target: Box::new(Expr::Ident("quit".to_owned())),
                args: vec![],
            })],
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

#[test]
fn statements_follow_imports() {
    let program = parse("import A from \"./a\";\nlet a = 1;").unwrap();
    assert_eq!(program.imports.len(), 1);
    assert_eq!(program.statements.len(), 1);
}
