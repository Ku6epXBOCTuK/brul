use super::*;
use crate::layout::{self, Available, LayoutBox, LayoutNode, Layouter, NodeShape};
use crate::text::TextEngine;
use std::collections::{HashMap, HashSet};

pub(super) struct Pass<'a> {
    pub(super) tree: &'a [Node],
    pub(super) layouter: &'a mut Layouter<Measure>,
    pub(super) cached: Option<&'a [LayoutBox]>,
    pub(super) blocked: Vec<bool>,
    pub(super) order: Vec<Slot>,
    pub(super) frozen: Vec<(Slot, LayoutBox)>,
}

#[derive(Clone, PartialEq)]
pub(super) enum Measure {
    Widget(Slot),
    Fixed(Size),
}

pub(super) fn dirty_subtrees(tree: &[Node], root: Slot, stale: &HashSet<Slot>) -> Vec<bool> {
    fn walk(tree: &[Node], slot: Slot, stale: &HashSet<Slot>, blocked: &mut [bool]) -> bool {
        let node = &tree[slot.index()];
        let mut dirty = stale.contains(&slot);
        for &child in &node.children {
            dirty |= walk(tree, child, stale, blocked);
        }
        blocked[slot.index()] = dirty;
        dirty
    }
    let mut blocked = vec![false; tree.len()];
    walk(tree, root, stale, &mut blocked);
    blocked
}

pub(super) fn move_children(tree: &[Node], slot: Slot, dx: i32, dy: i32, boxes: &mut [LayoutBox]) {
    for &child in &tree[slot.index()].children {
        let placed = &mut boxes[child.index()];
        placed.x = placed.x.saturating_add(dx);
        placed.y = placed.y.saturating_add(dy);
        move_children(tree, child, dx, dy, boxes);
    }
}

impl Pass<'_> {
    pub(super) fn build(
        &mut self,
        slot: Slot,
        available: Available,
    ) -> Result<(LayoutNode, u32), WidgetError> {
        let node = &self.tree[slot.index()];
        let layout_style = node.layout_style();
        let prepared = layout::prepare(&layout_style, available)?;
        self.order.push(slot);
        let key = node.id.index();
        if let (Some(cached), false) = (self.cached, self.blocked[slot.index()])
            && let Some(previous) = cached.get(slot.index())
        {
            let size = Size {
                width: previous
                    .width
                    .saturating_sub(layout::inset_x(&layout_style)),
                height: previous
                    .height
                    .saturating_sub(layout::inset_y(&layout_style)),
            };
            self.frozen.push((slot, previous.clone()));
            self.layouter.node(
                key,
                NodeShape {
                    style: &layout_style,
                    direction: Direction::Row,
                    offset: node.offset,
                    content: Measure::Fixed(size),
                    prepared,
                    children: &[],
                },
            );
            let layout_node = LayoutNode::leaf(size.width, size.height).with_style(layout_style);
            return Ok((
                match node.offset {
                    Some(offset) => layout_node.with_offset(offset.x, offset.y),
                    None => layout_node,
                },
                key,
            ));
        }

        let mut keys = Vec::new();
        let (direction, layout_node) = match &node.data {
            Content::View { direction } => {
                let mut children = Vec::with_capacity(node.children.len());
                keys.reserve(node.children.len());
                for &child in &node.children {
                    let (child_node, child_key) = self.build(child, prepared.available)?;
                    children.push(child_node);
                    keys.push(child_key);
                }
                let children = match direction {
                    Direction::Row => LayoutNode::row(children),
                    Direction::Column => {
                        LayoutNode::column(children).with_scroll(node.scroll_offset)
                    }
                };
                (*direction, children)
            }
            Content::Text { .. } | Content::Input { .. } | Content::Image(_) => {
                (Direction::Row, LayoutNode::leaf(0, 0))
            }
        };

        self.layouter.node(
            key,
            NodeShape {
                style: &layout_style,
                direction,
                offset: node.offset,
                content: Measure::Widget(slot),
                prepared,
                children: &keys,
            },
        );
        if self.blocked[slot.index()] {
            self.layouter.touch(key);
        }

        let layout_node = layout_node.with_style(layout_style);
        let layout_node = match node.offset {
            Some(offset) => layout_node.with_offset(offset.x, offset.y),
            None => layout_node,
        };
        Ok((layout_node, key))
    }
}

pub(super) fn measure_text(
    node: &Node,
    engine: &mut TextEngine,
    texts: &mut HashMap<WidgetId, ShapedText>,
    width: Option<u32>,
    scale: f32,
) -> Result<Size, WidgetError> {
    let (content, styles, width, preedit) = match &node.data {
        Content::Text { content, styles } => (content.as_str(), styles, width, ""),
        Content::Input {
            value,
            styles,
            multiline,
            preedit,
            ..
        } => (
            value.as_str(),
            styles,
            if *multiline { width } else { None },
            preedit.as_str(),
        ),
        Content::Image(image) => {
            return Ok(Size {
                width: image.width(),
                height: image.height(),
            });
        }
        _ => return Ok(Size::default()),
    };

    let mut styles = styles.clone();
    if let Some(class) = &node.class {
        styles.apply_layers(&class.text);
    }
    let max_width = width.map(|width| width as f32 / scale);
    let cached = texts.get(&node.id).filter(|cached| {
        cached.width == width
            && cached.scale == scale
            && cached.content == content
            && cached.preedit_text == preedit
    });
    if cached.is_none() {
        let mut variants =
            TextVariants::new(engine.layout(content, styles.base(), max_width, scale)?);
        for state in StyleState::ALL {
            let style = styles.resolve(state.flags());
            if style == *styles.base() {
                continue;
            }
            variants = variants.with(state, engine.layout(content, &style, max_width, scale)?);
        }
        let preedit_text = preedit.to_owned();
        let preedit = if preedit_text.is_empty() {
            None
        } else {
            Some(engine.layout(&preedit_text, styles.base(), None, scale)?)
        };
        texts.insert(
            node.id,
            ShapedText {
                content: content.to_owned(),
                width,
                scale,
                variants,
                preedit,
                preedit_text,
            },
        );
    }
    let size = texts
        .get(&node.id)
        .expect("shaped text cached")
        .variants
        .base
        .size;
    Ok(Size {
        width: size.width,
        height: size.height,
    })
}

pub(super) fn max_scroll_offset(nodes: &[Node], slot: Slot, boxes: &[LayoutBox]) -> Option<i32> {
    let node = &nodes[slot.index()];
    let viewport = &boxes[slot.index()];
    let bottom = node
        .children
        .iter()
        .map(|&child| {
            let placed = &boxes[child.index()];
            i64::from(placed.y) + i64::from(placed.height)
        })
        .max()?;
    let overflow = bottom + i64::from(node.scroll_offset.pixels())
        - i64::from(viewport.y)
        - i64::from(viewport.height);
    i32::try_from(overflow.max(0)).ok()
}

pub(super) fn flatten_boxes(
    boxed: &LayoutBox,
    order: &[Slot],
    cursor: &mut usize,
    boxes: &mut [LayoutBox],
) {
    let slot = order[*cursor];
    *cursor += 1;
    boxes[slot.index()] = LayoutBox {
        x: boxed.x,
        y: boxed.y,
        width: boxed.width,
        height: boxed.height,
        children: Vec::new(),
    };
    for child in &boxed.children {
        flatten_boxes(child, order, cursor, boxes);
    }
}
