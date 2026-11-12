use std::cell::RefCell;
use std::rc::Rc;

use bropicker::generated;
use brul::{Built, Component, ComponentHandle, HandlerEvent, Value};
use brul_gui::WidgetKind;

fn browser(name: &str, is_default: bool) -> Value {
    Value::Object(
        vec![
            ("icon".to_owned(), Value::Image("C:/icon.png".into())),
            ("emoji".to_owned(), Value::String("".into())),
            ("masked".to_owned(), Value::Bool(true)),
            ("name".to_owned(), Value::String(name.into())),
            (
                "path".to_owned(),
                Value::String(format!("C:/{name}").into()),
            ),
            ("isDefault".to_owned(), Value::Bool(is_default)),
        ]
        .into(),
    )
}

fn seeded() -> ComponentHandle {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    component.borrow_mut().set_names(generated::names());
    let mut state = component.borrow_mut();
    state
        .declare("url", Value::String("https://example.com".into()), false)
        .expect("url seed");
    state
        .declare("domain", Value::String("example.com".into()), false)
        .expect("domain seed");
    state
        .declare(
            "browsers",
            Value::Array(vec![browser("first", true), browser("second", false)].into()),
            false,
        )
        .expect("browsers seed");
    state
        .declare(
            "remembered",
            Value::Array(
                vec![Value::Object(
                    vec![
                        ("domain".to_owned(), Value::String("example.com".into())),
                        ("browser".to_owned(), Value::String("first".into())),
                    ]
                    .into(),
                )]
                .into(),
            ),
            false,
        )
        .expect("remembered seed");
    state
        .declare("selected", Value::Number(0.0), true)
        .expect("selected seed");
    state
        .declare("remember", Value::Bool(true), true)
        .expect("remember seed");
    state
        .declare("alwaysAsk", Value::Bool(false), true)
        .expect("alwaysAsk seed");
    state
        .declare("dark", Value::Bool(true), true)
        .expect("dark seed");
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
    assert!(config.frameless);
    assert!(config.transparent);
}

#[test]
fn the_window_table_lists_every_declared_window() {
    let windows = generated::windows();
    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0].name, "Picker");
    assert_eq!(windows[0].config.title, "bropicker");
    assert!(windows[0].config.frameless);
    assert_eq!(windows[1].name, "Settings");
    assert_eq!(windows[1].config.title, "bropicker - settings");
    assert_eq!(windows[1].config.width, 760);
    assert_eq!(windows[1].config.height, 560);
    assert!(!windows[1].config.frameless);
}

#[test]
fn the_settings_window_has_two_lists_and_its_own_key_handler() {
    let component = seeded();
    let built = (generated::windows()[1].view)(None, &component).expect("settings view");

    let key = built
        .handlers
        .iter()
        .find(|handler| handler.event == HandlerEvent::Key)
        .expect("key handler");
    assert_eq!(key.function, "handleSettingsKey");
    assert_eq!(
        built
            .structures
            .iter()
            .filter(|structure| structure.each)
            .count(),
        2
    );
}

#[test]
fn both_windows_read_the_same_browsers_signal() {
    let component = seeded();
    let windows = generated::windows();
    let picker = (windows[0].view)(None, &component).expect("picker view");
    let settings = (windows[1].view)(None, &component).expect("settings view");
    let each = |built: &Built| {
        built
            .structures
            .iter()
            .find(|structure| structure.each)
            .expect("each structure")
            .clone()
    };

    let value = (each(&settings).value)(&mut component.borrow_mut()).expect("value");
    assert!(matches!(value, Value::Array(items) if items.len() == 2));

    component
        .borrow_mut()
        .assign(
            "browsers",
            Value::Array(vec![browser("first", true)].into()),
        )
        .expect("assign");

    let value = (each(&picker).value)(&mut component.borrow_mut()).expect("value");
    assert!(matches!(value, Value::Array(items) if items.len() == 1));
}

#[test]
fn markup_script_block_is_emitted() {
    let script = generated::script().expect("script block");
    for name in [
        "launch",
        "openSelected",
        "toggleTheme",
        "toggleRemember",
        "toggleAlwaysAsk",
        "handleKey",
    ] {
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

    assert_eq!(structures.len(), 21);
    assert_eq!(
        structures.iter().filter(|structure| structure.each).count(),
        1
    );

    assert_eq!(handlers.len(), 8);
    let key = handlers
        .iter()
        .find(|handler| handler.event == HandlerEvent::Key)
        .expect("key handler");
    assert_eq!(key.widget, None);
    assert_eq!(key.function, "handleKey");

    assert_eq!(bindings.len(), 15);
    assert_eq!(props.len(), 18);

    let mut root: Vec<_> = widget.into_children();
    assert_eq!(root.len(), 1);
    let card = root.remove(0);
    assert_eq!(card.kind(), WidgetKind::View);
    let mut children: Vec<_> = card.into_children();
    assert_eq!(children.len(), 5);
    assert!(
        children
            .iter()
            .all(|child| child.kind() == WidgetKind::View)
    );
    let list = children.remove(2);
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
    assert_eq!(rows[0].key, Some(Value::String("first".into())));
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
