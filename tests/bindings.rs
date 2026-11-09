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

#[test]
fn each_rows_bindings_keep_their_item_scope() {
    let source = r#"
<script>
let prefix = "x";
let items = [{ label: "one" }, { label: "two" }];
</script>

<markup>
window Demo (title: "Demo") {
    Column {
        each (items as item, i (item.label)) {
            Text { prefix + item.label }
        }
    }
}
</markup>
"#;
    let loaded = load(source, None).expect("app loads");
    let ui = Ui::new(
        loaded.component,
        loaded.tree,
        loaded.bindings,
        loaded.handlers,
        loaded.root,
        loaded.stylesheet,
    );
    layout(&ui);

    ui.component
        .borrow_mut()
        .assign("prefix", Value::String("long".to_owned()))
        .expect("assign");
    brul_reactive::flush();
    layout(&ui);

    let mut tree = ui.tree.borrow_mut();
    let snapshot = tree.snapshot().cloned().expect("snapshot");
    drop(tree);
    let glyphs: Vec<usize> = snapshot
        .nodes
        .iter()
        .filter(|node| node.kind == WidgetKind::Text)
        .map(|node| node.text.as_ref().expect("text").glyphs.len())
        .collect();
    assert_eq!(glyphs, vec![7, 7]);
}
