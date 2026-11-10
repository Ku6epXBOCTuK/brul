use std::cell::RefCell;
use std::rc::Rc;

use bropicker::generated;
use brul::{Built, Component, ComponentHandle, HandlerEvent, Value};
use brul_gui::WidgetKind;

fn browser(name: &str, is_default: bool) -> Value {
    Value::Object(vec![
        ("icon".to_owned(), Value::String(name[..1].to_uppercase())),
        ("name".to_owned(), Value::String(name.to_owned())),
        ("path".to_owned(), Value::String(format!("C:/{name}"))),
        ("isDefault".to_owned(), Value::Bool(is_default)),
    ])
}

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
            Value::Array(vec![browser("first", true), browser("second", false)]),
            true,
        )
        .expect("browsers seed");
    state
        .declare("selected", Value::Number(0.0), true)
        .expect("selected seed");
    state
        .declare("remember", Value::Bool(true), true)
        .expect("remember seed");
    state
        .declare("alwaysAsk", Value::Bool(false), true)
        .expect("alwaysAsk seed");
    drop(state);
    let script = generated::script().expect("script block");
    brul::execute_program(&mut component.borrow_mut(), &script).expect("script");
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
    for name in ["launch", "toggleRemember", "toggleAlwaysAsk", "handleKey"] {
        assert!(
            script
                .functions
                .iter()
                .any(|function| function.name == name),
            "{name} is missing"
        );
    }
}

#[test]
fn view_builds_the_picker_layout() {
    let component = seeded();
    let built = generated::view(None, &component).expect("generated view");
    let Built {
        widget,
        bindings,
        handlers,
        structures,
        props,
    } = built;

    assert_eq!(structures.len(), 3);
    assert_eq!(
        structures.iter().filter(|structure| structure.each).count(),
        1
    );

    assert_eq!(handlers.len(), 5);
    let key = handlers
        .iter()
        .find(|handler| handler.event == HandlerEvent::Key)
        .expect("key handler");
    assert_eq!(key.widget, None);
    assert_eq!(key.function, "handleKey");

    assert_eq!(bindings.len(), 13);
    assert_eq!(props.len(), 16);

    let mut children: Vec<_> = widget.into_children();
    assert_eq!(children.len(), 3);
    assert!(
        children
            .iter()
            .all(|child| child.kind() == WidgetKind::View)
    );
    let list = children.remove(1);
    let rows: Vec<_> = list.into_children();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows.into_iter()
            .next()
            .expect("each rows")
            .into_children()
            .len(),
        2
    );
}

#[test]
fn structure_values_read_the_declared_seeds() {
    let component = seeded();
    let built = generated::view(None, &component).expect("generated view");

    let each = built
        .structures
        .iter()
        .find(|structure| structure.each)
        .expect("each structure");
    let value = (each.value)(&mut component.borrow_mut()).expect("each value");
    let Value::Array(items) = value else {
        panic!("each value is not an array");
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[1], browser("second", false));
}

#[test]
fn structure_rebuild_rebuilds_rows() {
    let component = seeded();
    let built = generated::view(None, &component).expect("generated view");

    let each = built
        .structures
        .iter()
        .find(|structure| structure.each)
        .expect("each structure");
    let (rebuilt, rows) = (each.rebuild)(None, &component).expect("rebuild");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].key, Some(Value::String("first".to_owned())));
    assert_eq!(rows[0].count, 1);
    assert_eq!(rebuilt.widget.into_children().len(), 2);
}

#[test]
fn unknown_theme_is_rejected() {
    let error = match generated::view(Some("sepia"), &seeded()) {
        Ok(_) => panic!("unknown theme was accepted"),
        Err(error) => error,
    };
    assert!(error.contains("unknown theme 'sepia'"), "{error}");
}
