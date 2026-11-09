use std::cell::RefCell;
use std::rc::Rc;

use bropicker::generated;
use brul::{Built, Component, ComponentHandle, HandlerEvent, Value};
use brul_gui::WidgetKind;

fn seeded() -> ComponentHandle {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut state = component.borrow_mut();
    state
        .declare(
            "url",
            Value::String("https://example.com".to_owned()),
            false,
        )
        .expect("url seed");
    state
        .declare(
            "browsers",
            Value::Array(vec![
                Value::String("first".to_owned()),
                Value::String("second".to_owned()),
            ]),
            true,
        )
        .expect("browsers seed");
    drop(state);
    component
}

#[test]
fn window_config_comes_from_the_markup() {
    let config = generated::window();
    assert_eq!(config.title, "bropicker");
    assert_eq!(config.width, 488);
    assert_eq!(config.height, 800);
}

#[test]
fn markup_script_block_is_emitted() {
    let script = generated::script().expect("script block");
    assert!(
        script
            .functions
            .iter()
            .any(|function| function.name == "handleBump")
    );
}

#[test]
fn view_builds_branches_each_and_the_imported_component() {
    let component = seeded();
    let built = generated::view(None, &component).expect("generated view");
    let Built {
        widget,
        bindings,
        handlers,
        structures,
        props,
    } = built;

    assert_eq!(structures.len(), 2);
    assert!(!structures[0].each);
    assert!(structures[1].each);

    assert_eq!(handlers.len(), 1);
    assert_eq!(handlers[0].event, HandlerEvent::Click);
    assert_eq!(handlers[0].function, "bump");

    assert_eq!(props.len(), 1);
    assert_eq!(props[0].name, "label");

    assert_eq!(bindings.len(), 4);

    let mut children = widget.into_children().into_iter();
    let branch = children.next().expect("if branch");
    let rows = children.next().expect("each rows");
    let badge = children.next().expect("imported component");
    assert!(children.next().is_none());

    assert_eq!(branch.into_children().len(), 1);
    assert_eq!(rows.into_children().len(), 2);
    assert_eq!(badge.kind(), WidgetKind::Text);
}

#[test]
fn structure_values_read_the_declared_seeds() {
    let component = seeded();
    let built = generated::view(None, &component).expect("generated view");

    assert_eq!(
        (built.structures[0].value)(&component),
        Some(Value::Bool(true))
    );
    assert_eq!(
        (built.structures[1].value)(&component),
        Some(Value::Array(vec![
            Value::String("first".to_owned()),
            Value::String("second".to_owned()),
        ]))
    );
}

#[test]
fn structure_rebuild_switches_the_branch() {
    let component = seeded();
    let built = generated::view(None, &component).expect("generated view");

    let (rebuilt, rows) = (built.structures[0].rebuild)(None, &component).expect("rebuild");
    assert_eq!(rebuilt.widget.into_children().len(), 1);
    assert!(rows.is_empty());

    component
        .borrow_mut()
        .assign("url", Value::String(String::new()))
        .expect("clear url");
    let (rebuilt, _) = (built.structures[0].rebuild)(None, &component).expect("rebuild");
    let mut children = rebuilt.widget.into_children();
    assert_eq!(children.len(), 1);
    assert_eq!(children.remove(0).kind(), WidgetKind::Text);
}

#[test]
fn unknown_theme_is_rejected() {
    let error = match generated::view(Some("sepia"), &seeded()) {
        Ok(_) => panic!("unknown theme was accepted"),
        Err(error) => error,
    };
    assert!(error.contains("unknown theme 'sepia'"), "{error}");
}
