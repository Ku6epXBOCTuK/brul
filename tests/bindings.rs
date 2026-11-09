use brul::Ui;
use brul_compiler::load;
use brul_gui::{Size, TextEngine, WidgetKind};
use brul_script::Value;

const APP: &str = r#"
<script>
let label = "one";
let left = 0;
</script>

<markup>
window Demo (title: "Demo") {
    Row {
        Text (x: left) { label }
        Text { "static" }
    }
}
</markup>
"#;

fn build() -> Ui {
    let loaded = load(APP, None).expect("app loads");
    Ui::new(
        loaded.component,
        loaded.tree,
        loaded.bindings,
        loaded.handlers,
        loaded.root,
        loaded.stylesheet,
    )
}

fn layout(ui: &Ui) -> Size {
    let size = Size {
        width: 300,
        height: 100,
    };
    ui.tree
        .borrow_mut()
        .layout(&mut TextEngine::new(), size, 1.0)
        .expect("layout");
    ui.tree.borrow_mut().take_dirty();
    size
}

#[test]
fn text_and_offset_bindings_follow_component_values() {
    let ui = build();
    let size = layout(&ui);
    ui.component
        .borrow_mut()
        .assign("label", Value::String(String::from("two")))
        .expect("assign");
    ui.component
        .borrow_mut()
        .assign("left", Value::Number(10.0))
        .expect("assign");
    brul_reactive::flush();
    assert!(ui.tree.borrow_mut().take_dirty().full);
    assert!(ui.tree.borrow().layout_dirty());
    ui.tree
        .borrow_mut()
        .layout(&mut TextEngine::new(), size, 1.0)
        .expect("layout");
    let snapshot = ui.tree.borrow_mut().snapshot().expect("snapshot").clone();
    let text = snapshot
        .nodes
        .iter()
        .find(|node| node.kind == WidgetKind::Text)
        .expect("text node");
    assert_eq!(text.x, 10);
}

#[test]
fn equal_values_do_not_mark_the_tree_dirty() {
    let ui = build();
    layout(&ui);
    ui.component
        .borrow_mut()
        .assign("label", Value::String(String::from("one")))
        .expect("assign");
    ui.component
        .borrow_mut()
        .assign("left", Value::Number(0.0))
        .expect("assign");
    brul_reactive::flush();
    let dirty = ui.tree.borrow_mut().take_dirty();
    assert!(!dirty.full);
    assert!(dirty.ids.is_empty());
    assert!(!ui.tree.borrow().layout_dirty());
}
