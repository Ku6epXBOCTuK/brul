use brul_script::{Environment, Error, Value};

#[test]
fn value_reports_its_kind() {
    assert_eq!(Value::Number(1.0).type_name(), "Number");
    assert_eq!(Value::String("a".to_owned()).type_name(), "String");
    assert_eq!(Value::Bool(true).type_name(), "Bool");
    assert_eq!(Value::Null.type_name(), "Null");
}

#[test]
fn declares_and_reads_variables() {
    let mut environment = Environment::new();
    environment
        .declare("selectedIndex", Value::Number(0.0), false)
        .unwrap();
    environment
        .declare("url", Value::String("about:blank".to_owned()), false)
        .unwrap();
    assert_eq!(environment.get("selectedIndex"), Some(&Value::Number(0.0)));
    assert_eq!(
        environment.get("url"),
        Some(&Value::String("about:blank".to_owned()))
    );
}

#[test]
fn unknown_variable_is_undefined() {
    let environment = Environment::new();
    assert_eq!(environment.get("missing"), None);
    assert_eq!(
        Error::Undefined("missing".to_owned()).to_string(),
        "variable 'missing' is not defined"
    );
}

#[test]
fn mutable_variable_takes_a_new_value() {
    let mut environment = Environment::new();
    environment
        .declare("count", Value::Number(1.0), false)
        .unwrap();
    environment.assign("count", Value::Number(2.0)).unwrap();
    assert_eq!(environment.get("count"), Some(&Value::Number(2.0)));
}

#[test]
fn constant_rejects_assignment() {
    let mut environment = Environment::new();
    environment
        .declare("limit", Value::Number(10.0), true)
        .unwrap();
    assert_eq!(
        environment.assign("limit", Value::Number(11.0)),
        Err(Error::Constant("limit".to_owned()))
    );
    assert_eq!(environment.get("limit"), Some(&Value::Number(10.0)));
}

#[test]
fn assigning_an_unknown_variable_is_undefined() {
    let mut environment = Environment::new();
    assert_eq!(
        environment.assign("count", Value::Number(1.0)),
        Err(Error::Undefined("count".to_owned()))
    );
}

#[test]
fn duplicate_declaration_is_rejected() {
    let mut environment = Environment::new();
    environment
        .declare("count", Value::Number(1.0), false)
        .unwrap();
    assert_eq!(
        environment.declare("count", Value::Number(2.0), false),
        Err(Error::Redeclared("count".to_owned()))
    );
}

#[test]
fn null_round_trips() {
    let mut environment = Environment::new();
    environment.declare("selected", Value::Null, false).unwrap();
    environment.assign("selected", Value::Bool(true)).unwrap();
    assert_eq!(environment.get("selected"), Some(&Value::Bool(true)));
}
