use std::rc::Rc;

use crate::{AtlasSlot, Color, Rect, Shadow, Texture, WidgetStyle};

#[derive(Clone, Copy, Debug, Default)]
pub struct CanvasSpec {
    pub animated: bool,
}

impl CanvasSpec {
    pub const fn new() -> Self {
        Self { animated: false }
    }

    pub const fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }
}

pub struct PaintCtx<'a> {
    pub rect: Rect,
    pub style: &'a WidgetStyle,
}

#[derive(Clone)]
pub struct PaintShape {
    pub rect: Rect,
    pub shadow: Option<Shadow>,
    pub draw_fill: bool,
}

#[derive(Clone, Copy)]
pub struct PaintGlyph {
    pub x: i32,
    pub y: i32,
    pub slot: AtlasSlot,
    pub tint: Option<Color>,
}

#[derive(Clone, Copy)]
pub struct PaintRect {
    pub rect: [i32; 4],
    pub color: Color,
}

#[derive(Clone)]
pub struct PaintImage {
    pub texture: Rc<Texture>,
    pub tint: Option<Color>,
    pub rect: [i32; 4],
}

#[derive(Clone)]
pub enum PaintCommand {
    Shape(PaintShape),
    Glyph(PaintGlyph),
    Rect(PaintRect),
    Image(PaintImage),
}

pub trait CanvasPainter {
    fn paint(&self, ctx: &PaintCtx<'_>) -> Vec<PaintCommand>;
}
