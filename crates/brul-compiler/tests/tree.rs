use brul_compiler::build_tree;
use brul_gui::{Size, TextEngine, Widget, WidgetKind, WidgetSnapshot, WidgetTree};
use brul_script::{Component, execute_program};
use brul_syntax::markup::{Declaration, Element, Member, parse_file};
use brul_syntax::script;
use brul_syntax::style::Stylesheet;

fn empty_stylesheet() -> Stylesheet {
    Stylesheet {
        themes: Vec::new(),
        rules: Vec::new(),
    }
}

fn component(source: &str) -> Component {
    let program = script::parse(source).expect("script parses");
    let mut component = Component::new();
    execute_program(&mut component, &program).expect("script runs");
    component
}

fn root(source: &str) -> Element {
    let file = parse_file(source).expect("markup parses");
    let Declaration::Window(window) = &file.declarations[0] else {
        panic!("expected a window");
    };
    window
        .members
        .iter()
        .find_map(|member| match member {
            Member::Element(element) => Some(element.clone()),
            _ => None,
        })
        .expect("window has a root element")
}

fn snapshot(widget: Widget) -> WidgetSnapshot {
    let mut tree = WidgetTree::new(widget);
    let mut engine = TextEngine::new();
    tree.layout(
        &mut engine,
        Size {
            width: 300,
            height: 300,
        },
        1.0,
    )
    .expect("layout");
    tree.snapshot().expect("snapshot").clone()
}

#[test]
fn static_tree_builds_containers_and_text() {
    let element = root(
        r#"
        window Picker (title: "x") {
            Column {
                Row {
                    Text { "Hello" }
                }
                View {
                }
            }
        }
        "#,
    );
    let widget = build_tree(&element, &empty_stylesheet(), None, &mut Component::new())
        .expect("tree builds");
    let snapshot = snapshot(widget);
    let kinds: Vec<WidgetKind> = snapshot.nodes.iter().map(|node| node.kind).collect();

    assert_eq!(
        kinds,
        vec![
            WidgetKind::View,
            WidgetKind::View,
            WidgetKind::Text,
            WidgetKind::View
        ]
    );
    let text = snapshot
        .nodes
        .iter()
        .find(|node| node.kind == WidgetKind::Text)
        .expect("text node");
    assert!(!text.text.as_ref().expect("text layout").glyphs.is_empty());
}

#[test]
fn unknown_widget_reports_its_position() {
    let element = root(
        r#"
        window Picker (title: "x") {
            Column {
                Toolbar { }
            }
        }
        "#,
    );
    let error = build_tree(&element, &empty_stylesheet(), None, &mut Component::new())
        .err()
        .expect("unknown widget");

    assert_eq!(error.message, "unknown widget 'Toolbar'");
    assert_eq!((error.line, error.column), (4, 17));
}

#[test]
fn image_is_not_supported_yet() {
    let element = root(
        r#"
        window Picker (title: "x") {
            Image { "icon.png" }
        }
        "#,
    );
    let error = build_tree(&element, &empty_stylesheet(), None, &mut Component::new())
        .err()
        .expect("image is deferred");

    assert_eq!(error.message, "widget 'Image' is not supported yet");
    assert_eq!((error.line, error.column), (3, 13));
}

#[test]
fn text_content_must_be_a_string() {
    let element = root(
        r#"
        window Picker (title: "x") {
            Text { 1 + 2 }
        }
        "#,
    );
    let error = build_tree(&element, &empty_stylesheet(), None, &mut Component::new())
        .err()
        .expect("expression content");

    assert_eq!(error.message, "Text content must be a string, got Number");
    assert_eq!((error.line, error.column), (3, 20));
}

#[test]
fn text_content_reads_a_script_variable() {
    let element = root(
        r#"
        window Picker (title: "x") {
            Text { title }
        }
        "#,
    );
    let mut component = component("let title = \"Browser\";");
    let widget =
        build_tree(&element, &empty_stylesheet(), None, &mut component).expect("tree builds");
    let snapshot = snapshot(widget);
    let text = snapshot
        .nodes
        .iter()
        .find(|node| node.kind == WidgetKind::Text)
        .expect("text node");

    assert!(!text.text.as_ref().expect("text layout").glyphs.is_empty());
}

#[test]
fn state_parameters_are_evaluated() {
    let element = root(
        r#"
        window Picker (title: "x") {
            Column (checked: on, selected: on, disabled: on, visible: on) { }
        }
        "#,
    );
    let mut component = component("let on = true;");
    let widget =
        build_tree(&element, &empty_stylesheet(), None, &mut component).expect("tree builds");
    let id = widget.id();
    let tree = WidgetTree::new(widget);
    let state = tree.state(id).expect("root state");

    assert!(state.checked);
    assert!(state.selected);
    assert!(state.disabled);
    assert!(state.visible);
}

#[test]
fn position_parameters_set_an_offset() {
    let element = root(
        r#"
        window Picker (title: "x") {
            Column {
                Text (x: 10, y: 5) { "hello" }
            }
        }
        "#,
    );
    let widget = build_tree(&element, &empty_stylesheet(), None, &mut Component::new())
        .expect("tree builds");
    let snapshot = snapshot(widget);
    let node = snapshot
        .nodes
        .iter()
        .find(|node| node.kind == WidgetKind::Text)
        .expect("text node");

    assert_eq!((node.x, node.y), (10, 5));
}

#[test]
fn parameter_types_are_checked() {
    let element = root(
        r#"
        window Picker (title: "x") {
            Column (visible: 1) { }
        }
        "#,
    );
    let error = build_tree(&element, &empty_stylesheet(), None, &mut Component::new())
        .err()
        .expect("type error");

    assert_eq!(
        error.message,
        "parameter 'visible' expects Bool, got Number"
    );
}

#[test]
fn unknown_parameters_are_rejected() {
    let element = root(
        r#"
        window Picker (title: "x") {
            Column (title: "x") { }
        }
        "#,
    );
    let error = build_tree(&element, &empty_stylesheet(), None, &mut Component::new())
        .err()
        .expect("arguments");

    assert_eq!(error.message, "parameter 'title' is not supported yet");
}

#[test]
fn handlers_are_not_supported_yet() {
    let element = root(
        r#"
        window Picker (title: "x") {
            Column (on click: toggle) { }
        }
        "#,
    );
    let error = build_tree(&element, &empty_stylesheet(), None, &mut Component::new())
        .err()
        .expect("handlers");

    assert_eq!(error.message, "handlers are not supported yet");
}

#[test]
fn if_is_not_supported_yet() {
    let element = root(
        r#"
        window Picker (title: "x") {
            Column {
                if (true) { }
            }
        }
        "#,
    );
    let error = build_tree(&element, &empty_stylesheet(), None, &mut Component::new())
        .err()
        .expect("if");

    assert_eq!(error.message, "'if' is not supported yet");
    assert_eq!((error.line, error.column), (3, 13));
}
