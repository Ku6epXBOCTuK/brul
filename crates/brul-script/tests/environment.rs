use brul_script::{Environment, Error, Value, VariableId};

const NAMES: &[&str] = &["selectedIndex", "url", "count", "limit", "selected"];

fn environment() -> Environment {
    Environment::new(NAMES)
}

fn id(name: &str) -> VariableId {
    let index = NAMES
        .iter()
        .position(|candidate| *candidate == name)
        .expect("name is in the table");
    VariableId::from_index(index as u32)
}

#[test]
fn value_reports_its_kind() {
    assert_eq!(Value::Number(1.0).type_name(), "Number");
    assert_eq!(Value::String("a".to_owned()).type_name(), "String");
    assert_eq!(Value::Bool(true).type_name(), "Bool");
    assert_eq!(Value::Null.type_name(), "Null");
}

#[test]
fn declares_and_reads_variables() {
    let mut environment = environment();
    environment
        .declare(id("selectedIndex"), Value::Number(0.0), false)
        .unwrap();
    environment
        .declare(id("url"), Value::String("about:blank".to_owned()), false)
        .unwrap();
    assert_eq!(
        environment.get(id("selectedIndex")),
        Some(&Value::Number(0.0))
    );
    assert_eq!(
        environment.get(id("url")),
        Some(&Value::String("about:blank".to_owned()))
    );
}

#[test]
fn unknown_variable_is_undefined() {
    let environment = environment();
    assert_eq!(environment.get(id("selected")), None);
    assert_eq!(
        Error::Undefined("missing".to_owned()).to_string(),
        "variable 'missing' is not defined"
    );
}

#[test]
fn mutable_variable_takes_a_new_value() {
    let mut environment = environment();
    environment
        .declare(id("count"), Value::Number(1.0), false)
        .unwrap();
    environment.assign(id("count"), Value::Number(2.0)).unwrap();
    assert_eq!(environment.get(id("count")), Some(&Value::Number(2.0)));
}

#[test]
fn constant_rejects_assignment() {
    let mut environment = environment();
    environment
        .declare(id("limit"), Value::Number(10.0), true)
        .unwrap();
    assert_eq!(
        environment.assign(id("limit"), Value::Number(11.0)),
        Err(Error::Constant("limit".to_owned()))
    );
    assert_eq!(environment.get(id("limit")), Some(&Value::Number(10.0)));
}

#[test]
fn assigning_an_unknown_variable_is_undefined() {
    let mut environment = environment();
    assert_eq!(
        environment.assign(id("count"), Value::Number(1.0)),
        Err(Error::Undefined("count".to_owned()))
    );
}

#[test]
fn duplicate_declaration_is_rejected() {
    let mut environment = environment();
    environment
        .declare(id("count"), Value::Number(1.0), false)
        .unwrap();
    assert_eq!(
        environment.declare(id("count"), Value::Number(2.0), false),
        Err(Error::Redeclared("count".to_owned()))
    );
}

#[test]
fn null_round_trips() {
    let mut environment = environment();
    environment
        .declare(id("selected"), Value::Null, false)
        .unwrap();
    environment
        .assign(id("selected"), Value::Bool(true))
        .unwrap();
    assert_eq!(environment.get(id("selected")), Some(&Value::Bool(true)));
}
