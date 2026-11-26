use super::*;
fn event_host(events: Vec<ComponentEvent>) -> EventHost {
    EventHost { events }
}

fn event(
    call: EventCall,
    params: &'static [PropType],
    component: &ComponentHandle,
) -> ComponentEvent {
    ComponentEvent {
        name: "click",
        call,
        params,
        component: Rc::clone(component),
    }
}

#[test]
fn event_host_rejects_an_unknown_event() {
    let component = Rc::new(RefCell::new(Component::new()));
    let mut host = event_host(vec![event(noop_event, &[], &component)]);
    let error = host.call("hover", &[]).unwrap_err();
    assert!(matches!(error, ScriptError::UnknownFunction(ref name) if name == "hover"));
}

#[test]
fn event_host_checks_the_arity_before_the_call() {
    let component = Rc::new(RefCell::new(Component::new()));
    let mut host = event_host(vec![event(noop_event, &[PropType::Bool], &component)]);
    let error = host.call("click", &[]).unwrap_err();
    assert!(matches!(
        error,
        ScriptError::FunctionArity {
            expected: 1,
            found: 0,
            ..
        }
    ));
}

#[test]
fn event_host_checks_the_argument_types() {
    let component = Rc::new(RefCell::new(Component::new()));
    let mut host = event_host(vec![event(noop_event, &[PropType::Bool], &component)]);
    let error = host.call("click", &[Value::Number(1.0)]).unwrap_err();
    assert!(matches!(error, ScriptError::EventArgument { .. }));
}

#[test]
fn event_host_runs_the_component_function() {
    let component = Rc::new(RefCell::new(Component::new()));
    component.borrow_mut().set_dispatch(noop);
    let mut host = event_host(vec![event(noop_event, &[], &component)]);
    assert_eq!(host.call("click", &[]), Ok(Value::Null));
}

#[test]
fn event_host_queues_the_body_and_leaves_errors_to_the_delivery() {
    let component = Rc::new(RefCell::new(Component::new()));
    component.borrow_mut().set_dispatch(failing);
    let mut host = event_host(vec![event(failing_event, &[], &component)]);
    assert_eq!(host.call("click", &[]), Ok(Value::Null));
    let PendingEvent {
        component,
        call,
        arguments,
    } = take_component();
    assert_eq!(
        call(&mut component.borrow_mut(), &arguments)
            .unwrap_err()
            .reason(),
        ScriptError::DivisionByZero.reason()
    );
}

#[test]
fn a_class_binding_applies_the_named_class() {
    let mut declared = component();
    declared
        .declare("title", Value::String("wide".into()), false)
        .unwrap();
    let component = Rc::new(RefCell::new(declared));
    let (ids, tree) = six_widgets();
    let binding = Binding {
        widget: ids[0],
        target: BindingTarget::Class {
            table: class_table,
            expression: expr!("title"),
        },
        scope: Scope::default(),
        component: Rc::clone(&component),
    };

    let _bindings = Bindings::new(&[binding], &tree, &Textures::default());
    let mut engine = brul_gui::TextEngine::new();
    tree.borrow_mut()
        .layout(
            &mut engine,
            brul_gui::Size {
                width: 200,
                height: 200,
            },
            1.0,
        )
        .expect("layout");
    let mut tree = tree.borrow_mut();
    let node = tree.snapshot().expect("snapshot").node(ids[0]).cloned();
    assert_eq!(node.expect("node").width, 40);
}

#[test]
fn a_class_binding_ignores_an_unknown_name() {
    let mut declared = component();
    declared
        .declare("title", Value::String("missing".into()), false)
        .unwrap();
    let component = Rc::new(RefCell::new(declared));
    let (ids, _tree) = six_widgets();
    let binding = Binding {
        widget: ids[0],
        target: BindingTarget::Class {
            table: class_table,
            expression: expr!("title"),
        },
        scope: Scope::default(),
        component,
    };

    assert!(matches!(
        evaluate(&binding, &Textures::default()),
        Write::Class(None)
    ));
}

fn class_table(name: &str) -> Option<brul_gui::Class> {
    (name == "wide").then(|| brul_gui::Class {
        layout: brul_gui::LayoutPatch {
            width: Some(brul_gui::Dimension::Px(40)),
            ..brul_gui::LayoutPatch::default()
        },
        ..brul_gui::Class::default()
    })
}
