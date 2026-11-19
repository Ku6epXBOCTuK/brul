use std::cell::RefCell;
use std::rc::Rc;

use brul_reactive::{Effect, flush};
use brul_script::{Component, Error, Scope, Value, VariableId, eval};
use brul_syntax::parser::parse_expression;

const NAMES: &[&str] = &[
    "checked", "count", "limit", "label", "offset", "missing", "item", "row",
];

fn component() -> Component {
    let mut component = Component::new();
    component.set_names(NAMES);
    component
}

fn id(name: &str) -> VariableId {
    let index = NAMES
        .iter()
        .position(|candidate| *candidate == name)
        .expect("name is in the table");
    VariableId::from_index(index as u32)
}

fn evaluate(component: &mut Component, source: &str) -> Result<Value, Error> {
    eval(component, &parse_expression(source).unwrap())
}

fn failure(component: &mut Component, source: &str) -> Error {
    match evaluate(component, source).unwrap_err() {
        Error::At(_, error) => *error,
        error => error,
    }
}

#[test]
fn property_reads_its_value() {
    let mut component = component();
    component
        .declare_property(id("checked"), Value::Bool(false))
        .unwrap();

    assert_eq!(component.read("checked"), Ok(Value::Bool(false)));
}

#[test]
fn ids_address_the_same_bindings_as_names() {
    let mut component = component();
    component
        .declare("count", Value::Number(2.0), false)
        .unwrap();
    component
        .declare_property(id("checked"), Value::Bool(false))
        .unwrap();

    assert_eq!(component.read_id(id("count")), Ok(Value::Number(2.0)));
    assert_eq!(component.read_id(id("checked")), Ok(Value::Bool(false)));
    component
        .assign_id(id("checked"), Value::Bool(true))
        .unwrap();
    assert_eq!(component.read("checked"), Ok(Value::Bool(true)));
}

#[test]
fn variable_shadows_a_property() {
    let mut component = component();
    component
        .declare_property(id("count"), Value::Number(1.0))
        .unwrap();
    component
        .declare("count", Value::Number(2.0), false)
        .unwrap();

    assert_eq!(component.read("count"), Ok(Value::Number(2.0)));
}

#[test]
fn assignment_writes_a_property() {
    let mut component = component();
    component
        .declare_property(id("checked"), Value::Bool(false))
        .unwrap();

    assert_eq!(
        evaluate(&mut component, "checked = true"),
        Ok(Value::Bool(true))
    );
    assert_eq!(component.read("checked"), Ok(Value::Bool(true)));
}

#[test]
fn assignment_writes_a_let_variable() {
    let mut component = component();
    component
        .declare("count", Value::Number(1.0), false)
        .unwrap();

    assert_eq!(
        evaluate(&mut component, "count = count + 2"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(component.read("count"), Ok(Value::Number(3.0)));
}

#[test]
fn compound_assignment_reads_the_current_value() {
    let mut component = component();
    component
        .declare("count", Value::Number(1.0), false)
        .unwrap();
    component
        .declare_property(id("offset"), Value::Number(4.0))
        .unwrap();

    assert_eq!(
        evaluate(&mut component, "count += 2"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        evaluate(&mut component, "offset -= 1"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn assignment_respects_constants() {
    let mut component = component();
    component
        .declare("limit", Value::Number(1.0), true)
        .unwrap();

    assert_eq!(
        failure(&mut component, "limit = 2"),
        Error::Constant("limit".to_owned())
    );
    assert_eq!(component.read("limit"), Ok(Value::Number(1.0)));
}

#[test]
fn unknown_names_are_undefined() {
    let mut component = component();

    assert_eq!(
        component.read("missing"),
        Err(Error::Undefined("missing".to_owned()))
    );
    assert_eq!(
        failure(&mut component, "missing = 1"),
        Error::Undefined("missing".to_owned())
    );
}

#[test]
fn declaring_a_property_twice_is_rejected() {
    let mut component = component();
    component
        .declare_property(id("checked"), Value::Bool(false))
        .unwrap();

    assert_eq!(
        component.declare_property(id("checked"), Value::Bool(true)),
        Err(Error::Redeclared("checked".to_owned()))
    );
}

#[test]
fn evaluation_resolves_identifiers() {
    let mut component = component();
    component
        .declare("count", Value::Number(2.0), false)
        .unwrap();
    component
        .declare_property(id("label"), Value::String("items".into()))
        .unwrap();

    assert_eq!(
        evaluate(&mut component, "count * 3"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        evaluate(&mut component, "label + \"s\""),
        Ok(Value::String("itemss".into()))
    );
}

#[test]
fn each_scope_is_stored_on_the_component() {
    let mut component = component();
    assert!(component.scope().get::<i32>().is_none());

    component.set_scope(Scope::each(7_i32));
    assert_eq!(component.scope().get::<i32>(), Some(&7));
    assert!(component.scope().get::<String>().is_none());

    component.set_scope(Scope::root());
    assert!(component.scope().get::<i32>().is_none());
}

#[test]
fn reading_a_property_tracks_it() {
    let component = Rc::new(RefCell::new(component()));
    component
        .borrow_mut()
        .declare_property(id("checked"), Value::Bool(false))
        .unwrap();

    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&seen);
    let source = Rc::clone(&component);
    let _effect = Effect::new(move || {
        let value = eval(
            &mut source.borrow_mut(),
            &parse_expression("checked").unwrap(),
        );
        log.borrow_mut().push(value.unwrap());
    });

    assert_eq!(*seen.borrow(), vec![Value::Bool(false)]);

    component
        .borrow_mut()
        .assign("checked", Value::Bool(true))
        .unwrap();
    flush();

    assert_eq!(*seen.borrow(), vec![Value::Bool(false), Value::Bool(true)]);
}

#[test]
fn an_each_scope_can_be_replaced_without_leaking_into_reads() {
    let mut component = component();
    component.set_scope(Scope::each(String::from("item")));
    assert_eq!(
        component.scope().get::<String>().map(String::as_str),
        Some("item")
    );

    component.set_scope(Scope::root());
    assert_eq!(
        component.read("item"),
        Err(Error::Undefined("item".to_owned()))
    );
}
