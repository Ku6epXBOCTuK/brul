use std::time::Duration;

use brul_gui::{
    Class, Color, Cursor, Dimension, Direction, DispatchEvent, KeyModifiers, Layers, LayoutPatch,
    LayoutStyle, Padding, Paint, RawEvent, Size, StyleSet, StyleState, TextAlign, TextEngine,
    TextStyle, TextStylePatch, TextStyleSet, Texture, Transitions, Widget, WidgetError, WidgetId,
    WidgetStyle, WidgetStylePatch, WidgetTree,
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
        background: Paint::Color(background),
        ..WidgetStyle::default()
    }
}

#[test]
#[cfg(debug_assertions)]
fn debug_path_uses_element_names_and_ordinals() {
    use brul_gui::EventTarget;

    let first = Widget::text("a").debug_named("Text");
    let second = Widget::text("b").debug_named("Text");
    let second_id = second.id();
    let root = Widget::column(vec![first, second])
        .debug_named("Container")
        .debug_source("demo.brul");
    let tree = WidgetTree::new(root);

    assert_eq!(
        tree.debug_path(second_id),
        format!(
            "Widget({}) demo.brul > Container:1st > Text:2nd",
            second_id.get()
        )
    );
    assert_eq!(tree.debug_target(EventTarget::Window), "Window");
}

#[test]
#[cfg(debug_assertions)]
fn debug_path_uses_the_nearest_view_source() {
    let label = Widget::text("x").debug_named("Text");
    let label_id = label.id();
    let button = Widget::row(vec![label])
        .debug_named("Button")
        .debug_source("button.brul");
    let root = Widget::column(vec![button])
        .debug_named("Container")
        .debug_source("gallery.brul");
    let tree = WidgetTree::new(root);

    assert!(
        tree.debug_path(label_id)
            .starts_with(&format!("Widget({}) button.brul", label_id.get()))
    );
}

#[test]
#[cfg(debug_assertions)]
fn debug_path_counts_every_sibling() {
    let children = (0..21)
        .map(|_| Widget::text("x").debug_named("Text"))
        .collect::<Vec<_>>();
    let eleventh = children[10].id();
    let twenty_first = children[20].id();
    let root = Widget::column(children)
        .debug_named("Container")
        .debug_source("demo.brul");
    let tree = WidgetTree::new(root);

    assert!(tree.debug_path(eleventh).ends_with("Text:11th"));
    assert!(tree.debug_path(twenty_first).ends_with("Text:21st"));
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

    let events = tree.pointer_raw_down(5, 5).expect("down");
    assert!(events.contains(&DispatchEvent::raw(text_id, RawEvent::drag(5, 5))));
}

#[test]
fn pointer_move_reaches_the_pressed_path_with_modifiers() {
    let text = Widget::text("ok").with_layout(fixed(80, 80));
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

    let modifiers = KeyModifiers {
        shift: true,
        ..KeyModifiers::default()
    };
    tree.set_modifiers(modifiers);
    tree.pointer_raw_down(5, 5).expect("down");

    let moved = tree.pointer_raw_moved(7, 9).expect("move");
    for id in [text_id, child_id, root_id] {
        assert!(moved.contains(&DispatchEvent::raw(
            id,
            RawEvent::pointer_move(7, 9, modifiers)
        )));
    }

    tree.pointer_raw_up(5, 5).expect("up");
    let moved = tree.pointer_raw_moved(7, 9).expect("move");
    assert!(!moved.iter().any(|event| matches!(
        event,
        DispatchEvent::Raw(raw) if raw.event.kind() == brul_gui::RawKind::PointerMove
    )));
}

#[test]
fn a_release_carries_the_up_point_and_modifiers() {
    let child = Widget::column(vec![]).with_layout(fixed(80, 80));
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

    let modifiers = KeyModifiers {
        shift: true,
        ..KeyModifiers::default()
    };
    tree.set_modifiers(modifiers);
    tree.pointer_raw_down(5, 5).expect("down");

    let up = tree.pointer_raw_up(7, 9).expect("up");
    for id in [child_id, root_id] {
        assert!(up.contains(&DispatchEvent::raw(id, RawEvent::release(7, 9, modifiers))));
    }
}

