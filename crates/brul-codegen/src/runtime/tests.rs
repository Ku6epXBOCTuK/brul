use bindings::{image_binding, six_widgets};
#[path = "tests/macros.rs"]
#[macro_use]
mod macros;
#[path = "tests/bindings.rs"]
mod bindings;
#[path = "tests/events.rs"]
mod events;
#[path = "tests/primitives.rs"]
mod primitives;
use super::binding::offset_value;
use super::event::EventHost;
use super::*;
use brul_gui::{
    Dimension, Image, KeyModifiers, KeyPress, LayoutStyle, Size, TextEngine, Texture, Widget,
    WidgetId, WidgetTree,
};
use brul_script::{
    Component, Dyn, Error as ScriptError, EventCall, Host, Scope, Value, noop_event,
};
use brul_syntax::TypeName;
use brul_syntax::markup::PropType;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn take_component() -> PendingEvent {
    match take_pending().expect("queued") {
        Pending::Component(event) => event,
        Pending::Ui(_) | Pending::Raw(_) => panic!("a component event was expected"),
    }
}

const NAMES: &[&str] = &["name", "visible", "x", "y", "title", "on", "off", "icon"];

fn component() -> Component {
    let mut component = Component::new();
    component.set_names(NAMES);
    component
}

fn noop(_: &mut Component, _: &str, _: Vec<Dyn>) -> Result<Dyn, ScriptError> {
    Ok(Dyn::null())
}

fn failing_event(_: &mut Component, _: &[Dyn]) -> Result<Dyn, ScriptError> {
    Err(ScriptError::DivisionByZero)
}

fn failing(_: &mut Component, _: &str, _: Vec<Dyn>) -> Result<Dyn, ScriptError> {
    Err(ScriptError::DivisionByZero)
}

fn declared(name: &str, value: Value) -> Component {
    let mut component = component();
    component.declare(name, value, false).unwrap();
    component
}

#[test]
fn text_binding_follows_the_component() {
    let mut component = component();
    component
        .declare("name", Value::String("hello".into()), false)
        .unwrap();
    let widget = Widget::text("");
    let id = widget.id();
    let tree = Rc::new(RefCell::new(WidgetTree::new(widget)));
    let component = Rc::new(RefCell::new(component));
    let bindings = [Binding {
        widget: id,
        target: BindingTarget::Text(text!("name")),
        scope: Scope::default(),
        component: Rc::clone(&component),
    }];

    let _bindings = Bindings::new(&bindings, &tree, &Textures::default());
    assert!(!tree.borrow_mut().set_text(id, "hello".to_owned()));

    component
        .borrow_mut()
        .assign("name", Value::String("bye".into()))
        .unwrap();
    brul_reactive::flush();
    assert!(!tree.borrow_mut().set_text(id, "bye".to_owned()));
}

fn reentrant(component: &mut Component) -> Result<Value, ScriptError> {
    component.call("go", &[])
}

struct TreeProbe {
    tree: Rc<RefCell<WidgetTree>>,
    free: Rc<Cell<bool>>,
}

impl Host for TreeProbe {
    fn call(&mut self, _name: &str, _arguments: &[Value]) -> Result<Value, ScriptError> {
        self.free.set(self.tree.try_borrow_mut().is_ok());
        Ok(Value::Bool(self.free.get()))
    }
}

fn probe(component: &mut Component) -> Result<Value, ScriptError> {
    component.call("probe", &[])
}

fn probe_flag(component: &mut Component) -> Result<bool, ScriptError> {
    (&probe(component)?).try_into()
}

#[test]
fn a_binding_expression_reaches_the_tree_while_it_runs() {
    let widget = Widget::text("");
    let id = widget.id();
    let tree = Rc::new(RefCell::new(WidgetTree::new(widget)));
    let free = Rc::new(Cell::new(false));
    let handle = Rc::new(RefCell::new(Component::new()));
    handle.borrow_mut().set_host(TreeProbe {
        tree: Rc::clone(&tree),
        free: Rc::clone(&free),
    });
    let bindings = [Binding {
        widget: id,
        target: BindingTarget::Visible(probe_flag),
        scope: Scope::default(),
        component: Rc::clone(&handle),
    }];

    let _bindings = Bindings::new(&bindings, &tree, &Textures::default());
    assert!(free.get());
    assert!(!tree.borrow_mut().set_visible(id, true));
}

