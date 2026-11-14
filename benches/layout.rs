use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use brul_gui::{Align, Dimension, Justify, LayoutNode, LayoutStyle, Padding, Size, layout};
use criterion::{Criterion, Throughput};

const VIEWPORT: Size = Size {
    width: 520,
    height: 480,
};
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
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn nodes(tree: &LayoutNode) -> usize {
    match tree {
        LayoutNode::Leaf { .. } => 1,
        LayoutNode::Container { children, .. } => 1 + children.iter().map(nodes).sum::<usize>(),
    }
}

fn preset(label: u32) -> LayoutNode {
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
    LayoutNode::row(vec![LayoutNode::leaf(label, 18)]).with_style(boxed)
}

fn preset_column(padding_right: u32) -> LayoutNode {
    let style = LayoutStyle {
        width: Dimension::Percent(50.0),
        spacing: 8,
        padding: Padding {
            right: padding_right,
            ..Padding::default()
        },
        ..LayoutStyle::default()
    };
    LayoutNode::column((0..4).map(|index| preset(100 + index)).collect()).with_style(style)
}

fn action(width: u32) -> LayoutNode {
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
    LayoutNode::row(vec![LayoutNode::leaf(60, 18)]).with_style(style)
}

fn note_obs() -> LayoutNode {
    let root = LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Percent(100.0),
        padding: Padding::all(18),
        border_width: 2,
        spacing: 14,
        ..LayoutStyle::default()
    };
    let header = LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(32),
        spacing: 12,
        justify: Justify::SpaceBetween,
        align: Align::Center,
        ..LayoutStyle::default()
    };
    let editor = LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(120),
        padding: Padding::all(12),
        border_width: 1,
        stretch: 1.0,
        ..LayoutStyle::default()
    };
    let presets = LayoutStyle {
        width: Dimension::Percent(100.0),
        ..LayoutStyle::default()
    };
    let actions = LayoutStyle {
        width: Dimension::Percent(100.0),
        spacing: 10,
        align: Align::Center,
        ..LayoutStyle::default()
    };
    let afk_row = LayoutStyle {
        width: Dimension::Percent(100.0),
        spacing: 12,
        align: Align::Center,
        ..LayoutStyle::default()
    };
    let afk_toggle = LayoutStyle {
        height: Dimension::Px(28),
        spacing: 10,
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
        stretch: 1.0,
        ..LayoutStyle::default()
    };

    LayoutNode::column(vec![
        LayoutNode::row(vec![LayoutNode::leaf(48, 24), LayoutNode::leaf(80, 18)])
            .with_style(header),
        LayoutNode::leaf(484, 120).with_style(editor),
        LayoutNode::row(vec![preset_column(5), preset_column(0)]).with_style(presets),
        LayoutNode::row(vec![action(18), action(14)]).with_style(actions),
        LayoutNode::row(vec![
            LayoutNode::row(vec![
                LayoutNode::column(vec![LayoutNode::leaf(12, 12)]).with_style(afk_box),
                LayoutNode::leaf(150, 18),
            ])
            .with_style(afk_toggle),
            LayoutNode::leaf(200, 30).with_style(afk_input),
        ])
        .with_style(afk_row),
        LayoutNode::leaf(200, 18),
        LayoutNode::leaf(240, 16),
    ])
    .with_style(root)
}

fn synthetic(rows: usize, per_row: usize) -> LayoutNode {
    let row = LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(24),
        spacing: 4,
        ..LayoutStyle::default()
    };
    let cell = LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(16),
        ..LayoutStyle::default()
    };
    let children = (0..rows)
        .map(|_| {
            LayoutNode::row(
                (0..per_row)
                    .map(|_| LayoutNode::leaf(10, 16).with_style(cell))
                    .collect(),
            )
            .with_style(row)
        })
        .collect();
    LayoutNode::column(children).with_style(LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Percent(100.0),
        spacing: 2,
        ..LayoutStyle::default()
    })
}

fn report_footprint(name: &str, tree: &LayoutNode) {
    let allocations = ALLOCATIONS.load(Ordering::Relaxed);
    let bytes = BYTES.load(Ordering::Relaxed);
    for _ in 0..PASSES {
        black_box(layout(black_box(tree), VIEWPORT).expect("bench tree is valid"));
    }
    let allocations = (ALLOCATIONS.load(Ordering::Relaxed) - allocations) as f64 / PASSES as f64;
    let bytes = (BYTES.load(Ordering::Relaxed) - bytes) as f64 / PASSES as f64;
    println!("{name}: {allocations:.1} allocations, {bytes:.0} requested bytes per layout");
}

fn measure(criterion: &mut Criterion) {
    let note_obs = note_obs();
    let synthetic = synthetic(25, 19);

    report_footprint("note-obs", &note_obs);
    report_footprint("synthetic-500", &synthetic);

    let mut group = criterion.benchmark_group("layout");

    group.throughput(Throughput::Elements(nodes(&note_obs) as u64));
    group.bench_function("note-obs", |bencher| {
        bencher.iter(|| layout(black_box(&note_obs), VIEWPORT).expect("note-obs tree is valid"));
    });

    group.throughput(Throughput::Elements(nodes(&synthetic) as u64));
    group.bench_function("synthetic-500", |bencher| {
        bencher.iter(|| layout(black_box(&synthetic), VIEWPORT).expect("synthetic tree is valid"));
    });
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    measure(&mut criterion);
    criterion.final_summary();
}
