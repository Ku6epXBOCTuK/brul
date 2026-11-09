use brul_gui::{
    Color, Dimension, LayoutStyle, Padding, Size, StyleSet, StyleState, TextEngine, Texture,
    Widget, WidgetError, WidgetEvent, WidgetState, WidgetStyle, WidgetTree,
};

fn fixed(width: u32, height: u32) -> LayoutStyle {
    LayoutStyle {
        width: Dimension::Px(width),
        height: Dimension::Px(height),
        ..LayoutStyle::default()
    }
}

fn style(background: Color) -> WidgetStyle {
    WidgetStyle {
        background,
        ..WidgetStyle::default()
    }
}

#[test]
fn replacing_children_keeps_the_other_widgets() {
    let first = Widget::text("a").with_layout(fixed(40, 20));
    let first_id = first.id();
    let second = Widget::text("b").with_layout(fixed(40, 20));
    let second_id = second.id();
    let root = Widget::row(vec![first, second]).with_layout(fixed(100, 40));
    let root_id = root.id();
    let mut tree = WidgetTree::new(root);

    let replacement = Widget::text("c").with_layout(fixed(40, 20));
    assert!(tree.replace_children(root_id, 0, 1, vec![replacement]));
    assert!(!tree.replace_children(second_id, 0, 0, Vec::new()));

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
    let snapshot = tree.snapshot().cloned().expect("snapshot");
    assert!(snapshot.nodes.iter().any(|node| node.id == second_id));
    assert!(!snapshot.nodes.iter().any(|node| node.id == first_id));
}

#[test]
fn draggable_widget_reports_a_drag_on_press() {
    let text = Widget::text("drag")
        .with_layout(fixed(40, 20))
        .draggable(true);
    let text_id = text.id();
    let root = Widget::column(vec![text]).with_layout(fixed(100, 100));
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

    let events = tree.pointer_down(5, 5).expect("down");
    assert!(events.contains(&WidgetEvent::Drag(text_id)));
}

#[test]
fn children_are_placed_inside_padding_and_spacing() {
    let first = Widget::column(vec![]).with_layout(fixed(40, 20));
    let first_id = first.id();
    let second = Widget::column(vec![]).with_layout(fixed(40, 20));
    let second_id = second.id();
    let root = Widget::column(vec![first, second]).with_layout(LayoutStyle {
        width: Dimension::Px(200),
        height: Dimension::Px(100),
        padding: Padding::all(10),
        spacing: 5,
        ..LayoutStyle::default()
    });
    let root_id = root.id();
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 200,
            height: 100,
        },
        1.0,
    )
    .expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");

    assert_eq!(
        snapshot.size,
        Size {
            width: 200,
            height: 100
        }
    );
    let root_node = snapshot.node(root_id).expect("root");
    assert_eq!((root_node.x, root_node.y), (0, 0));
    let first_node = snapshot.node(first_id).expect("first");
    assert_eq!((first_node.x, first_node.y), (10, 10));
    let second_node = snapshot.node(second_id).expect("second");
    assert_eq!((second_node.x, second_node.y), (10, 35));
}

#[test]
fn text_is_measured_and_wrapped_by_layout() {
    let text =
        Widget::text("one two three four five six seven eight nine ten").with_layout(LayoutStyle {
            width: Dimension::Px(80),
            ..LayoutStyle::default()
        });
    let text_id = text.id();
    let root = Widget::column(vec![text]).with_layout(fixed(120, 200));
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 120,
            height: 200,
        },
        1.0,
    )
    .expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");
    let node = snapshot.node(text_id).expect("text");

    assert_eq!(node.width, 80);
    assert!(node.height > 28);
    let text = node.text.as_ref().expect("text layout");
    assert!(!text.glyphs.is_empty());
    assert!(text.size.width <= 80);
}

#[test]
fn image_keeps_texture_size() {
    let texture = Texture::from_rgba(2, 1, &[255, 0, 0, 255, 0, 255, 0, 255]).expect("texture");
    let image = Widget::image(texture);
    let image_id = image.id();
    let mut tree = WidgetTree::new(image);
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
    let node = snapshot.node(image_id).expect("image");

    assert_eq!((node.width, node.height), (2, 1));
    assert!(node.texture.is_some());
}

#[test]
fn hit_test_returns_deepest_widget() {
    let text = Widget::text("ok");
    let text_id = text.id();
    let child = Widget::column(vec![text])
        .with_layout(fixed(100, 100))
        .with_offset(20, 20);
    let root = Widget::column(vec![child]).with_layout(fixed(200, 200));
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 200,
            height: 200,
        },
        1.0,
    )
    .expect("layout");

    assert_eq!(tree.hit(25, 25).expect("hit"), Some(text_id));
}

