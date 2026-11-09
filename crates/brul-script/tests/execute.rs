use brul_script::{Component, Error, Value, execute};
use brul_syntax::ast::Position;
use brul_syntax::script::parse;

fn run(source: &str) -> Result<Component, Error> {
    let program = parse(source).unwrap();
    let mut component = Component::new();
    execute(&mut component, &program.statements)?;
    Ok(component)
}

fn run_error(source: &str) -> Error {
    let program = parse(source).unwrap();
    let mut component = Component::new();
    execute(&mut component, &program.statements).unwrap_err()
}

#[test]
fn declarations_are_executed_in_order() {
    let component = run("let first = 1; const second = first + 2;").unwrap();
    assert_eq!(component.read("first"), Ok(Value::Number(1.0)));
    assert_eq!(component.read("second"), Ok(Value::Number(3.0)));
}

#[test]
fn let_without_initializer_is_null() {
    let component = run("let value;").unwrap();
    assert_eq!(component.read("value"), Ok(Value::Null));
}

#[test]
fn block_executes_its_statements() {
    let component = run("{ let inside = 1; let after = inside + 1; }").unwrap();
    assert_eq!(component.read("after"), Ok(Value::Number(2.0)));
}

#[test]
fn if_runs_only_the_taken_branch() {
    let component = run("let value = 0; if (true) { value = 1; } else { value = 2; }").unwrap();
    assert_eq!(component.read("value"), Ok(Value::Number(1.0)));

    let component = run("let value = 0; if (false) { value = 1; } else { value = 2; }").unwrap();
    assert_eq!(component.read("value"), Ok(Value::Number(2.0)));
}

#[test]
fn if_without_else_does_nothing_when_false() {
    let component = run("let value = 7; if (false) { value = 1; }").unwrap();
    assert_eq!(component.read("value"), Ok(Value::Number(7.0)));
}

#[test]
fn else_if_chain_picks_the_first_true_branch() {
    let source = r#"
        let value = 0;
        if (false) {
            value = 1;
        } else if (false) {
            value = 2;
        } else {
            value = 3;
        }
    "#;
    let component = run(source).unwrap();
    assert_eq!(component.read("value"), Ok(Value::Number(3.0)));
}

#[test]
fn condition_must_be_a_bool() {
    let error = run_error("if (1) { }");
    assert_eq!(
        error,
        Error::At(
            Position { line: 1, column: 5 },
            Box::new(Error::Expected {
                operation: "if",
                expected: "Bool",
                found: "Number",
            }),
        )
    );
}

#[test]
fn runtime_error_carries_the_expression_position() {
    let error = run_error("let value = 1 / 0;");
    assert_eq!(
        error.position(),
        Some(Position {
            line: 1,
            column: 13
        })
    );
    assert_eq!(error.to_string(), "line 1, column 13: division by zero");
}

#[test]
fn runtime_error_stops_the_script() {
    let mut component = Component::new();
    let program = parse("let value = 1 / 0; let after = 2;").unwrap();
    assert!(execute(&mut component, &program.statements).is_err());
    assert_eq!(
        component.read("after"),
        Err(Error::Undefined("after".to_owned()))
    );
}

#[test]
fn nested_error_keeps_the_inner_position() {
    let error = run_error("let value = 1 + [1][4];");
    assert_eq!(
        error.position(),
        Some(Position {
            line: 1,
            column: 17
        })
    );
    assert_eq!(
        error,
        Error::At(
            Position {
                line: 1,
                column: 17
            },
            Box::new(Error::OutOfBounds {
                index: 4,
                length: 1,
            }),
        )
    );
}

#[test]
fn assignment_to_a_non_place_is_unsupported() {
    let error = run_error("(1 + 2) = 3;");
    assert_eq!(error.reason(), Error::Unsupported("expression").reason());
}
