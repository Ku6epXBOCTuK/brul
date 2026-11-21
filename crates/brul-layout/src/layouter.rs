use std::collections::{HashMap, HashSet};

use taffy::{
    AvailableSpace, NodeId, Size as TaffySize, Style as TaffyStyle, TaffyTree, compute_leaf_layout,
};

use crate::taffy::taffy_style;
use crate::{Direction, LayoutBox, LayoutStyle, Offset, ScrollOffset, Size};

pub struct NodeShape<'a, C> {
    pub style: &'a LayoutStyle,
    pub direction: Direction,
    pub offset: Option<Offset>,
    pub scroll: ScrollOffset,
    pub content: C,
    pub children: &'a [u32],
    pub transparent: bool,
    pub layer: bool,
    pub constrained: bool,
}

struct Entry<C> {
    node: Option<NodeId>,
    nodes: Vec<NodeId>,
    style: TaffyStyle,
    layout: LayoutStyle,
    direction: Direction,
    offset: Option<Offset>,
    scroll: ScrollOffset,
    content: C,
    children: Vec<u32>,
    layer: bool,
    constrained: bool,
}

pub struct Layouter<C> {
    tree: TaffyTree<C>,
    entries: HashMap<u32, Entry<C>>,
    live: HashSet<u32>,
    scratch: Vec<NodeId>,
    stack: Vec<u32>,
}

impl<C: PartialEq + Clone> Layouter<C> {
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
            entries: HashMap::new(),
            live: HashSet::new(),
            scratch: Vec::new(),
            stack: Vec::new(),
        }
    }

    pub fn begin(&mut self) {
        self.live.clear();
    }

    pub fn finish(&mut self) {
        let Self {
            entries,
            live,
            tree,
            ..
        } = self;
        entries.retain(|key, entry| {
            if live.contains(key) {
                return true;
            }
            if let Some(node) = entry.node {
                tree.remove(node).expect("layout tree");
            }
            false
        });
    }

    pub fn node(&mut self, key: u32, shape: NodeShape<C>) {
        self.live.insert(key);
        self.child_nodes(shape.children);
        let placed = !shape.transparent;

        let Self {
            tree,
            entries,
            scratch,
            ..
        } = self;

        if entries
            .get(&key)
            .is_some_and(|entry| entry.node.is_some() != placed)
        {
            let entry = entries.remove(&key).expect("layout entry");
            if let Some(node) = entry.node {
                tree.remove(node).expect("layout tree");
            }
        }

        if let Some(entry) = entries.get_mut(&key) {
            entry.scroll = shape.scroll;
            entry.layer = shape.layer;
            entry.constrained = shape.constrained;
            if entry.children.as_slice() != shape.children {
                entry.children.clear();
                entry.children.extend_from_slice(shape.children);
            }
            update_nodes(entry, tree, scratch);

            let Some(node) = entry.node else {
                entry.layout = shape.style.clone();
                entry.direction = shape.direction;
                entry.offset = shape.offset;
                entry.constrained = shape.constrained;
                entry.content = shape.content;
                return;
            };
            update_style(entry, tree, node, &shape);
            if entry.content != shape.content {
                entry.content = shape.content.clone();
                tree.set_node_context(node, Some(shape.content))
                    .expect("layout tree");
            }
            return;
        }

        let taffy = taffy_style(
            shape.style,
            shape.direction,
            shape.offset,
            shape.constrained,
        );
        let content = shape.content;
        entries.insert(
            key,
            Entry {
                node: placed.then(|| {
                    let node = tree
                        .new_leaf_with_context(taffy.clone(), content.clone())
                        .expect("layout tree");
                    if !scratch.is_empty() {
                        tree.set_children(node, scratch).expect("layout tree");
                    }
                    node
                }),
                nodes: scratch.to_vec(),
                style: taffy,
                layout: shape.style.clone(),
                direction: shape.direction,
                offset: shape.offset,
                scroll: shape.scroll,
                content,
                children: shape.children.to_vec(),
                layer: shape.layer,
                constrained: shape.constrained,
            },
        );
    }

    fn child_nodes(&mut self, children: &[u32]) {
        let Self {
            entries,
            scratch,
            stack,
            ..
        } = self;
        scratch.clear();
        stack.clear();
        stack.extend(children.iter().copied().rev());
        while let Some(key) = stack.pop() {
            let entry = entries.get(&key).expect("layout child");
            match entry.node {
                Some(node) => scratch.push(node),
                None => stack.extend(entry.children.iter().copied().rev()),
            }
        }
    }
}

fn update_nodes<C>(entry: &mut Entry<C>, tree: &mut TaffyTree<C>, nodes: &[NodeId]) {
    if entry.nodes.as_slice() != nodes {
        entry.nodes.clear();
        entry.nodes.extend_from_slice(nodes);
        if let Some(node) = entry.node {
            tree.set_children(node, nodes).expect("layout tree");
        }
    }
}

fn update_style<C>(
    entry: &mut Entry<C>,
    tree: &mut TaffyTree<C>,
    node: NodeId,
    shape: &NodeShape<'_, C>,
) {
    if entry.layout != shape.style.clone()
        || entry.direction != shape.direction
        || entry.offset != shape.offset
        || entry.constrained != shape.constrained
    {
        let taffy = taffy_style(
            shape.style,
            shape.direction,
            shape.offset,
            shape.constrained,
        );
        entry.style = taffy.clone();
        tree.set_style(node, taffy).expect("layout tree");
        entry.layout = shape.style.clone();
        entry.direction = shape.direction;
        entry.offset = shape.offset;
        entry.constrained = shape.constrained;
    }
}

