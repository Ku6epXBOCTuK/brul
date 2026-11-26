#[path = "widget/layer_cases.rs"]
mod layer_cases;
#[path = "widget/layout_cases.rs"]
mod layout_cases;
#[path = "widget/style_cases.rs"]
mod style_cases;

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