#[test]
fn an_event_raised_from_an_expression_is_delivered_after_the_expression_returns() {
    let handle = Rc::new(RefCell::new(Component::new()));
    handle.borrow_mut().set_dispatch(noop);
    handle.borrow_mut().set_host(EventHost {
        events: vec![ComponentEvent {
            name: "go",
            call: noop_event,
            params: &[],
            component: Rc::clone(&handle),
        }],
    });

    assert_eq!(reentrant(&mut handle.borrow_mut()), Ok(Value::Null));

    let PendingEvent {
        component,
        call,
        arguments,
    } = take_component();
    assert!(call(&mut component.borrow_mut(), &arguments).is_ok());
}

#[test]
fn text_returns_the_string_and_names_the_actual_type() {
    let mut component = declared("name", Value::String("hello".into()));
    assert_eq!(
        text(&mut component, text!("name"), "name").unwrap(),
        "hello"
    );

    let mut component = declared("name", Value::Number(3.0));
    let error = text(&mut component, text!("name"), "name").unwrap_err();
    assert!(matches!(
        error,
        ViewError::Expected { found, .. } if found == Value::Number(3.0).type_name()
    ));
}

#[test]
fn boolean_returns_the_flag_and_names_the_parameter() {
    let name = "visible";
    let mut component = declared(name, Value::Bool(true));
    assert!(boolean(&mut component, flag!("visible"), name).unwrap());

    let mut component = declared(name, Value::String("yes".into()));
    let error = boolean(&mut component, flag!("visible"), name).unwrap_err();
    assert!(
        matches!(error, ViewError::Expected { target, .. } if target == format!("parameter '{name}'"))
    );
}

#[test]
fn an_absent_offset_is_zero() {
    let mut component = Component::new();
    assert_eq!(offset(&mut component, None, "x").unwrap(), 0);
}

#[test]
fn offset_rounds_fractions_and_rejects_the_rest() {
    let name = "x";
    let mut component = declared(name, Value::Number(-7.0));
    assert_eq!(
        offset(&mut component, Some(number!("x")), name).unwrap(),
        -7
    );

    let mut component = declared(name, Value::Number(2.5));
    assert_eq!(offset(&mut component, Some(number!("x")), name).unwrap(), 3);

    for value in [f64::NAN, f64::from(i32::MAX) + 1.0] {
        let mut component = declared(name, Value::Number(value));
        assert!(offset(&mut component, Some(number!("x")), name).is_err());
    }
}

#[test]
fn an_offset_binding_rounds_and_ignores_what_it_has_no_number_for() {
    let none = RefCell::new(Component::new());
    assert_eq!(offset_value(&none, None), Some(0));

    let fractional = RefCell::new(declared("x", Value::Number(1.5)));
    assert_eq!(offset_value(&fractional, Some(number!("x"))), Some(2));

    let missing = RefCell::new(Component::new());
    assert_eq!(offset_value(&missing, Some(number!("x"))), None);

    let text = RefCell::new(declared("x", Value::String("4".into())));
    assert_eq!(offset_value(&text, Some(number!("x"))), None);

    let whole = RefCell::new(declared("x", Value::Number(4.0)));
    assert_eq!(offset_value(&whole, Some(number!("x"))), Some(4));
}

#[test]
fn event_names_round_trip_and_stay_distinct() {
    let events = EventKind::ALL;
    for event in events {
        assert_eq!(event.name().parse::<EventKind>().unwrap(), *event);
    }
    let mut names: Vec<_> = events.iter().map(|event| event.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), events.len());
    assert!("hover".parse::<EventKind>().is_err());
}
