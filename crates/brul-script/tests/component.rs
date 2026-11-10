use std::cell::RefCell;
use std::rc::Rc;

use brul_reactive::{Effect, flush};
use brul_script::{Component, Error, Value, Variable, eval};
use brul_syntax::parser::parse_expression;

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
    let mut component = Component::new();
    component
        .declare_property("checked", Value::Bool(false))
        .unwrap();

    assert_eq!(component.read("checked"), Ok(Value::Bool(false)));
}

#[test]
fn variable_shadows_a_property() {
    let mut component = Component::new();
    component
        .declare_property("count", Value::Number(1.0))
        .unwrap();
    component
        .declare("count", Value::Number(2.0), false)
        .unwrap();

    assert_eq!(component.read("count"), Ok(Value::Number(2.0)));
}

#[test]
fn assignment_writes_a_property() {
    let mut component = Component::new();
    component
        .declare_property("checked", Value::Bool(false))
        .unwrap();

    assert_eq!(
        evaluate(&mut component, "checked = true"),
        Ok(Value::Bool(true))
    );
    assert_eq!(component.read("checked"), Ok(Value::Bool(true)));
}

#[test]
fn assignment_writes_a_let_variable() {
    let mut component = Component::new();
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
    let mut component = Component::new();
    component
        .declare("count", Value::Number(1.0), false)
        .unwrap();
    component
        .declare_property("offset", Value::Number(4.0))
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
    let mut component = Component::new();
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
    let mut component = Component::new();

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
    let mut component = Component::new();
    component
        .declare_property("checked", Value::Bool(false))
        .unwrap();

    assert_eq!(
        component.declare_property("checked", Value::Bool(true)),
        Err(Error::Redeclared("checked".to_owned()))
    );
}

#[test]
fn evaluation_resolves_identifiers() {
    let mut component = Component::new();
    component
        .declare("count", Value::Number(2.0), false)
        .unwrap();
    component
        .declare_property("label", Value::String("items".to_owned()))
        .unwrap();

    assert_eq!(
        evaluate(&mut component, "count * 3"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        evaluate(&mut component, "label + \"s\""),
        Ok(Value::String("itemss".to_owned()))
    );
}

#[test]
fn build_scopes_share_their_snapshot_until_the_stack_changes() {
    let mut component = Component::new();
    component.push_build_scope(&[Variable {
        name: "item",
        value: Value::Number(1.0),
    }]);

    let first = component.build_scopes();
    let second = component.build_scopes();
    assert!(Rc::ptr_eq(&first, &second));
    assert_eq!(first.len(), 1);

    component.push_build_scope(&[Variable {
        name: "row",
        value: Value::Number(2.0),
    }]);
    let deeper = component.build_scopes();
    assert!(!Rc::ptr_eq(&first, &deeper));
    assert_eq!(deeper.len(), 2);

    component.pop_build_scope();
    component.pop_build_scope();
    let empty = component.build_scopes();
    assert!(empty.is_empty());
    assert!(!Rc::ptr_eq(&deeper, &empty));
}

#[test]
fn reading_a_property_tracks_it() {
    let component = Rc::new(RefCell::new(Component::new()));
    component
        .borrow_mut()
        .declare_property("checked", Value::Bool(false))
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