#[test]
fn hover_switches_style_snapshot() {
    let base = style(Color::rgb(1, 2, 3));
    let hovered = style(Color::rgb(4, 5, 6));
    let view = Widget::column(vec![])
        .with_layout(fixed(50, 50))
        .with_styles(StyleSet::new(base).with(StyleState::Hovered, hovered));
    let id = view.id();
    let mut tree = WidgetTree::new(view);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 50,
            height: 50,
        },
        1.0,
    )
    .expect("layout");

    assert_eq!(
        tree.pointer_moved(10, 10).expect("move"),
        vec![WidgetEvent::PointerEnter(id)]
    );
    {
        let snapshot = tree.snapshot().expect("snapshot");
        let node = snapshot.node(id).expect("view");
        assert_eq!(node.style.background, hovered.background);
        assert!(node.state.hovered);
    }

    assert_eq!(tree.pointer_leave(), vec![WidgetEvent::PointerLeave(id)]);
    let snapshot = tree.snapshot().expect("snapshot");
    let node = snapshot.node(id).expect("view");
    assert_eq!(node.style.background, base.background);
    assert!(!node.state.hovered);
}

#[test]
fn click_bubbles_from_leaf_to_root() {
    let text = Widget::text("ok");
    let text_id = text.id();
    let child = Widget::column(vec![text]).with_layout(fixed(80, 80));
    let child_id = child.id();
    let root = Widget::column(vec![child]).with_layout(fixed(100, 100));
    let root_id = root.id();
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

    assert_eq!(
        tree.pointer_down(5, 5).expect("down"),
        vec![
            WidgetEvent::Press(text_id),
            WidgetEvent::Press(child_id),
            WidgetEvent::Press(root_id),
        ]
    );
    assert_eq!(
        tree.pointer_up(5, 5).expect("up"),
        vec![
            WidgetEvent::Release(text_id),
            WidgetEvent::Release(child_id),
            WidgetEvent::Release(root_id),
            WidgetEvent::Click(text_id),
            WidgetEvent::Click(child_id),
            WidgetEvent::Click(root_id),
        ]
    );
    assert_eq!(
        tree.state(root_id),
        Some(WidgetState {
            hovered: false,
            pressed: false,
            ..WidgetState::default()
        })
    );
}

#[test]
fn disabled_subtree_does_not_receive_clicks() {
    let child = Widget::column(vec![Widget::text("ok")])
        .with_layout(fixed(80, 80))
        .disabled(true);
    let child_id = child.id();
    let root = Widget::column(vec![child]).with_layout(fixed(100, 100));
    let root_id = root.id();
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

    let down = tree.pointer_down(5, 5).expect("down");
    let up = tree.pointer_up(5, 5).expect("up");

    assert_eq!(down, vec![WidgetEvent::Press(root_id)]);
    assert!(!up.contains(&WidgetEvent::Click(child_id)));
    assert!(up.contains(&WidgetEvent::Click(root_id)));
}

#[test]
fn checked_state_switches_style_and_toggles() {
    let base = style(Color::rgb(1, 2, 3));
    let checked = style(Color::rgb(7, 8, 9));
    let view = Widget::column(vec![])
        .with_layout(fixed(20, 20))
        .with_styles(StyleSet::new(base).with(StyleState::Checked, checked));
    let id = view.id();
    let mut tree = WidgetTree::new(view);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 20,
            height: 20,
        },
        1.0,
    )
    .expect("layout");
    assert!(tree.set_checked(id, true));
    {
        let snapshot = tree.snapshot().expect("snapshot");
        let node = snapshot.node(id).expect("view");
        assert!(node.state.checked);
        assert_eq!(node.style.background, checked.background);
    }
    assert_eq!(tree.toggle_checked(id), Some(false));
    let snapshot = tree.snapshot().expect("snapshot");
    assert_eq!(
        snapshot.node(id).expect("view").style.background,
        base.background
    );
}

#[test]
fn invisible_widget_is_not_hit_or_snapshotted() {
    let child = Widget::column(vec![])
        .visible(false)
        .with_layout(fixed(80, 80));
    let child_id = child.id();
    let root = Widget::column(vec![child]).with_layout(fixed(100, 100));
    let root_id = root.id();
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

    assert_eq!(tree.hit(5, 5).expect("hit"), Some(root_id));
    assert!(tree.snapshot().expect("snapshot").node(child_id).is_none());
}

#[test]
fn events_before_layout_are_errors() {
    let mut tree = WidgetTree::new(Widget::column(vec![]));

    assert_eq!(tree.pointer_moved(0, 0), Err(WidgetError::NoLayout));
    assert!(matches!(tree.snapshot(), Err(WidgetError::NoLayout)));
}

#[test]
fn invalid_scale_is_rejected() {
    let mut tree = WidgetTree::new(Widget::column(vec![]));
    let mut engine = TextEngine::new();

    assert_eq!(
        tree.layout(
            &mut engine,
            Size {
                width: 10,
                height: 10
            },
            0.0
        ),
        Err(WidgetError::InvalidScale)
    );
}

#[test]
fn unknown_widget_state_is_ignored() {
    let stranger = Widget::text("x");
    let stranger_id = stranger.id();
    let mut tree = WidgetTree::new(Widget::column(vec![]));

    assert!(!tree.set_checked(stranger_id, true));
    assert_eq!(tree.state(stranger_id), None);
}
