use brul_gui::{
    Color, Dimension, Dirty, Frame, LayoutStyle, Size, StyleSet, StyleState, TextEngine, Texture,
    Widget, WidgetStyle, WidgetTree,
};

fn view(color: Color, width: u32, height: u32) -> Widget {
    Widget::column(Vec::new())
        .with_layout(LayoutStyle {
            width: Dimension::Px(width),
            height: Dimension::Px(height),
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            background: color,
            ..WidgetStyle::default()
        }))
}

fn full_view(color: Color) -> Widget {
    Widget::column(Vec::new())
        .with_layout(LayoutStyle {
            width: Dimension::Px(16),
            height: Dimension::Px(16),
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            background: color,
            ..WidgetStyle::default()
        }))
}

fn pixel(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
    let index = ((y * frame.width() + x) * 4) as usize;
    let data = &frame.pixels()[index..index + 4];
    [data[0], data[1], data[2], data[3]]
}

#[test]
fn paints_background_and_rounds_corners() {
    let red = Color::rgb(255, 0, 0);
    let root = Widget::column(Vec::new())
        .with_layout(LayoutStyle {
            width: Dimension::Px(16),
            height: Dimension::Px(16),
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            background: red,
            radius: 8,
            ..WidgetStyle::default()
        }));
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

    assert_eq!(pixel(&frame, 8, 8), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 0, 0), [0, 0, 0, 0]);
}

#[test]
fn paints_text_ink() {
    let text = Widget::text("A").with_styles(StyleSet::new(WidgetStyle {
        text: Color::rgb(255, 0, 0),
        ..WidgetStyle::default()
    }));
    let mut engine = TextEngine::new();
    let mut tree = WidgetTree::new(text);
    tree.layout(
        &mut engine,
        Size {
            width: 64,
            height: 64,
        },
        1.0,
    )
    .expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");
    let mut frame = Frame::new(64, 64);

    frame.paint(snapshot, engine.atlas(), &Dirty::all());

    let inked = frame
        .pixels()
        .as_chunks::<4>()
        .0
        .iter()
        .any(|data| data[3] > 0 && data[0] > data[1]);
    assert!(inked, "text left no red ink");
}

#[test]
fn paints_tinted_image() {
    let texture = Texture::from_rgba(1, 1, &[255, 255, 255, 255]).expect("texture");
    let image = Widget::image(texture).with_styles(StyleSet::new(WidgetStyle {
        tint: Some(Color::rgb(255, 128, 0)),
        ..WidgetStyle::default()
    }));
    let mut engine = TextEngine::new();
    let mut tree = WidgetTree::new(image);
    tree.layout(
        &mut engine,
        Size {
            width: 8,
            height: 8,
        },
        1.0,
    )
    .expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");
    let mut frame = Frame::new(8, 8);

    frame.paint(snapshot, engine.atlas(), &Dirty::all());

    assert_eq!(pixel(&frame, 0, 0), [255, 128, 0, 255]);
}

#[test]
fn opacity_produces_premultiplied_pixels() {
    let texture = Texture::from_rgba(1, 1, &[255, 255, 255, 255]).expect("texture");
    let image = Widget::image(texture).with_styles(StyleSet::new(WidgetStyle {
        opacity: 0.5,
        ..WidgetStyle::default()
    }));
    let mut engine = TextEngine::new();
    let mut tree = WidgetTree::new(image);
    tree.layout(
        &mut engine,
        Size {
            width: 8,
            height: 8,
        },
        1.0,
    )
    .expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");
    let mut frame = Frame::new(8, 8);

    frame.paint(snapshot, engine.atlas(), &Dirty::all());

    assert_eq!(pixel(&frame, 0, 0), [128, 128, 128, 128]);
}

#[test]
fn dirty_region_leaves_the_rest_of_the_frame_alone() {
    let red = Color::rgb(255, 0, 0);
    let blue = Color::rgb(0, 0, 255);
    let first = view(red, 8, 8);
    let second = view(red, 8, 8).with_styles(
        StyleSet::new(WidgetStyle {
            background: red,
            ..WidgetStyle::default()
        })
        .with(
            StyleState::Checked,
            WidgetStyle {
                background: blue,
                ..WidgetStyle::default()
            },
        ),
    );
    let second_id = second.id();
    let root = Widget::column(vec![first, second]).with_layout(LayoutStyle {
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
    let mut frame = Frame::new(16, 16);
    let dirty = tree.take_dirty();
    let snapshot = tree.snapshot().expect("snapshot");
    frame.paint(snapshot, engine.atlas(), &dirty);
    assert_eq!(pixel(&frame, 0, 0), [255, 0, 0, 255]);

    tree.set_checked(second_id, true);
    let dirty = tree.take_dirty();
    assert!(!dirty.full);
    assert!(dirty.ids.contains(&second_id));
    let snapshot = tree.snapshot().expect("snapshot");
    frame.paint(snapshot, engine.atlas(), &dirty);

    assert_eq!(pixel(&frame, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 0, 8), [0, 0, 255, 255]);
}

#[test]
fn full_repaint_replaces_previous_pixels() {
    let mut engine = TextEngine::new();
    let mut first = WidgetTree::new(full_view(Color::rgb(255, 0, 0)));
    first
        .layout(
            &mut engine,
            Size {
                width: 16,
                height: 16,
            },
            1.0,
        )
        .expect("layout");
    let mut frame = Frame::new(16, 16);
    let snapshot = first.snapshot().expect("snapshot");
    frame.paint(snapshot, engine.atlas(), &Dirty::all());

    let mut second = WidgetTree::new(full_view(Color::rgb(0, 0, 255)));
    second
        .layout(
            &mut engine,
            Size {
                width: 16,
                height: 16,
            },
            1.0,
        )
        .expect("layout");
    let snapshot = second.snapshot().expect("snapshot");
    frame.paint(snapshot, engine.atlas(), &Dirty::all());

    assert_eq!(pixel(&frame, 0, 0), [0, 0, 255, 255]);
}
