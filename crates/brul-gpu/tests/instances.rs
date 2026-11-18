use brul_gpu::shape_instances;
use brul_gui::{
    Color, Dimension, Gradient, LayoutStyle, Paint, Shadow, Size, StyleSet, TextEngine, Widget,
    WidgetStyle, WidgetTree,
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

    let shapes = shape_instances(&snapshot);
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

    let shapes = shape_instances(&snapshot);
    assert_eq!(shapes.len(), 2);
    assert_eq!(shapes[0].extra, [0.0, 2.0, 3.0, 2.0]);
    assert_eq!(shapes[0].padding[0], 7.0);
    assert_eq!(shapes[0].params[2], 6.0);
    assert_eq!(shapes[1].extra[3], 0.0);
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

    let shapes = shape_instances(&snapshot);
    assert_eq!(shapes.len(), 1);
    assert_eq!(shapes[0].extra[0], 90.0);
    assert_eq!(shapes[0].from[0], 10.0 / 255.0);
    assert_eq!(shapes[0].to[0], 40.0 / 255.0);
}
