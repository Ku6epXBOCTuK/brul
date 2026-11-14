use super::*;
use crate::image::Image;
use crate::layout::Size;
use crate::rect::ClipRect;
use crate::text::{CaretPosition, SelectionRect, TextLayout};

#[derive(Clone)]
pub struct WidgetSnapshot {
    pub size: Size,
    pub nodes: Vec<WidgetSnapshotNode>,
}

impl WidgetSnapshot {
    pub fn node(&self, id: WidgetId) -> Option<&WidgetSnapshotNode> {
        self.nodes.iter().find(|node| node.id == id)
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
    pub text: Option<TextLayout>,
    pub preedit: Option<TextLayout>,
    pub caret: Option<CaretPosition>,
    pub selection: Vec<SelectionRect>,
    pub scroll: i32,
    pub image: Option<Image>,
    pub clips: Vec<ClipRect>,
}
