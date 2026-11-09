use brul::Ui;
use brul_compiler::load;
use brul_gui::window::WindowCommand;
use brul_gui::{Size, TextEngine, WidgetEvent, WidgetId, WidgetKind};
use brul_script::Value;

const APP: &str = r#"
<script>
let hits = 0;
function hit() {
    hits = hits + 1;
}
</script>

<markup>
window Demo (title: "Demo") {
    Column {
        Text (on click: hit, on press: hit, on release: hit, on enter: hit, on leave: hit) {
            "tap"
        }
        Text { "static" }
    }
}
</markup>
"#;

fn build(source: &str) -> Ui {
    let loaded = load(source, None).expect("app loads");
    Ui::new(
        loaded.component,
        loaded.tree,
        loaded.bindings,
        loaded.handlers,
    )
}

fn text_ids(ui: &Ui) -> Vec<WidgetId> {
    ui.tree
        .borrow_mut()
        .layout(
            &mut TextEngine::new(),
            Size {
                width: 300,
                height: 100,
            },
            1.0,
        )
        .expect("layout");
    let snapshot = ui.tree.borrow_mut().snapshot().expect("snapshot").clone();
    snapshot
        .nodes
        .iter()
        .filter(|node| node.kind == WidgetKind::Text)
        .map(|node| node.id)
        .collect()
}

fn count(ui: &Ui) -> f64 {
    match ui.component.borrow().read("hits").expect("hits") {
        Value::Number(value) => value,
        other => panic!("unexpected value: {other:?}"),
    }
}

const KEY: &str = r#"
<script>
let code = "";
function handleKey(ev) {
    code = ev.code;
}
</script>

<markup>
window Demo (title: "Demo", on key: handleKey) {
    Column {
        Text { "press a key" }
    }
}
</markup>
"#;

#[test]
fn window_key_handler_receives_the_code() {
    let mut ui = build(KEY);
    let _ = ui.dispatch(&[WidgetEvent::Key("Enter".to_owned())]);
    assert_eq!(
        ui.component.borrow().read("code").expect("code"),
        Value::String("Enter".to_owned())
    );
}

#[test]
fn drag_accepts_drag_window_handler() {
    let source = r#"
<markup>
window Demo (title: "Demo") {
    Column (on drag: dragWindow) {
        Text { "drag" }
    }
}
</markup>
"#;
    let loaded = load(source, None).expect("app loads");
    assert!(loaded.handlers.is_empty());
}

#[test]
fn key_on_an_element_is_rejected() {
    let source = r#"
<script>
function handleKey(ev) {
}
</script>

<markup>
window Demo (title: "Demo") {
    Column (on key: handleKey) {
        Text { "x" }
    }
}
</markup>
"#;
    let error = load(source, None).err().expect("error");
    assert_eq!(
        error.to_string(),
        "line 9, column 5: key events are only supported on the window"
    );
}

#[test]
fn window_rejects_element_events() {
    let source = r#"
<script>
function handleClick() {
}
</script>

<markup>
window Demo (title: "Demo", on click: handleClick) {
    Column {
        Text { "x" }
    }
}
</markup>
"#;
    let error = load(source, None).err().expect("error");
    assert_eq!(
        error.to_string(),
        "line 8, column 7: window event 'click' is not supported yet"
    );
}

#[test]
fn window_rejects_a_second_key_handler() {
    let source = r#"
<script>
function handleKey(ev) {
}
</script>

<markup>
window Demo (title: "Demo", on key: handleKey, on key: handleKey) {
    Column {
        Text { "x" }
    }
}
</markup>
"#;
    let error = load(source, None).err().expect("error");
    assert_eq!(
        error.to_string(),
        "line 8, column 7: event 'key' is handled twice"
    );
}

#[test]
fn widget_events_call_script_handlers() {
    let mut ui = build(APP);
    let id = text_ids(&ui)[0];

    let _ = ui.dispatch(&[
        WidgetEvent::Click(id),
        WidgetEvent::Press(id),
        WidgetEvent::Release(id),
        WidgetEvent::PointerEnter(id),
        WidgetEvent::PointerLeave(id),
    ]);

    assert_eq!(count(&ui), 5.0);
}

#[test]
fn events_of_other_widgets_are_ignored() {
    let mut ui = build(APP);
    let id = text_ids(&ui)[1];

    let _ = ui.dispatch(&[WidgetEvent::Click(id)]);

    assert_eq!(count(&ui), 0.0);
}

#[test]
fn builtin_exit_and_drag_push_window_commands() {
    let source = r#"
<script>
function leave() {
    exit();
}
function move() {
    dragWindow();
}
</script>

<markup>
window Demo (title: "Demo") {
    Column {
        Text (on click: leave) { "exit" }
        Text (on click: move) { "drag" }
    }
}
</markup>
"#;
    let mut ui = build(source);
    let ids = text_ids(&ui);

    assert_eq!(
        ui.dispatch(&[WidgetEvent::Click(ids[0])]),
        vec![WindowCommand::Exit]
    );
    assert_eq!(
        ui.dispatch(&[WidgetEvent::Click(ids[1])]),
        vec![WindowCommand::Drag]
    );
}

#[test]
fn handler_must_exist_in_the_script() {
    let error = load(
        r#"
<markup>
window Demo (title: "Demo") {
    Text (on click: missing) { "tap" }
}
</markup>
"#,
        None,
    )
    .err()
    .expect("error");

    assert_eq!(
        error.to_string(),
        "line 4, column 5: function 'missing' is not defined"
    );
}

#[test]
fn unknown_event_is_rejected() {
    let error = load(
        r#"
<script>
function hit() { }
</script>

<markup>
window Demo (title: "Demo") {
    Text (on hover: hit) { "tap" }
}
</markup>
"#,
        None,
    )
    .err()
    .expect("error");

    assert_eq!(
        error.to_string(),
        "line 8, column 5: event 'hover' is not supported yet"
    );
}

#[test]
fn duplicate_event_is_rejected() {
    let error = load(
        r#"
<script>
function hit() { }
</script>

<markup>
window Demo (title: "Demo") {
    Text (on click: hit, on click: hit) { "tap" }
}
</markup>
"#,
        None,
    )
    .err()
    .expect("error");

    assert_eq!(
        error.to_string(),
        "line 8, column 5: event 'click' is handled twice"
    );
}
