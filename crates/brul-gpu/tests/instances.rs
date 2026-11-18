use brul_gpu::{clip_set, decoration_instances, shape_instances};
use brul_gui::{
    CaretPosition, ClipRect, Color, Dimension, Dirty, Gradient, LayoutStyle, Paint, SelectionRect,
    Shadow, Size, StyleSet, TextEngine, Widget, WidgetKind, WidgetStyle, WidgetTree,
};

fn snapshot(style: WidgetStyle, border_width: u32) -> brul_gui::WidgetSnapshot {
    let root = Widget::column(Vec::new())
        .with_layout(LayoutStyle {
            width: Dimension::Px(20),
            height: Dimension::Px(20),
            border_width,
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(style));
    let mut engine = TextEngine::new();
    let mut tree = WidgetTree::new(root);
    tree.layout(
        &mut engine,
        Size {
            width: 20,
            height: 20,
        },
        1.0,
    )
    .expect("layout");
    tree.snapshot().expect("snapshot").clone()
}

fn two_node_snapshot() -> brul_gui::WidgetSnapshot {
    let solid = || {
        Widget::column(Vec::new())
            .with_layout(LayoutStyle {
                width: Dimension::Px(10),
                height: Dimension::Px(10),
                ..LayoutStyle::default()
            })
            .with_styles(StyleSet::new(WidgetStyle {
                background: Paint::Color(Color::rgb(255, 0, 0)),
                ..WidgetStyle::default()
            }))
    };
    let mut engine = TextEngine::new();
    let mut tree = WidgetTree::new(Widget::column(vec![solid(), solid()]));
    tree.layout(
        &mut engine,
        Size {
            width: 64,
            height: 64,
        },
        1.0,
    )
    .expect("layout");
    tree.snapshot().expect("snapshot").clone()
}

#[test]
fn a_dirty_filter_builds_only_changed_shapes() {
    let snapshot = two_node_snapshot();
    let full = shape_instances(&snapshot, &brul_gpu::ClipSet::default(), &Dirty::all());
    assert_eq!(full.len(), 2);
    let dirty = Dirty {
        full: false,
        ids: vec![snapshot.nodes.last().expect("node").id],
    };

    let shapes = shape_instances(&snapshot, &brul_gpu::ClipSet::default(), &dirty);

    assert_eq!(shapes, vec![*full.last().expect("shape")]);
}

#[test]
fn a_dirty_filter_leaves_other_clip_ranges_empty() {
    let mut snapshot = two_node_snapshot();
    snapshot.nodes[1].clips = vec![ClipRect::new(1, 2, 3, 4, 0)];
    let dirty = Dirty {
        full: false,
        ids: vec![snapshot.nodes[1].id],
    };

    let clips = clip_set(&snapshot, &dirty);

    assert_eq!(clips.clips.len(), 1);
    assert_eq!(clips.ranges.len(), snapshot.nodes.len());
    assert_eq!(clips.ranges[0], brul_gpu::ClipRange::default());
    assert_eq!(
        clips.ranges[1],
        brul_gpu::ClipRange {
            offset: 0,
            count: 1
        }
    );
}

#[test]
fn the_shader_parses_and_validates() {
    let module = naga::front::wgsl::parse_str(brul_gpu::SHADER).expect("WGSL parses");
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("WGSL validates");
}

#[test]
fn a_solid_node_becomes_one_shape_with_border_coverage() {
    let snapshot = snapshot(
        WidgetStyle {
            background: Paint::Color(Color::rgb(255, 0, 0)),
            border: Color::rgb(0, 0, 255),
            border_radius: 4,
            opacity: 0.5,
            ..WidgetStyle::default()
        },
        2,
    );

    let shapes = shape_instances(&snapshot, &brul_gpu::ClipSet::default(), &Dirty::all());
    assert_eq!(shapes.len(), 1);
    assert_eq!(shapes[0].bounds, [0.0, 0.0, 20.0, 20.0]);
    assert_eq!(shapes[0].params, [4.0, 2.0, 0.0, 0.5]);
    assert_eq!(shapes[0].fill, [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(shapes[0].border, [0.0, 0.0, 1.0, 1.0]);
    assert_eq!(shapes[0].extra, [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn a_shadow_adds_a_separate_padded_instance() {
    let snapshot = snapshot(
        WidgetStyle {
            background: Paint::Color(Color::rgb(255, 0, 0)),
            shadow: Shadow {
                x: 2,
                y: 3,
                blur: 6,
                color: Color::rgba(0, 0, 0, 128),
            },
            ..WidgetStyle::default()
        },
        0,
    );

    let shapes = shape_instances(&snapshot, &brul_gpu::ClipSet::default(), &Dirty::all());
    assert_eq!(shapes.len(), 2);
    assert_eq!(shapes[0].extra, [0.0, 2.0, 3.0, 2.0]);
    assert_eq!(shapes[0].padding[0], 7.0);
    assert_eq!(shapes[0].params[2], 6.0);
    assert_eq!(shapes[1].extra[3], 0.0);
}

#[test]
fn clip_ranges_are_flattened_and_encoded_into_shapes() {
    let mut first = snapshot(
        WidgetStyle {
            background: Paint::Color(Color::rgb(255, 0, 0)),
            ..WidgetStyle::default()
        },
        0,
    );
    first.nodes.push(first.nodes[0].clone());
    first.nodes[1].clips = vec![
        ClipRect::new(1, 2, 30, 40, 5),
        ClipRect::new(3, 4, 10, 20, 0),
    ];

    let clips = clip_set(&first, &Dirty::all());
    assert_eq!(clips.clips.len(), 2);
    assert_eq!(clips.clips[0].bounds, [1.0, 2.0, 30.0, 40.0]);
    assert_eq!(clips.clips[0].radius[0], 5.0);
    assert_eq!(clips.clips[1].bounds, [3.0, 4.0, 10.0, 20.0]);
    assert_eq!(
        clips.ranges[0],
        brul_gpu::ClipRange {
            offset: 0,
            count: 0
        }
    );
    assert_eq!(
        clips.ranges[1],
        brul_gpu::ClipRange {
            offset: 0,
            count: 2
        }
    );

    let shapes = shape_instances(&first, &clips, &Dirty::all());
    assert_eq!(shapes.len(), 2);
    assert_eq!(shapes[0].padding, [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(shapes[1].padding, [1.0, 0.0, 2.0, 0.0]);
}

#[test]
fn input_decorations_become_unfilled_shapes() {
    let mut snapshot = snapshot(WidgetStyle::default(), 0);
    let node = &mut snapshot.nodes[0];
    node.kind = WidgetKind::Input;
    node.selection = vec![SelectionRect {
        x: 1,
        y: 2,
        width: 5,
        height: 3,
    }];
    node.caret = Some(CaretPosition {
        x: 3,
        y: 4,
        height: 7,
    });
    let clips = clip_set(&snapshot, &Dirty::all());

    let shapes = decoration_instances(&snapshot, &clips, &Dirty::all());
    assert_eq!(shapes.len(), 2);
    assert_eq!(shapes[0].bounds, [1.0, 2.0, 5.0, 3.0]);
    assert_eq!(shapes[0].fill[3], 60.0 / 255.0);
    assert_eq!(shapes[1].bounds, [3.0, 4.0, 1.0, 7.0]);
    assert_eq!(shapes[1].fill[3], 1.0);
    assert_eq!(shapes[1].params[1], 0.0);
    assert_eq!(shapes[1].padding[0], 0.0);
}

#[test]
fn a_gradient_sets_the_angle_and_end_colors() {
    let snapshot = snapshot(
        WidgetStyle {
            background: Paint::Gradient(Gradient::Linear {
                angle: 90.0,
                from: Color::rgb(10, 20, 30),
                to: Color::rgb(40, 50, 60),
            }),
            ..WidgetStyle::default()
        },
        0,
    );

    let shapes = shape_instances(&snapshot, &brul_gpu::ClipSet::default(), &Dirty::all());
    assert_eq!(shapes.len(), 1);
    assert_eq!(shapes[0].extra[0], 90.0);
    assert_eq!(shapes[0].from[0], 10.0 / 255.0);
    assert_eq!(shapes[0].to[0], 40.0 / 255.0);
}