impl<C> Layouter<C> {
    pub fn touch(&mut self, key: u32) {
        let Some(entry) = self.entries.get(&key) else {
            return;
        };
        match entry.node {
            Some(node) => self.tree.mark_dirty(node).expect("layout tree"),
            None => {
                for child in entry.children.clone() {
                    self.touch(child);
                }
            }
        }
    }

    pub fn compute(
        &mut self,
        root: u32,
        viewport: Size,
        mut measure: impl FnMut(&mut C, Option<u32>) -> Size,
    ) {
        let node = self
            .entries
            .get(&root)
            .expect("layout root")
            .node
            .expect("layout root");

        // NOTE: taffy only rejects ids it did not create, and every id here comes from this tree
        self.tree
            .compute_layout_with_measure(
                node,
                TaffySize {
                    width: AvailableSpace::Definite(viewport.width as f32),
                    height: AvailableSpace::Definite(viewport.height as f32),
                },
                |inputs, _node, context, style| {
                    compute_leaf_layout(
                        inputs,
                        style,
                        |_, _| 0.0,
                        |_, available| {
                            let width = available
                                .width
                                .into_option()
                                .map(|width| width.max(0.0) as u32);
                            let size = context
                                .map(|content| measure(content, width))
                                .unwrap_or_default();
                            TaffySize {
                                width: size.width as f32,
                                height: size.height as f32,
                            }
                        },
                    )
                },
            )
            .expect("layout tree");
    }

    pub fn collect_into(&self, root: u32, boxes: &mut [LayoutBox]) {
        let origin = Placement {
            frame: (0, 0),
            origin: (0, 0),
            direction: Direction::Row,
            scroll: 0,
        };
        self.collect_entry(root, 0, 0, origin, boxes);
    }

    fn collect_entry(
        &self,
        key: u32,
        x: i32,
        y: i32,
        placement: Placement,
        boxes: &mut [LayoutBox],
    ) {
        let entry = self.entries.get(&key).expect("layout entry");
        let own = match entry.node {
            Some(node) => {
                let placed = self.tree.layout(node).expect("layout tree");
                boxes[key as usize] = LayoutBox {
                    x,
                    y,
                    width: placed.size.width.max(0.0) as u32,
                    height: placed.size.height.max(0.0) as u32,
                    children: Vec::new(),
                };
                Placement {
                    frame: (x, y),
                    origin: (
                        clamp_i32(
                            i64::from(x)
                                + i64::from(entry.layout.border_width.left)
                                + i64::from(entry.layout.padding.left),
                        ),
                        clamp_i32(
                            i64::from(y)
                                + i64::from(entry.layout.border_width.top)
                                + i64::from(entry.layout.padding.top),
                        ),
                    ),
                    direction: entry.direction,
                    scroll: entry.scroll.pixels(),
                }
            }
            None => placement,
        };

        let mut bounds: Option<(i32, i32, i32, i32)> = None;
        for child_key in &entry.children {
            let child = self.entries.get(child_key).expect("layout child");
            let (child_x, child_y) = if child.layer {
                (0, 0)
            } else {
                match child.node {
                    Some(node) => {
                        let placed = self.tree.layout(node).expect("layout tree");
                        let location = (placed.location.x as i32, placed.location.y as i32);
                        child_position(
                            own.direction,
                            own.scroll,
                            own.frame,
                            own.origin,
                            child.offset,
                            location,
                        )
                    }
                    None => (own.frame.0, own.frame.1),
                }
            };
            self.collect_entry(*child_key, child_x, child_y, own, boxes);
            if entry.node.is_none() && !child.layer {
                let placed = &boxes[*child_key as usize];
                let right = placed.x + placed.width as i32;
                let bottom = placed.y + placed.height as i32;
                bounds = Some(match bounds {
                    Some((left, top, right_old, bottom_old)) => (
                        left.min(placed.x),
                        top.min(placed.y),
                        right_old.max(right),
                        bottom_old.max(bottom),
                    ),
                    None => (placed.x, placed.y, right, bottom),
                });
            }
        }

        if entry.node.is_none() {
            let (left, top, right, bottom) = bounds.unwrap_or((x, y, x, y));
            boxes[key as usize] = LayoutBox {
                x: left,
                y: top,
                width: (right - left).max(0) as u32,
                height: (bottom - top).max(0) as u32,
                children: Vec::new(),
            };
        }
    }
}

#[derive(Clone, Copy)]
struct Placement {
    frame: (i32, i32),
    origin: (i32, i32),
    direction: Direction,
    scroll: i32,
}

impl<C: PartialEq + Clone> Default for Layouter<C> {
    fn default() -> Self {
        Self::new()
    }
}

fn child_position(
    direction: Direction,
    scroll: i32,
    frame: (i32, i32),
    origin: (i32, i32),
    offset: Option<Offset>,
    location: (i32, i32),
) -> (i32, i32) {
    match offset {
        Some(offset) => (
            clamp_i32(i64::from(origin.0) + i64::from(offset.x)),
            clamp_i32(i64::from(origin.1) + i64::from(offset.y)),
        ),
        None => {
            let mut x = clamp_i32(i64::from(frame.0) + i64::from(location.0));
            let mut y = clamp_i32(i64::from(frame.1) + i64::from(location.1));
            match direction {
                Direction::Row => x = clamp_i32(i64::from(x) - i64::from(scroll)),
                Direction::Column => y = clamp_i32(i64::from(y) - i64::from(scroll)),
            }
            (x, y)
        }
    }
}

fn clamp_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[cfg(test)]
mod tests;
