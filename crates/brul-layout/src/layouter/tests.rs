use super::*;
use crate::*;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone)]
struct Probe {
    id: u32,
    calls: Rc<Cell<usize>>,
}

impl PartialEq for Probe {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

struct Counted {
    id: u32,
    clones: Rc<Cell<usize>>,
}

impl PartialEq for Counted {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Clone for Counted {
    fn clone(&self) -> Self {
        self.clones.set(self.clones.get() + 1);
        Self {
            id: self.id,
            clones: Rc::clone(&self.clones),
        }
    }
}

#[test]
fn a_repeat_pass_reuses_layout_buffers() {
    let clones = Rc::new(Cell::new(0));
    let style = LayoutStyle::default();
    let empty: &[u32] = &[];
    let single: &[u32] = &[1];
    let mut layouter = Layouter::<Counted>::new();

    let pass = |layouter: &mut Layouter<Counted>| {
        layouter.begin();
        for (key, children) in [(1, empty), (0, single)] {
            layouter.node(
                key,
                NodeShape {
                    style: &style,
                    direction: Direction::Row,
                    offset: None,
                    scroll: ScrollOffset::ZERO,
                    content: Counted {
                        id: key,
                        clones: Rc::clone(&clones),
                    },
                    children,
                    transparent: false,
                    layer: false,
                    constrained: false,
                },
            );
        }
        layouter.finish();
    };

    pass(&mut layouter);
    let baseline = clones.get();
    let (children, nodes) = {
        let root = layouter.entries.get(&0).expect("layout entry");
        (root.children.as_ptr(), root.nodes.as_ptr())
    };

    pass(&mut layouter);
    let root = layouter.entries.get(&0).expect("layout entry");
    assert_eq!(clones.get(), baseline);
    assert_eq!(root.children.as_ptr(), children);
    assert_eq!(root.nodes.as_ptr(), nodes);
}

#[test]
fn a_clean_tree_is_not_measured_again() {
    let calls = Rc::new(Cell::new(0));
    let root_style = LayoutStyle {
        width: Dimension::Px(100),
        height: Dimension::Px(50),
        ..LayoutStyle::default()
    };
    let child_style = LayoutStyle {
        width: Dimension::Px(20),
        height: Dimension::Px(10),
        ..LayoutStyle::default()
    };
    let viewport = Size {
        width: 100,
        height: 50,
    };
    let mut layouter = Layouter::<Probe>::new();

    let build = |layouter: &mut Layouter<Probe>| {
        layouter.begin();
        layouter.node(
            1,
            NodeShape {
                style: &child_style,
                direction: Direction::Row,
                offset: None,
                scroll: ScrollOffset::ZERO,
                content: Probe {
                    id: 1,
                    calls: Rc::clone(&calls),
                },
                children: &[],
                transparent: false,
                layer: false,
                constrained: false,
            },
        );
        layouter.node(
            0,
            NodeShape {
                style: &root_style,
                direction: Direction::Column,
                offset: None,
                scroll: ScrollOffset::ZERO,
                content: Probe {
                    id: 0,
                    calls: Rc::clone(&calls),
                },
                children: &[1],
                transparent: false,
                layer: false,
                constrained: false,
            },
        );
        layouter.finish();
        layouter.compute(0, viewport, |content, _width| {
            content.calls.set(content.calls.get() + 1);
            Size {
                width: 20,
                height: 10,
            }
        });
    };

    build(&mut layouter);
    let measured = calls.get();
    assert!(measured >= 1);

    let mut first = vec![LayoutBox::default(); 2];
    layouter.collect_into(0, &mut first);
    assert_eq!(first[1].width, 20);
    assert_eq!(first[0].width, 100);

    build(&mut layouter);
    assert_eq!(calls.get(), measured);

    let mut second = vec![LayoutBox::default(); 2];
    layouter.collect_into(0, &mut second);
    assert_eq!(second, first);
}
