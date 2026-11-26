use std::cell::RefCell;
use std::rc::Rc;

use brul::{Component, ComponentHandle, Error, Value};

thread_local! {
    static SEEN: RefCell<Option<(f64, String)>> = const { RefCell::new(None) };
}

#[brul::widget(name = "Gauge", events(changed: [Number]))]
pub fn gauge(
    _theme: Option<&str>,
    _build: &mut brul::Build,
    component: &ComponentHandle,
    value: f64,
    label: String,
) -> Result<brul::Built, brul::ViewError> {
    SEEN.with(|seen| *seen.borrow_mut() = Some((value, label.clone())));
    component
        .borrow_mut()
        .call("changed", &[Value::Number(value + 1.0)])
        .expect("the changed event is declared");
    assert!(
        component
            .borrow_mut()
            .call("changed", &[Value::String("bad".into())])
            .is_err()
    );
    Ok(brul::Built {
        widget: brul::gui::Widget::text(format!("{label}: {value}")),
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

const NAMES: &[&str] = &["value", "label"];

fn number(component: &mut Component) -> Result<f64, Error> {
    component.seed("value", brul::ast::Position { line: 0, column: 0 })
}

fn text(component: &mut Component) -> Result<String, Error> {
    component.seed("label", brul::ast::Position { line: 0, column: 0 })
}

#[test]
fn a_registered_widget_receives_its_props_and_declares_its_events() {
    let parent: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    parent.borrow_mut().set_names(NAMES);
    parent
        .borrow_mut()
        .declare("value", Value::Number(42.0), false)
        .expect("value declared");
    parent
        .borrow_mut()
        .declare("label", Value::String("load".into()), false)
        .expect("label declared");
    let mut build = brul::Build::new(brul::Services::default());
    let props = [
        brul::PropArgument {
            name: "value",
            ty: brul::PropType::Number,
            expression: brul::PropExpression::Number(number),
        },
        brul::PropArgument {
            name: "label",
            ty: brul::PropType::String,
            expression: brul::PropExpression::Text(text),
        },
    ];
    let events = [brul::ComponentEvent {
        name: "changed",
        call: brul::noop_event,
        params: &[brul::PropType::Number],
        component: Rc::clone(&parent),
    }];

    let built = brul::instance(
        &mut build,
        &parent,
        brul::Instance {
            theme: None,
            view: __brul_widget_gauge,
            script: None,
            name: "Gauge",
            names: NAMES,
            props: &props,
            defaults: &[],
            events: &events,
            slots: &[],
        },
    )
    .expect("the widget builds");

    assert!(built.bindings.is_empty());
    let seen = SEEN.with(|seen| seen.borrow().clone());
    assert_eq!(seen, Some((42.0, "load".to_owned())));
}
