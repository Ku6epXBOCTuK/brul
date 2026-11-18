use super::*;
use crate::image::Image;
use crate::layout::Size;
use crate::rect::ClipRect;
use crate::text::{CaretPosition, SelectionRect, TextLayout};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone)]
pub struct WidgetSnapshot {
    pub size: Size,
    pub nodes: Vec<WidgetSnapshotNode>,
    pub index: HashMap<WidgetId, usize>,
}

impl WidgetSnapshot {
    pub fn node(&self, id: WidgetId) -> Option<&WidgetSnapshotNode> {
        self.index.get(&id).map(|&at| &self.nodes[at])
    }
}

#[derive(Clone)]
pub struct WidgetSnapshotNode {
    pub id: WidgetId,
    pub kind: WidgetKind,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub border_width: u32,
    pub content_x: i32,
    pub content_y: i32,
    pub content_width: u32,
    pub content_height: u32,
    pub state: WidgetState,
    pub style: WidgetStyle,
    pub text: Option<Rc<TextLayout>>,
    pub preedit: Option<Rc<TextLayout>>,
    pub caret: Option<CaretPosition>,
    pub selection: Vec<SelectionRect>,
    pub scroll: i32,
    pub image: Option<Image>,
    pub clips: Vec<ClipRect>,
}

impl WidgetSnapshotNode {
    pub(crate) fn same_child_clip(&self, fresh: &Self) -> bool {
        self.x == fresh.x
            && self.y == fresh.y
            && self.width == fresh.width
            && self.height == fresh.height
            && self.style.relayout() == fresh.style.relayout()
    }
}
