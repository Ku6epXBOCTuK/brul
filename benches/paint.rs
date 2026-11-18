use std::hint::black_box;

use brul_gui::{
    Color, Dimension, Dirty, Frame, LayoutStyle, Paint, Size, StyleSet, TextEngine, Widget,
    WidgetStyle, WidgetTree,
};
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

fn paint_full_frame(c: &mut Criterion) {
    let root = Widget::column(Vec::new())
        .with_layout(LayoutStyle {
            width: Dimension::Px(WIDTH),
            height: Dimension::Px(HEIGHT),
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            background: Paint::Color(Color::rgb(20, 30, 40)),
            border_radius: 24,
            ..WidgetStyle::default()
        }));
    let mut engine = TextEngine::new();
    let mut tree = WidgetTree::new(root);
    tree.layout(
        &mut engine,
        Size {
            width: WIDTH,
            height: HEIGHT,
        },
        1.0,
    )
    .expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");
    let dirty = Dirty::all();
    let mut frame = Frame::new(WIDTH, HEIGHT);

    c.bench_function("paint full frame 1920x1080", |b| {
        b.iter_batched(
            || (),
            |()| {
                frame.paint(snapshot, engine.atlas(), &dirty);
                black_box(frame.pixels().as_ptr());
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, paint_full_frame);
criterion_main!(benches);
