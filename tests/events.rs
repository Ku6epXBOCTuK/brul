use brul::Ui;
use brul_compiler::load;
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

#[test]
fn widget_events_call_script_handlers() {
    let mut ui = build(APP);
    let id = text_ids(&ui)[0];

    ui.dispatch(&[
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

    ui.dispatch(&[WidgetEvent::Click(id)]);

    assert_eq!(count(&ui), 0.0);
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
