use std::cell::RefCell;
use std::rc::Rc;

use brul_reactive::{Effect, flush};
use brul_script::eval::apply_binary;
use brul_script::{Component, Error, Scope, Value, VariableId};
use brul_syntax::ast::BinaryOp;

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

fn number(value: f64) -> Value {
    Value::Number(value)
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
    component.declare("count", number(2.0), false).unwrap();
    component
        .declare_property(id("checked"), Value::Bool(false))
        .unwrap();

    assert_eq!(component.read_id(id("count")), Ok(number(2.0)));
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
        .declare_property(id("count"), number(1.0))
        .unwrap();
    component.declare("count", number(2.0), false).unwrap();

    assert_eq!(component.read("count"), Ok(number(2.0)));
}

#[test]
fn assignment_writes_a_property() {
    let mut component = component();
    component
        .declare_property(id("checked"), Value::Bool(false))
        .unwrap();

    assert_eq!(component.assign("checked", Value::Bool(true)), Ok(()));
    assert_eq!(component.read("checked"), Ok(Value::Bool(true)));
}

#[test]
fn assignment_writes_a_let_variable() {
    let mut component = component();
    component.declare("count", number(1.0), false).unwrap();

    let value = apply_binary(BinaryOp::Add, component.read("count").unwrap(), number(2.0)).unwrap();
    assert_eq!(component.assign("count", value.clone()), Ok(()));
    assert_eq!(component.read("count"), Ok(number(3.0)));
}

#[test]
fn compound_assignment_reads_the_current_value() {
    let mut component = component();
    component.declare("count", number(1.0), false).unwrap();
    component
        .declare_property(id("offset"), number(4.0))
        .unwrap();

    let value = apply_binary(BinaryOp::Add, component.read("count").unwrap(), number(2.0)).unwrap();
    component.assign("count", value).unwrap();
    let value = apply_binary(
        BinaryOp::Sub,
        component.read("offset").unwrap(),
        number(1.0),
    )
    .unwrap();
    component.assign("offset", value).unwrap();

    assert_eq!(component.read("count"), Ok(number(3.0)));
    assert_eq!(component.read("offset"), Ok(number(3.0)));
}

#[test]
fn assignment_respects_constants() {
    let mut component = component();
    component.declare("limit", number(1.0), true).unwrap();

    assert_eq!(
        component.assign("limit", number(2.0)),
        Err(Error::Constant("limit".to_owned()))
    );
    assert_eq!(component.read("limit"), Ok(number(1.0)));
}

#[test]
fn unknown_names_are_undefined() {
    let mut component = component();

    assert_eq!(
        component.read("missing"),
        Err(Error::Undefined("missing".to_owned()))
    );
    assert_eq!(
        component.assign("missing", number(1.0)),
        Err(Error::Undefined("missing".to_owned()))
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
    component.declare("count", number(2.0), false).unwrap();
    component
        .declare_property(id("label"), Value::String("items".into()))
        .unwrap();

    let value = apply_binary(BinaryOp::Mul, component.read("count").unwrap(), number(3.0)).unwrap();
    assert_eq!(value, number(6.0));
    let value = apply_binary(
        BinaryOp::Add,
        component.read("label").unwrap(),
        Value::String("s".into()),
    )
    .unwrap();
    assert_eq!(value, Value::String("itemss".into()));
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
        let value = source.borrow_mut().read("checked").unwrap();
        log.borrow_mut().push(value);
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
