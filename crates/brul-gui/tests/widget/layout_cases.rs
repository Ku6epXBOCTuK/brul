use super::*;
#[test]
fn a_class_restyles_a_built_widget_without_relayout() {
    let view = Widget::column(Vec::new())
        .with_layout(fixed(50, 50))
        .with_styles(StyleSet::new(style(Color::rgb(1, 2, 3))));
    let id = view.id();
    let mut tree = WidgetTree::new(view);
    let mut engine = TextEngine::new();
    let viewport = Size {
        width: 100,
        height: 100,
    };
    tree.layout(&mut engine, viewport, 1.0).expect("layout");

    let class = Class {
        style: Layers::new(WidgetStylePatch {
            background: Some(Paint::Color(Color::rgb(9, 8, 7))),
            ..WidgetStylePatch::default()
        }),
        ..Class::default()
    };
    assert!(tree.apply_class(id, Some(&class)));
    assert!(!tree.layout_dirty());
    assert!(!tree.apply_class(id, Some(&class)));

    let node = tree.snapshot().expect("snapshot").node(id).cloned();
    assert_eq!(
        node.expect("node").style.background,
        Paint::Color(Color::rgb(9, 8, 7))
    );

    assert!(tree.apply_class(id, None));
    let node = tree.snapshot().expect("snapshot").node(id).cloned();
    assert_eq!(
        node.expect("node").style.background,
        Paint::Color(Color::rgb(1, 2, 3))
    );
}

#[test]
fn a_class_carries_state_patches_and_transitions() {
    let base = style(Color::rgb(1, 2, 3));
    let hovered = Paint::Color(Color::rgb(4, 5, 6));
    let view = Widget::column(Vec::new())
        .with_layout(fixed(50, 50))
        .with_styles(StyleSet::new(base));
    let id = view.id();
    let mut tree = WidgetTree::new(view);
    let mut engine = TextEngine::new();
    let viewport = Size {
        width: 50,
        height: 50,
    };
    tree.layout(&mut engine, viewport, 1.0).expect("layout");

    let class = Class {
        style: Layers::new(WidgetStylePatch::default()).with(
            StyleState::Hovered,
            WidgetStylePatch {
                background: Some(hovered),
                ..WidgetStylePatch::default()
            },
        ),
        transitions: Transitions {
            background: Some(Duration::from_millis(100)),
            ..Transitions::default()
        },
        ..Class::default()
    };
    assert!(tree.apply_class(id, Some(&class)));
    tree.set_hovered(id, true);
    let node = tree.snapshot().expect("snapshot").node(id).cloned();
    assert_eq!(node.expect("node").style.background, base.background);

    tree.advance(Duration::from_millis(200));
    let node = tree.snapshot().expect("snapshot").node(id).cloned();
    assert_eq!(node.expect("node").style.background, hovered);
}

#[test]
fn a_style_patch_rewrites_the_snapshot_in_place() {
    let view = Widget::column(Vec::new())
        .with_layout(fixed(50, 50))
        .with_styles(StyleSet::new(style(Color::rgb(1, 2, 3))));
    let id = view.id();
    let mut tree = WidgetTree::new(view);
    let mut engine = TextEngine::new();
    let viewport = Size {
        width: 50,
        height: 50,
    };
    tree.layout(&mut engine, viewport, 1.0).expect("layout");
    let before = tree.snapshot().expect("snapshot").nodes.as_ptr();

    let class = Class {
        style: Layers::new(WidgetStylePatch {
            background: Some(Paint::Color(Color::rgb(9, 8, 7))),
            ..WidgetStylePatch::default()
        }),
        ..Class::default()
    };
    assert!(tree.apply_class(id, Some(&class)));
    assert!(!tree.layout_dirty(), "color does not relayout");
    assert!(
        tree.snapshot_dirty(),
        "the stale snapshot waits for a patch"
    );

    let snapshot = tree.snapshot().expect("snapshot");
    assert_eq!(snapshot.nodes.as_ptr(), before, "patched in place");
    assert_eq!(
        snapshot.node(id).expect("node").style.background,
        Paint::Color(Color::rgb(9, 8, 7))
    );
    assert!(!tree.snapshot_dirty());
}

#[test]
fn a_clip_change_rebuilds_the_child_clip_rects() {
    let child = Widget::text("line").with_layout(fixed(40, 20));
    let child_id = child.id();
    let root = Widget::column(vec![child])
        .with_layout(fixed(60, 40))
        .with_styles(StyleSet::new(WidgetStyle {
            border_radius: 8,
            ..WidgetStyle::default()
        }));
    let root_id = root.id();
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();
    let viewport = Size {
        width: 60,
        height: 40,
    };
    tree.layout(&mut engine, viewport, 1.0).expect("layout");
    assert!(
        tree.snapshot()
            .expect("snapshot")
            .node(child_id)
            .expect("child")
            .clips
            .is_empty()
    );

    let class = Class {
        style: Layers::new(WidgetStylePatch {
            clip: Some(true),
            ..WidgetStylePatch::default()
        }),
        ..Class::default()
    };
    assert!(tree.apply_class(root_id, Some(&class)));
    assert!(!tree.layout_dirty(), "clip does not relayout");

    let snapshot = tree.snapshot().expect("snapshot");
    let clips = &snapshot.node(child_id).expect("child").clips;
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].radius, 8);
}

