use super::*;
#[test]
fn unknown_widget_state_is_ignored() {
    let stranger = Widget::text("x");
    let stranger_id = stranger.id();
    let mut tree = WidgetTree::new(Widget::column(vec![]));

    assert!(!tree.set_checked(stranger_id, true));
    assert_eq!(tree.state(stranger_id), None);
}

#[test]
fn layer_covers_the_window_ignoring_root_padding() {
    let content = Widget::column(Vec::new()).with_layout(fixed(40, 40));
    let content_id = content.id();
    let layer = Widget::layer(Vec::new());
    let layer_id = layer.id();
    let root = Widget::column(vec![content, layer]).with_layout(LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Percent(100.0),
        padding: Padding::all(20),
        ..LayoutStyle::default()
    });
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 200,
            height: 120,
        },
        1.0,
    )
    .expect("layout");

    let snapshot = tree.snapshot().expect("snapshot");
    let layer_box = snapshot.node(layer_id).expect("layer");
    assert_eq!(
        (layer_box.x, layer_box.y, layer_box.width, layer_box.height),
        (0, 0, 200, 120)
    );
    let content_box = snapshot.node(content_id).expect("content");
    assert_eq!((content_box.x, content_box.y), (20, 20));
}

#[test]
fn layer_is_snapshotted_after_content_declared_later() {
    let layer = Widget::layer(Vec::new());
    let layer_id = layer.id();
    let content = Widget::column(Vec::new()).with_layout(fixed(100, 100));
    let content_id = content.id();
    let root = Widget::column(vec![layer, content]).with_layout(fixed(100, 100));
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 100,
            height: 100,
        },
        1.0,
    )
    .expect("layout");

    let snapshot = tree.snapshot().expect("snapshot");
    let position = |id| snapshot.nodes.iter().position(|node| node.id == id);
    assert!(position(content_id) < position(layer_id));
}

#[test]
fn a_layer_child_takes_input_before_content() {
    let button = Widget::column(Vec::new()).with_layout(fixed(40, 40));
    let button_id = button.id();
    let content = Widget::column(vec![button]).with_layout(fixed(100, 100));
    let content_id = content.id();
    let scrim = Widget::column(Vec::new()).with_layout(LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Percent(100.0),
        ..LayoutStyle::default()
    });
    let scrim_id = scrim.id();
    let layer = Widget::layer(vec![scrim]);
    let layer_id = layer.id();
    let root = Widget::column(vec![content, layer]).with_layout(fixed(100, 100));
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 100,
            height: 100,
        },
        1.0,
    )
    .expect("layout");

    assert_eq!(tree.hit(5, 5).expect("hit"), Some(scrim_id));
    assert_ne!(tree.hit(5, 5).expect("hit"), Some(button_id));
    assert!(tree.snapshot().expect("snapshot").node(layer_id).is_some());
    assert!(
        tree.snapshot()
            .expect("snapshot")
            .node(content_id)
            .is_some()
    );
}

#[test]
fn empty_part_of_a_layer_lets_the_click_through() {
    let button = Widget::column(Vec::new()).with_layout(fixed(100, 100));
    let button_id = button.id();
    let bubble = Widget::column(Vec::new()).with_layout(fixed(20, 20));
    let bubble_id = bubble.id();
    let layer = Widget::layer(vec![bubble]);
    let root = Widget::column(vec![button, layer]).with_layout(fixed(100, 100));
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 100,
            height: 100,
        },
        1.0,
    )
    .expect("layout");

    assert_eq!(tree.hit(5, 5).expect("hit"), Some(bubble_id));
    assert_eq!(tree.hit(50, 50).expect("hit"), Some(button_id));
}

#[test]
fn invisible_layer_is_not_hit() {
    let button = Widget::column(Vec::new()).with_layout(fixed(100, 100));
    let button_id = button.id();
    let layer = Widget::layer(Vec::new())
        .visible(false)
        .with_layout(fixed(100, 100));
    let root = Widget::column(vec![button, layer]).with_layout(fixed(100, 100));
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 100,
            height: 100,
        },
        1.0,
    )
    .expect("layout");

    assert_eq!(tree.hit(50, 50).expect("hit"), Some(button_id));
}

#[test]
fn a_layer_inside_a_structure_still_takes_input_above_content() {
    let content = Widget::column(Vec::new()).with_layout(fixed(100, 100));
    let content_id = content.id();
    let bubble = Widget::column(Vec::new()).with_layout(fixed(20, 20));
    let bubble_id = bubble.id();
    let layer = Widget::layer(vec![bubble]);
    let root =
        Widget::column(vec![content, Widget::structure(vec![layer])]).with_layout(fixed(100, 100));
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 100,
            height: 100,
        },
        1.0,
    )
    .expect("layout");

    assert_eq!(tree.hit(5, 5).expect("hit"), Some(bubble_id));
    assert_eq!(tree.hit(50, 50).expect("hit"), Some(content_id));
}
