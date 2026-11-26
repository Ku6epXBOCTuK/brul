use super::*;
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
fn a_clipped_child_is_not_hit() {
    let child = Widget::column(Vec::new()).with_layout(fixed(32, 16));
    let child_id = child.id();
    let parent = Widget::column(vec![child])
        .with_layout(fixed(16, 16))
        .with_styles(StyleSet::new(WidgetStyle {
            clip: true,
            border_radius: 4,
            ..WidgetStyle::default()
        }));
    let parent_id = parent.id();
    let root = Widget::column(vec![parent]).with_layout(fixed(32, 16));
    let root_id = root.id();
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 32,
            height: 16,
        },
        1.0,
    )
    .expect("layout");

    assert_eq!(tree.hit(0, 0).expect("hit"), Some(parent_id));
    assert_ne!(tree.hit(20, 8).expect("hit"), Some(child_id));
    assert_eq!(tree.hit(20, 8).expect("hit"), Some(root_id));
}

#[test]
fn a_class_clip_is_applied_to_hit_testing() {
    let child = Widget::column(Vec::new()).with_layout(fixed(32, 16));
    let child_id = child.id();
    let parent = Widget::column(vec![child]).with_layout(fixed(16, 16));
    let parent_id = parent.id();
    let root = Widget::column(vec![parent]).with_layout(fixed(32, 16));
    let root_id = root.id();
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();
    let viewport = Size {
        width: 32,
        height: 16,
    };

    tree.layout(&mut engine, viewport, 1.0).expect("layout");

    let class = Class {
        style: Layers::new(WidgetStylePatch {
            clip: Some(true),
            border_radius: Some(4),
            ..WidgetStylePatch::default()
        }),
        ..Class::default()
    };
    assert!(tree.apply_class(parent_id, Some(&class)));
    tree.layout(&mut engine, viewport, 1.0).expect("relayout");

    assert_eq!(tree.hit(0, 0).expect("hit"), Some(parent_id));
    assert_ne!(tree.hit(20, 8).expect("hit"), Some(child_id));
    assert_eq!(tree.hit(20, 8).expect("hit"), Some(root_id));
}

#[test]
fn clipped_child_hover_does_not_stick_across_moves() {
    let child = Widget::column(Vec::new()).with_layout(fixed(32, 16));
    let child_id = child.id();
    let parent = Widget::column(vec![child])
        .with_layout(fixed(16, 16))
        .with_styles(StyleSet::new(WidgetStyle {
            clip: true,
            ..WidgetStyle::default()
        }));
    let root = Widget::column(vec![parent]).with_layout(fixed(32, 16));
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 32,
            height: 16,
        },
        1.0,
    )
    .expect("layout");

    let inside = tree.pointer_raw_moved(8, 8).expect("move inside");
    assert!(inside.contains(&DispatchEvent::raw(child_id, RawEvent::pointer_enter())));
    let outside = tree.pointer_raw_moved(20, 8).expect("move outside");
    assert!(outside.contains(&DispatchEvent::raw(child_id, RawEvent::pointer_leave())));
    let back = tree.pointer_raw_moved(8, 8).expect("move back");
    assert!(back.contains(&DispatchEvent::raw(child_id, RawEvent::pointer_enter())));
}

#[test]
fn hover_switches_style_snapshot() {
    let base = style(Color::rgb(1, 2, 3));
    let hovered = style(Color::rgb(4, 5, 6));
    let view = Widget::column(vec![])
        .with_layout(fixed(50, 50))
        .with_styles(StyleSet::new(base).with(
            StyleState::Hovered,
            WidgetStylePatch {
                background: Some(hovered.background),
                ..WidgetStylePatch::default()
            },
        ));
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
        tree.pointer_raw_moved(10, 10).expect("move"),
        vec![DispatchEvent::raw(id, RawEvent::pointer_enter())]
    );
    tree.set_hovered(id, true);
    {
        let snapshot = tree.snapshot().expect("snapshot");
        let node = snapshot.node(id).expect("view");
        assert_eq!(node.style.background, hovered.background);
        assert!(node.state.hovered);
    }

    assert_eq!(
        tree.pointer_raw_leave(),
        vec![DispatchEvent::raw(id, RawEvent::pointer_leave())]
    );
    tree.set_hovered(id, false);
    let snapshot = tree.snapshot().expect("snapshot");
    let node = snapshot.node(id).expect("view");
    assert_eq!(node.style.background, base.background);
    assert!(!node.state.hovered);
}

#[test]
fn cursor_is_default_without_hover() {
    let view = Widget::column(vec![])
        .with_layout(fixed(50, 50))
        .with_styles(StyleSet::new(WidgetStyle {
            cursor: Cursor::Pointer,
            ..WidgetStyle::default()
        }));
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

    assert_eq!(tree.cursor(), Cursor::Default);
}

#[test]
fn cursor_follows_the_hovered_path() {
    let child = Widget::column(vec![])
        .with_layout(fixed(40, 20))
        .with_styles(StyleSet::new(WidgetStyle {
            cursor: Cursor::Text,
            ..WidgetStyle::default()
        }));
    let root = Widget::column(vec![child])
        .with_layout(fixed(100, 100))
        .with_styles(StyleSet::new(WidgetStyle {
            cursor: Cursor::Pointer,
            ..WidgetStyle::default()
        }));
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

    tree.pointer_raw_moved(5, 5).expect("move");
    assert_eq!(tree.cursor(), Cursor::Text);
    tree.pointer_raw_moved(60, 60).expect("move");
    assert_eq!(tree.cursor(), Cursor::Pointer);
    tree.pointer_raw_leave();
    assert_eq!(tree.cursor(), Cursor::Default);
}

