use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use brul_gui::{
    Align, Dimension, Direction, Justify, LayoutStyle, Padding, Size, TextEngine, TextStyle,
    Widget, WidgetTree,
};
use criterion::{Criterion, Throughput};

const VIEWPORT: Size = Size {
    width: 520,
    height: 480,
};
const SCALE: f32 = 1.0;
const PASSES: usize = 100;

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: AllocLayout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: AllocLayout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: AllocLayout, size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

struct Tree {
    widget: Widget,
    nodes: usize,
}

fn leaf(label: u32, height: u32, style: LayoutStyle) -> Tree {
    Tree {
        widget: Widget::styled_text(label.to_string(), TextStyle::new(height as f32))
            .with_layout(style),
        nodes: 1,
    }
}

fn group(direction: Direction, children: Vec<Tree>, style: LayoutStyle) -> Tree {
    let nodes = 1 + children.iter().map(|child| child.nodes).sum::<usize>();
    Tree {
        widget: Widget::view(
            direction,
            children.into_iter().map(|child| child.widget).collect(),
        )
        .with_layout(style),
        nodes,
    }
}

fn preset(label: u32) -> Tree {
    let boxed = LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(30),
        padding: Padding {
            left: 12,
            right: 12,
            ..Padding::default()
        },
        border_width: 1,
        align: Align::Center,
        ..LayoutStyle::default()
    };
    group(
        Direction::Row,
        vec![leaf(label, 18, LayoutStyle::default())],
        boxed,
    )
}

fn preset_column(padding_right: u32) -> Tree {
    let style = LayoutStyle {
        width: Dimension::Percent(50.0),
        gap: 8,
        padding: Padding {
            right: padding_right,
            ..Padding::default()
        },
        ..LayoutStyle::default()
    };
    group(
        Direction::Column,
        (0..4).map(|index| preset(100 + index)).collect(),
        style,
    )
}

fn action(width: u32) -> Tree {
    let style = LayoutStyle {
        height: Dimension::Px(34),
        padding: Padding {
            left: width,
            right: width,
            ..Padding::default()
        },
        border_width: 1,
        align: Align::Center,
        justify: Justify::Center,
        ..LayoutStyle::default()
    };
    group(
        Direction::Row,
        vec![leaf(60, 18, LayoutStyle::default())],
        style,
    )
}

fn window() -> Tree {
    let root = LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Percent(100.0),
        padding: Padding::all(18),
        border_width: 2,
        gap: 14,
        ..LayoutStyle::default()
    };
    let header = LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(32),
        gap: 12,
        justify: Justify::SpaceBetween,
        align: Align::Center,
        ..LayoutStyle::default()
    };
    let editor = LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(120),
        padding: Padding::all(12),
        border_width: 1,
        flex_grow: 1.0,
        ..LayoutStyle::default()
    };
    let presets = LayoutStyle {
        width: Dimension::Percent(100.0),
        ..LayoutStyle::default()
    };
    let actions = LayoutStyle {
        width: Dimension::Percent(100.0),
        gap: 10,
        align: Align::Center,
        ..LayoutStyle::default()
    };
    let afk_row = LayoutStyle {
        width: Dimension::Percent(100.0),
        gap: 12,
        align: Align::Center,
        ..LayoutStyle::default()
    };
    let afk_toggle = LayoutStyle {
        height: Dimension::Px(28),
        gap: 10,
        align: Align::Center,
        ..LayoutStyle::default()
    };
    let afk_box = LayoutStyle {
        width: Dimension::Px(20),
        height: Dimension::Px(20),
        border_width: 2,
        align: Align::Center,
        justify: Justify::Center,
        ..LayoutStyle::default()
    };
    let afk_input = LayoutStyle {
        height: Dimension::Px(30),
        padding: Padding {
            left: 10,
            right: 10,
            ..Padding::default()
        },
        border_width: 1,
        flex_grow: 1.0,
        ..LayoutStyle::default()
    };

    group(
        Direction::Column,
        vec![
            group(
                Direction::Row,
                vec![
                    leaf(48, 24, LayoutStyle::default()),
                    leaf(80, 18, LayoutStyle::default()),
                ],
                header,
            ),
            leaf(484, 120, editor),
            group(
                Direction::Row,
                vec![preset_column(5), preset_column(0)],
                presets,
            ),
            group(Direction::Row, vec![action(18), action(14)], actions),
            group(
                Direction::Row,
                vec![
                    group(
                        Direction::Row,
                        vec![
                            group(
                                Direction::Column,
                                vec![leaf(12, 12, LayoutStyle::default())],
                                afk_box,
                            ),
                            leaf(150, 18, LayoutStyle::default()),
                        ],
                        afk_toggle,
                    ),
                    leaf(200, 30, afk_input),
                ],
                afk_row,
            ),
            leaf(200, 18, LayoutStyle::default()),
            leaf(240, 16, LayoutStyle::default()),
        ],
        root,
    )
}

fn synthetic(rows: usize, per_row: usize) -> Tree {
    let row = LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(24),
        gap: 4,
        ..LayoutStyle::default()
    };
    let cell = LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(16),
        ..LayoutStyle::default()
    };
    group(
        Direction::Column,
        (0..rows)
            .map(|_| {
                group(
                    Direction::Row,
                    (0..per_row).map(|_| leaf(10, 16, cell)).collect(),
                    row,
                )
            })
            .collect(),
        LayoutStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Percent(100.0),
            gap: 2,
            ..LayoutStyle::default()
        },
    )
}

struct Bench {
    tree: WidgetTree,
    engine: TextEngine,
    nodes: usize,
}

fn prepare(tree: Tree) -> Bench {
    let nodes = tree.nodes;
    let mut tree = WidgetTree::new(tree.widget);
    let mut engine = TextEngine::new();
    for _ in 0..2 {
        tree.layout(&mut engine, VIEWPORT, SCALE)
            .expect("bench tree is valid");
    }
    Bench {
        tree,
        engine,
        nodes,
    }
}

fn report_footprint(name: &str, bench: &mut Bench) {
    let allocations = ALLOCATIONS.load(Ordering::Relaxed);
    let bytes = BYTES.load(Ordering::Relaxed);
    for _ in 0..PASSES {
        bench
            .tree
            .layout(black_box(&mut bench.engine), VIEWPORT, SCALE)
            .expect("bench tree is valid");
    }
    let allocations = (ALLOCATIONS.load(Ordering::Relaxed) - allocations) as f64 / PASSES as f64;
    let bytes = (BYTES.load(Ordering::Relaxed) - bytes) as f64 / PASSES as f64;
    println!("{name}: {allocations:.1} allocations, {bytes:.0} requested bytes per frame");
}

fn measure(criterion: &mut Criterion) {
    let mut window = prepare(window());
    report_footprint("window", &mut window);

    let mut synthetic = prepare(synthetic(25, 19));
    report_footprint("synthetic-500", &mut synthetic);

    let mut group = criterion.benchmark_group("layout");

    group.throughput(Throughput::Elements(window.nodes as u64));
    group.bench_function("window", |bencher| {
        bencher.iter(|| {
            window
                .tree
                .layout(black_box(&mut window.engine), VIEWPORT, SCALE)
                .expect("bench tree is valid")
        });
    });

    group.throughput(Throughput::Elements(synthetic.nodes as u64));
    group.bench_function("synthetic-500", |bencher| {
        bencher.iter(|| {
            synthetic
                .tree
                .layout(black_box(&mut synthetic.engine), VIEWPORT, SCALE)
                .expect("bench tree is valid")
        });
    });

    group.finish();
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    measure(&mut criterion);
    criterion.final_summary();
}
