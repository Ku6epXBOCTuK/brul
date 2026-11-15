use super::*;
use crate::layout::{self, Layouter, NodeShape};
use crate::text::TextEngine;
use std::collections::{HashMap, HashSet};

pub(super) struct Pass<'a> {
    pub(super) tree: &'a [Node],
    pub(super) layouter: &'a mut Layouter<Slot>,
    pub(super) stale: &'a HashSet<Slot>,
}

impl Pass<'_> {
    pub(super) fn build(&mut self, slot: Slot) -> Result<u32, WidgetError> {
        let node = &self.tree[slot.index()];
        let layout_style = node.layout_style();
        layout::validate(&layout_style)?;
        let key = slot.index() as u32;

        let mut keys = Vec::new();
        let (direction, scroll) = match &node.data {
            Content::View { direction } => {
                keys.reserve(node.children.len());
                for &child in &node.children {
                    keys.push(self.build(child)?);
                }
                let scroll = match direction {
                    Direction::Column => node.scroll_offset,
                    Direction::Row => ScrollOffset::ZERO,
                };
                (*direction, scroll)
            }
            Content::Text { .. } | Content::Input { .. } | Content::Image(_) => {
                (Direction::Row, ScrollOffset::ZERO)
            }
        };

        self.layouter.node(
            key,
            NodeShape {
                style: &layout_style,
                direction,
                offset: node.offset,
                scroll,
                content: slot,
                children: &keys,
            },
        );
        if self.stale.contains(&slot) {
            self.layouter.touch(key);
        }
        Ok(key)
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
