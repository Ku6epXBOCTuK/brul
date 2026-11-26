use super::*;
#[test]
fn paints_a_shadow_below_its_box() {
    let shadow = Shadow {
        x: 0,
        y: 4,
        blur: 0,
        color: Color::rgb(255, 0, 0),
    };
    let (_tree, _engine, frame) =
        painted(padded_root(shadowed_child(shadow, Color::rgb(0, 0, 255))));

    assert_eq!(pixel(&frame, 12, 12), [0, 0, 255, 255]);
    assert_eq!(pixel(&frame, 12, 18), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 12, 21), [0, 0, 0, 0]);
}

#[test]
fn shadow_blur_fades_with_distance() {
    let shadow = Shadow {
        x: 0,
        y: 0,
        blur: 2,
        color: Color::rgb(0, 0, 0),
    };
    let (_tree, _engine, frame) =
        painted(padded_root(shadowed_child(shadow, Color::rgb(0, 0, 255))));

    let faded = pixel(&frame, 12, 17)[3];
    assert!(faded > 0, "blurred edge is transparent");
    assert!(faded < 80, "blurred edge is too opaque: {faded}");
    assert_eq!(pixel(&frame, 12, 0), [0, 0, 0, 0]);
}

#[test]
fn removing_a_hovered_shadow_clears_its_tail() {
    let blue = Color::rgb(0, 0, 255);
    let red = Color::rgb(255, 0, 0);
    let base_shadow = Shadow {
        x: 0,
        y: 4,
        blur: 0,
        color: Color::TRANSPARENT,
    };
    let child = shadowed_child(base_shadow, blue);
    let hovered = WidgetStyle {
        background: Paint::Color(blue),
        shadow: Shadow {
            color: red,
            ..base_shadow
        },
        ..WidgetStyle::default()
    };
    let child = child.with_styles(
        StyleSet::new(WidgetStyle {
            background: Paint::Color(blue),
            shadow: base_shadow,
            ..WidgetStyle::default()
        })
        .with(
            StyleState::Hovered,
            WidgetStylePatch {
                background: Some(hovered.background),
                shadow: Some(hovered.shadow.color),
                ..WidgetStylePatch::default()
            },
        ),
    );
    let child_id = child.id();
    let (mut tree, engine, mut frame) = painted(padded_root(child));

    tree.set_hovered(child_id, true);
    let dirty = tree.take_dirty();
    assert!(dirty.ids.contains(&child_id));
    let snapshot = tree.snapshot().expect("snapshot");
    frame.paint(snapshot, engine.atlas(), &dirty);
    assert_eq!(pixel(&frame, 12, 18), [255, 0, 0, 255]);

    tree.set_hovered(child_id, false);
    let dirty = tree.take_dirty();
    let snapshot = tree.snapshot().expect("snapshot");
    frame.paint(snapshot, engine.atlas(), &dirty);
    assert_eq!(pixel(&frame, 12, 18), [0, 0, 0, 0]);
}

#[test]
fn a_dirty_parent_repaints_its_child() {
    let child = view(Color::rgb(0, 255, 0), 16, 16);
    let child_id = child.id();
    let root = padded_root(child);
    let root_id = root.id();
    let (mut tree, engine, mut frame) = painted(root);

    tree.mark_dirty(root_id);
    let dirty = tree.take_dirty();
    assert!(dirty.ids.contains(&child_id));

    let snapshot = tree.snapshot().expect("snapshot");
    frame.paint(snapshot, engine.atlas(), &dirty);
    assert_eq!(pixel(&frame, 16, 16), [0, 255, 0, 255]);
}

#[test]
fn clips_children_to_a_rounded_parent() {
    let child = Widget::column(Vec::new())
        .with_layout(LayoutStyle {
            width: Dimension::Px(32),
            height: Dimension::Px(16),
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            background: Paint::Color(Color::rgb(255, 0, 0)),
            ..Default::default()
        }));
    let parent = Widget::column(vec![child])
        .with_layout(LayoutStyle {
            width: Dimension::Px(16),
            height: Dimension::Px(16),
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            clip: true,
            border_radius: 4,
            ..Default::default()
        }));
    let mut tree = WidgetTree::new(parent);
    let mut engine = TextEngine::new();
    tree.layout(
        &mut engine,
        Size {
            width: 32,
            height: 32,
        },
        1.0,
    )
    .expect("layout");
    let mut frame = Frame::new(32, 32);
    frame.paint(
        tree.snapshot().expect("snapshot"),
        engine.atlas(),
        &Dirty::all(),
    );

    assert_eq!(pixel(&frame, 8, 8), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 20, 8), [0, 0, 0, 0]);
    assert_eq!(pixel(&frame, 0, 0), [0, 0, 0, 0]);
}

#[test]
fn clip_keeps_a_growing_text_child_inside_its_box() {
    let text = Widget::text("x".repeat(20) + "\n" + &"x".repeat(20)).with_styles(StyleSet::new(
        WidgetStyle {
            text: Color::rgb(255, 0, 0),
            ..WidgetStyle::default()
        },
    ));
    let body = Widget::column(vec![text])
        .with_layout(LayoutStyle {
            width: Dimension::Px(100),
            flex_grow: 1.0,
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            clip: true,
            ..WidgetStyle::default()
        }));
    let root = Widget::column(vec![body]).with_layout(LayoutStyle {
        width: Dimension::Px(100),
        height: Dimension::Px(40),
        ..LayoutStyle::default()
    });
    let mut engine = TextEngine::new();
    let mut tree = WidgetTree::new(root);
    tree.layout(
        &mut engine,
        Size {
            width: 100,
            height: 80,
        },
        1.0,
    )
    .expect("layout");
    let mut frame = Frame::new(100, 80);
    frame.paint(
        tree.snapshot().expect("snapshot"),
        engine.atlas(),
        &Dirty::all(),
    );

    let ink = (0..80).any(|y| pixel(&frame, 4, y)[3] > 0);
    assert!(ink, "the text should be drawn inside the clip box");
    let outside = (44..80).any(|y| (0..100).any(|x| pixel(&frame, x, y)[3] > 0));
    assert!(!outside, "ink must not escape the 40px clip box");
}

#[test]
fn layer_paints_above_content_declared_after_it() {
    let layer = Widget::layer(Vec::new()).with_styles(StyleSet::new(WidgetStyle {
        background: Paint::Color(Color::rgb(0, 0, 255)),
        ..WidgetStyle::default()
    }));
    let content = full_view(Color::rgb(255, 0, 0));
    let root = Widget::column(vec![layer, content]).with_layout(LayoutStyle {
        width: Dimension::Px(16),
        height: Dimension::Px(16),
        ..LayoutStyle::default()
    });
    let mut engine = TextEngine::new();
    let mut tree = WidgetTree::new(root);
    tree.layout(
        &mut engine,
        Size {
            width: 16,
            height: 16,
        },
        1.0,
    )
    .expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");
    let mut frame = Frame::new(16, 16);

    frame.paint(snapshot, engine.atlas(), &Dirty::all());

    assert_eq!(pixel(&frame, 8, 8), [0, 0, 255, 255]);
}