#[test]
fn leaving_the_window_cancels_a_press_instead_of_releasing_it() {
    let child = Widget::column(vec![]).with_layout(fixed(80, 80));
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

    tree.pointer_raw_down(5, 5).expect("down");
    let left = tree.pointer_raw_leave();
    for id in [child_id, root_id] {
        assert!(left.contains(&DispatchEvent::raw(id, RawEvent::pointer_cancel())));
    }
    assert!(!left.iter().any(|event| matches!(
        event,
        DispatchEvent::Raw(raw) if raw.event.kind() == brul_gui::RawKind::Release
    )));
}

#[test]
fn children_are_placed_inside_padding_and_gap() {
    let first = Widget::column(vec![]).with_layout(fixed(40, 20));
    let first_id = first.id();
    let second = Widget::column(vec![]).with_layout(fixed(40, 20));
    let second_id = second.id();
    let root = Widget::column(vec![first, second]).with_layout(LayoutStyle {
        width: Dimension::Px(200),
        height: Dimension::Px(100),
        padding: Padding::all(10),
        gap: 5,
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
fn percent_width_spans_a_generated_structure_wrapper() {
    let item = Widget::column(Vec::new()).with_layout(LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(24),
        ..LayoutStyle::default()
    });
    let item_id = item.id();
    let rows = Widget::structure(vec![item]);
    let root = Widget::column(vec![rows]).with_layout(LayoutStyle {
        width: Dimension::Px(488),
        height: Dimension::Px(400),
        padding: Padding::all(12),
        ..LayoutStyle::default()
    });
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();

    tree.layout(
        &mut engine,
        Size {
            width: 488,
            height: 400,
        },
        1.0,
    )
    .expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");
    let item = snapshot.node(item_id).expect("item");

    assert_eq!(item.width, 464);
    assert_eq!(item.x, 12);
}

#[test]
fn a_structure_group_is_not_a_box() {
    let first = Widget::view(Direction::Row, Vec::new()).with_layout(LayoutStyle {
        height: Dimension::Px(20),
        ..LayoutStyle::default()
    });
    let first_id = first.id();
    let empty = Widget::structure(Vec::new());
    let second = Widget::view(Direction::Row, Vec::new()).with_layout(LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(20),
        ..LayoutStyle::default()
    });
    let second_id = second.id();
    let group = Widget::structure(vec![second]);
    let root = Widget::column(vec![first, empty, group]).with_layout(LayoutStyle {
        width: Dimension::Px(200),
        gap: 10,
        ..LayoutStyle::default()
    });
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
    let snapshot = tree.snapshot().expect("snapshot");
    let first = snapshot.node(first_id).expect("first");
    let second = snapshot.node(second_id).expect("second");

    assert_eq!((first.x, first.y), (0, 0));
    assert_eq!(
        (second.x, second.y),
        (0, 30),
        "an empty group takes a gap slot and pushes its sibling down"
    );
    assert_eq!(second.width, 200, "a group does not bound a percent child");
}

#[test]
fn text_in_an_auto_container_shapes_inside_its_padding() {
    let sentence =
        "aa bb cc dd ee ff gg hh ii jj kk ll mm nn oo pp qq rr ss tt uu vv ww xx yy zz ".repeat(3);
    let reference = Widget::text(sentence.clone()).with_layout(LayoutStyle {
        width: Dimension::Px(360),
        ..LayoutStyle::default()
    });
    let reference_id = reference.id();
    let reference_root = Widget::column(vec![reference]).with_layout(fixed(400, 400));

    let text = Widget::text(sentence).with_layout(LayoutStyle::default());
    let text_id = text.id();
    let padded = Widget::view(Direction::Column, vec![text]).with_layout(LayoutStyle {
        padding: Padding::all(20),
        ..LayoutStyle::default()
    });
    let root = Widget::column(vec![padded]).with_layout(fixed(400, 400));

    let mut engine = TextEngine::new();
    let mut reference_tree = WidgetTree::new(reference_root);
    let mut tree = WidgetTree::new(root);
    for tree in [&mut reference_tree, &mut tree] {
        tree.layout(
            &mut engine,
            Size {
                width: 400,
                height: 400,
            },
            1.0,
        )
        .expect("layout");
    }
    let reference_snapshot = reference_tree.snapshot().expect("snapshot");
    let snapshot = tree.snapshot().expect("snapshot");
    let reference_node = reference_snapshot.node(reference_id).expect("reference");
    let node = snapshot.node(text_id).expect("text");

    assert_eq!(node.height, reference_node.height);
    let text = node.text.as_ref().expect("text layout");
    let reference_text = reference_node.text.as_ref().expect("text layout");
    assert_eq!(text.size, reference_text.size);
    assert!(text.size.width <= 360);
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
    assert!(node.image.is_some());
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
