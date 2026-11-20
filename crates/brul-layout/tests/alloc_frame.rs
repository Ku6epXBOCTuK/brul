use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use brul_layout::{
    Dimension, Direction, LayoutBox, LayoutNode, LayoutStyle, Layouter, NodeShape, Offset,
    ScrollOffset, Size,
};

struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn allocations() -> usize {
    ALLOCATIONS.load(Ordering::Relaxed)
}

const VIEWPORT: Size = Size {
    width: 520,
    height: 480,
};

struct Shape {
    style: LayoutStyle,
    direction: Direction,
    offset: Option<Offset>,
    scroll: ScrollOffset,
    content: Size,
    children: Vec<u32>,
}

fn flatten(node: &LayoutNode, shapes: &mut Vec<Shape>, key: u32) -> u32 {
    match node {
        LayoutNode::Leaf {
            style,
            size,
            offset,
        } => {
            shapes.push(Shape {
                style: style.clone(),
                direction: Direction::Row,
                offset: *offset,
                scroll: ScrollOffset::ZERO,
                content: *size,
                children: Vec::new(),
            });
            key + 1
        }
        LayoutNode::Container {
            style,
            direction,
            children,
            offset,
            scroll_offset,
        } => {
            let mut next = key;
            let mut keys = Vec::with_capacity(children.len());
            for child in children {
                keys.push(next);
                next = flatten(child, shapes, next);
            }
            shapes.push(Shape {
                style: style.clone(),
                direction: *direction,
                offset: *offset,
                scroll: *scroll_offset,
                content: Size::default(),
                children: keys,
            });
            next
        }
    }
}

fn tree() -> LayoutNode {
    let cell = LayoutStyle {
        width: Dimension::Px(20),
        height: Dimension::Px(20),
        ..LayoutStyle::default()
    };
    let header = LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(32),
        ..LayoutStyle::default()
    };
    LayoutNode::row(vec![
        LayoutNode::row(vec![LayoutNode::leaf(48, 24), LayoutNode::leaf(80, 18)])
            .with_style(header),
        LayoutNode::leaf(484, 120),
        LayoutNode::row(vec![
            LayoutNode::column(vec![LayoutNode::leaf(12, 12)]).with_style(cell),
            LayoutNode::leaf(150, 18),
        ]),
    ])
    .with_style(LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Percent(100.0),
        ..LayoutStyle::default()
    })
}

#[test]
fn a_repeated_layout_frame_does_not_allocate() {
    let root = tree();
    let mut shapes = Vec::new();
    flatten(&root, &mut shapes, 1);
    let root_key = shapes.len() as u32;

    let mut layouter = Layouter::<Size>::new();
    let mut boxes = vec![LayoutBox::default(); shapes.len() + 1];

    let pass = |layouter: &mut Layouter<Size>, boxes: &mut [LayoutBox]| {
        layouter.begin();
        for (index, shape) in shapes.iter().enumerate() {
            layouter.node(
                index as u32 + 1,
                NodeShape {
                    style: &shape.style,
                    direction: shape.direction,
                    offset: shape.offset,
                    scroll: shape.scroll,
                    content: shape.content,
                    children: &shape.children,
                    transparent: false,
                    layer: false,
                    constrained: false,
                },
            );
        }
        layouter.finish();
        layouter.compute(root_key, VIEWPORT, |content, _width| *content);
        layouter.collect_into(root_key, boxes);
    };

    pass(&mut layouter, &mut boxes);
    pass(&mut layouter, &mut boxes);

    let before = allocations();
    for _ in 0..100 {
        pass(&mut layouter, &mut boxes);
    }
    assert_eq!(allocations() - before, 0, "repeated layout frame");
}
