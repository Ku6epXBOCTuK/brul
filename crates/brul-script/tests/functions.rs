use brul_script::{Component, Error, Value, execute_program};
use brul_syntax::ast::Position;
use brul_syntax::script::parse;

const NAMES: &[&str] = &["total", "n", "value", "step", "count", "a", "b"];

fn component() -> Component {
    let mut component = Component::new();
    component.set_names(NAMES);
    component
}

fn run(source: &str) -> Component {
    let program = parse(source).unwrap();
    let mut component = component();
    execute_program(&mut component, &program).unwrap();
    component
}

#[test]
fn call_executes_the_function_body() {
    let component = run("let total = 0; function bump() { total = total + 1; } bump();");
    assert_eq!(component.read("total"), Ok(Value::Number(1.0)));
}

#[test]
fn parameters_bind_the_call_arguments() {
    let component = run("let total = 0; function add(n) { total = total + n; } add(3);");
    assert_eq!(component.read("total"), Ok(Value::Number(3.0)));
}

#[test]
fn parameter_shadows_a_component_variable() {
    let component = run("let value = 1; function change(value) { value = 2; } change(5);");
    assert_eq!(component.read("value"), Ok(Value::Number(1.0)));
}

#[test]
fn local_declarations_do_not_leak_between_calls() {
    let component = run(
        "let total = 0; function bump() { let step = 1; total = total + step; } bump(); bump();",
    );
    assert_eq!(component.read("total"), Ok(Value::Number(2.0)));
    assert_eq!(
        component.read("step"),
        Err(Error::Undefined("step".to_owned()))
    );
}

#[test]
fn function_may_mutate_a_property() {
    let mut component = component();
    let count = component.id("count").expect("count is in the table");
    component
        .declare_property(count, Value::Number(0.0))
        .unwrap();
    let program = parse("function bump() { count = count + 1; } bump();").unwrap();
    execute_program(&mut component, &program).unwrap();
    assert_eq!(component.read("count"), Ok(Value::Number(1.0)));
}

#[test]
fn function_may_call_itself() {
    let component = run(
        "let total = 0;\nfunction sum(n) {\n  if (n > 0) {\n    total = total + n;\n    sum(n - 1);\n  }\n}\nsum(3);",
    );
    assert_eq!(component.read("total"), Ok(Value::Number(6.0)));
}

#[test]
fn functions_are_registered_before_statements_run() {
    let component = run("let total = 0; bump(); function bump() { total = total + 1; }");
    assert_eq!(component.read("total"), Ok(Value::Number(1.0)));
}

#[test]
fn arity_mismatch_reports_the_call_position() {
    let program = parse("function pair(a, b) { }\npair(1);").unwrap();
    let mut component = component();
    let error = execute_program(&mut component, &program).unwrap_err();
    assert_eq!(
        error,
        Error::At(
            Position { line: 2, column: 1 },
            Box::new(Error::FunctionArity {
                name: "pair".to_owned(),
                expected: 2,
                found: 1,
            }),
        )
    );
    assert_eq!(
        error.to_string(),
        "line 2, column 1: function 'pair' expects 2 arguments, got 1"
    );
}

#[test]
fn duplicate_function_is_rejected() {
    let program = parse("function run() { }\nfunction run() { }").unwrap();
    let mut component = component();
    assert_eq!(
        execute_program(&mut component, &program),
        Err(Error::FunctionRedeclared("run".to_owned()))
    );
}

#[test]
fn unknown_function_still_falls_through_to_the_host() {
    let program = parse("launchBrowser(1);").unwrap();
    let mut component = component();
    assert_eq!(
        execute_program(&mut component, &program),
        Err(Error::At(
            Position { line: 1, column: 1 },
            Box::new(Error::UnknownFunction("launchBrowser".to_owned())),
        ))
    );
}