#[test]
fn a_style_geometry_change_relayouts_the_widget() {
    let view = Widget::column(Vec::new()).with_layout(fixed(50, 50));
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

    let class = Class {
        style: Layers::new(WidgetStylePatch {
            border_radius: Some(8),
            ..WidgetStylePatch::default()
        }),
        ..Class::default()
    };
    assert!(tree.apply_class(id, Some(&class)));
    assert!(tree.layout_dirty(), "geometry relayouts");
}

#[test]
fn a_class_carrying_a_layout_relayouts_the_widget() {
    let view = Widget::column(Vec::new()).with_layout(fixed(50, 50));
    let id = view.id();
    let mut tree = WidgetTree::new(view);
    let mut engine = TextEngine::new();
    let viewport = Size {
        width: 100,
        height: 100,
    };
    tree.layout(&mut engine, viewport, 1.0).expect("layout");

    let class = Class {
        layout: LayoutPatch {
            width: Some(Dimension::Px(80)),
            ..LayoutPatch::default()
        },
        ..Class::default()
    };
    assert!(tree.apply_class(id, Some(&class)));
    assert!(tree.layout_dirty());
    tree.layout(&mut engine, viewport, 1.0).expect("relayout");

    let node = tree.snapshot().expect("snapshot").node(id).cloned();
    assert_eq!(node.expect("node").width, 80);

    assert!(tree.apply_class(id, None));
    tree.layout(&mut engine, viewport, 1.0).expect("relayout");
    let node = tree.snapshot().expect("snapshot").node(id).cloned();
    assert_eq!(node.expect("node").width, 50);
}

#[test]
fn a_class_can_turn_a_column_into_a_row() {
    let first = Widget::column(vec![]).with_layout(fixed(40, 20));
    let second = Widget::column(vec![]).with_layout(fixed(40, 20));
    let second_id = second.id();
    let root = Widget::column(vec![first, second]).with_layout(LayoutStyle {
        width: Dimension::Px(200),
        height: Dimension::Px(100),
        gap: 5,
        ..LayoutStyle::default()
    });
    let root_id = root.id();
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();
    let viewport = Size {
        width: 200,
        height: 100,
    };

    tree.layout(&mut engine, viewport, 1.0).expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");
    assert_eq!(snapshot.node(second_id).expect("second").y, 25);

    let class = Class {
        layout: LayoutPatch {
            flex_direction: Some(Direction::Row),
            ..LayoutPatch::default()
        },
        ..Class::default()
    };
    assert!(tree.apply_class(root_id, Some(&class)));
    assert!(tree.layout_dirty());
    tree.layout(&mut engine, viewport, 1.0).expect("relayout");
    let snapshot = tree.snapshot().expect("snapshot");
    let second = snapshot.node(second_id).expect("second");
    assert_eq!((second.x, second.y), (45, 0));
}

#[test]
fn a_grown_branch_is_laid_out_again_when_its_sibling_shrinks() {
    fn build(left: &str) -> (WidgetTree, WidgetId, WidgetId, WidgetId) {
        let left_widget = Widget::text(left);
        let left_id = left_widget.id();
        let inner = Widget::text("ok");
        let inner_id = inner.id();
        let right = Widget::column(vec![inner]).with_layout(LayoutStyle {
            flex_grow: 1.0,
            ..LayoutStyle::default()
        });
        let right_id = right.id();
        let root = Widget::column(vec![
            Widget::row(vec![left_widget, right]).with_layout(fixed(100, 40)),
        ]);
        (WidgetTree::new(root), left_id, right_id, inner_id)
    }

    let (mut tree, left_id, right_id, inner_id) = build("aaaa");
    let mut engine = TextEngine::new();
    let viewport = Size {
        width: 100,
        height: 40,
    };
    tree.layout(&mut engine, viewport, 1.0).expect("layout");
    let before = tree.snapshot().cloned().expect("snapshot");
    let right_before = before.node(right_id).expect("right").width;

    assert!(tree.set_text(left_id, "a".to_string()));
    tree.layout(&mut engine, viewport, 1.0).expect("relayout");
    let after = tree.snapshot().cloned().expect("snapshot");
    let right_after = after.node(right_id).expect("right").clone();
    assert!(right_after.width > right_before);

    let (mut fresh, _, fresh_right_id, fresh_inner_id) = build("a");
    fresh.layout(&mut engine, viewport, 1.0).expect("layout");
    let expected = fresh.snapshot().cloned().expect("snapshot");
    let expected_right = expected.node(fresh_right_id).expect("right");
    let inner = after.node(inner_id).expect("inner");
    let expected_inner = expected.node(fresh_inner_id).expect("inner");

    assert_eq!(right_after.width, expected_right.width);
    assert_eq!(right_after.height, expected_right.height);
    assert_eq!(inner.x, expected_inner.x);
    assert_eq!(inner.width, expected_inner.width);
}

#[test]
fn events_before_layout_are_errors() {
    let mut tree = WidgetTree::new(Widget::column(vec![]));

    assert_eq!(tree.pointer_raw_moved(0, 0), Err(WidgetError::NoLayout));
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