#[test]
fn cursor_can_change_in_a_state() {
    let view = Widget::column(vec![])
        .with_layout(fixed(50, 50))
        .with_styles(StyleSet::new(WidgetStyle::default()).with(
            StyleState::Pressed,
            WidgetStylePatch {
                cursor: Some(Cursor::Grabbing),
                ..WidgetStylePatch::default()
            },
        ));
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

    tree.pointer_raw_moved(5, 5).expect("move");
    assert_eq!(tree.cursor(), Cursor::Default);
    tree.set_pressed(id, true);
    assert_eq!(tree.cursor(), Cursor::Grabbing);
}

#[test]
fn hover_switches_the_text_variant_without_relayout() {
    let styles = TextStyleSet::new(TextStyle::default()).with(
        StyleState::Hovered,
        TextStylePatch {
            align: Some(TextAlign::Right),
            ..TextStylePatch::default()
        },
    );
    let text = Widget::text_with_styles("wide text", styles).with_layout(fixed(80, 20));
    let id = text.id();
    let mut tree = WidgetTree::new(text);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 80,
            height: 20,
        },
        1.0,
    )
    .expect("layout");

    let base = {
        let snapshot = tree.snapshot().expect("snapshot");
        let node = snapshot.node(id).expect("text");
        node.text.clone().expect("text layout")
    };

    tree.set_hovered(id, true);
    let hovered = {
        let snapshot = tree.snapshot().expect("snapshot");
        let node = snapshot.node(id).expect("text");
        assert_eq!(node.width, 80);
        assert_eq!(node.height, 20);
        node.text.clone().expect("text layout")
    };
    assert!(hovered.glyphs[0].x > base.glyphs[0].x);

    tree.set_hovered(id, false);
    let after = {
        let snapshot = tree.snapshot().expect("snapshot");
        let node = snapshot.node(id).expect("text");
        node.text.clone().expect("text layout")
    };
    assert_eq!(after.glyphs[0].x, base.glyphs[0].x);
}

#[test]
fn checked_state_switches_style_and_toggles() {
    let base = style(Color::rgb(1, 2, 3));
    let checked = style(Color::rgb(7, 8, 9));
    let view = Widget::column(vec![])
        .with_layout(fixed(20, 20))
        .with_styles(StyleSet::new(base).with(
            StyleState::Checked,
            WidgetStylePatch {
                background: Some(checked.background),
                ..WidgetStylePatch::default()
            },
        ));
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
fn relayout_moves_an_untouched_branch_by_the_size_of_a_dirty_sibling() {
    let header = Widget::text("one two three four");
    let header_id = header.id();
    let inner = Widget::text("ok");
    let inner_id = inner.id();
    let body = Widget::column(vec![inner]).with_layout(fixed(40, 10));
    let body_id = body.id();
    let root = Widget::column(vec![header, body]).with_layout(LayoutStyle {
        width: Dimension::Px(40),
        ..LayoutStyle::default()
    });
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();
    let viewport = Size {
        width: 40,
        height: 200,
    };

    tree.layout(&mut engine, viewport, 1.0).expect("layout");
    let before = tree.snapshot().cloned().expect("snapshot");
    let header_before = before.node(header_id).expect("header").clone();
    let body_before = before.node(body_id).expect("body").clone();
    let inner_before = before.node(inner_id).expect("inner").clone();
    assert!(header_before.height > 20);

    assert!(tree.set_text(header_id, "one".to_string()));
    tree.layout(&mut engine, viewport, 1.0).expect("relayout");
    let after = tree.snapshot().cloned().expect("snapshot");
    let header_after = after.node(header_id).expect("header");
    let body_after = after.node(body_id).expect("body");
    let inner_after = after.node(inner_id).expect("inner");
    let shift = header_after.height as i32 - header_before.height as i32;
    assert!(shift < 0);

    assert_eq!(body_after.x, body_before.x);
    assert_eq!(body_after.y, body_before.y + shift);
    assert_eq!(body_after.width, body_before.width);
    assert_eq!(body_after.height, body_before.height);
    assert_eq!(inner_after.y, inner_before.y + shift);
    assert_eq!(inner_after.x, inner_before.x);
}

#[test]
fn set_image_replaces_the_texture_and_relayouts() {
    let image = Widget::image(Texture::empty());
    let id = image.id();
    let mut tree = WidgetTree::new(image);
    let mut engine = TextEngine::new();
    let viewport = Size {
        width: 10,
        height: 10,
    };

    tree.layout(&mut engine, viewport, 1.0).expect("layout");
    let texture = std::rc::Rc::new(Texture::from_rgba(2, 3, &[0u8; 24]).expect("texture"));
    assert!(tree.set_image(id, brul_gui::Image::Raster(std::rc::Rc::clone(&texture))));
    assert!(!tree.set_image(id, brul_gui::Image::Raster(texture)));
    tree.layout(&mut engine, viewport, 1.0).expect("relayout");
    let node = tree
        .snapshot()
        .expect("snapshot")
        .node(id)
        .cloned()
        .expect("node");

    assert_eq!((node.width, node.height), (2, 3));
}
