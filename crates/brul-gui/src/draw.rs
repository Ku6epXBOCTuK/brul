use std::rc::Rc;

use crate::{
    AtlasFormat, AtlasSlot, CanvasPainter, CaretPosition, Color, Dirty, Gradient, Image, Paint,
    PaintCommand, PaintCtx, Rect, Shadow, TextLayout, Texture, WidgetId, WidgetKind,
    WidgetSnapshot, WidgetSnapshotNode, WidgetStyle,
};

#[derive(Clone, Debug)]
pub struct ShapeCommand {
    pub node: usize,
    pub rect: Rect,
    pub shadow: Option<Shadow>,
    pub draw_fill: bool,
    pub opacity: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct GlyphCommand {
    pub node: usize,
    pub x: i32,
    pub y: i32,
    pub slot: AtlasSlot,
    pub tint: Option<Color>,
    pub opacity: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct RectCommand {
    pub node: usize,
    pub rect: [i32; 4],
    pub color: Color,
    pub opacity: f32,
}

#[derive(Clone)]
pub struct ImageCommand {
    pub node: usize,
    pub texture: Rc<Texture>,
    pub tint: Option<Color>,
    pub rect: [i32; 4],
    pub opacity: f32,
}

#[derive(Clone)]
pub enum DrawCommand {
    Shape(ShapeCommand),
    Glyph(GlyphCommand),
    Rect(RectCommand),
    Image(ImageCommand),
}

pub fn draw_commands(snapshot: &WidgetSnapshot, dirty: &Dirty, out: &mut Vec<DrawCommand>) {
    out.clear();
    for (index, node) in snapshot.nodes.iter().enumerate() {
        if !node_is_dirty(dirty, node.id) || node.width == 0 || node.height == 0 {
            continue;
        }
        let Some(opacity) = opacity(node.style.opacity) else {
            continue;
        };
        if node.kind != WidgetKind::PlatformView {
            push_shape(index, node, opacity, out);
        }
        match node.kind {
            WidgetKind::Text => push_text(index, node, opacity, out),
            WidgetKind::Image => push_image(index, node, opacity, out),
            WidgetKind::Input => {
                push_selection(index, node, opacity, out);
                push_text(index, node, opacity, out);
                if let (Some(preedit), Some(caret)) = (&node.preedit, node.caret) {
                    let shift = preedit_shift(caret.x, preedit.size.width);
                    push_glyphs(index, node, preedit, shift, opacity, out);
                    push_underline(index, node, preedit, caret, shift, opacity, out);
                }
                push_caret(index, node, opacity, out);
            }
            WidgetKind::Canvas => {
                if let Some(painter) = &node.canvas {
                    push_canvas(index, node, painter.as_ref(), opacity, out);
                }
            }
            WidgetKind::View | WidgetKind::Structure | WidgetKind::PlatformView => {}
        }
    }
}

fn push_shape(index: usize, node: &WidgetSnapshotNode, opacity: f32, out: &mut Vec<DrawCommand>) {
    let shadow = (node.style.shadow.color.a > 0).then_some(node.style.shadow);
    let draw_fill = match node.style.background {
        Paint::Color(color) => color.a > 0,
        Paint::Gradient(Gradient::Linear { angle, from, .. }) => angle != 0.0 && from.a > 0,
    };
    let draw_border = node.border.visible();
    if shadow.is_none() && !draw_fill && !draw_border {
        return;
    }
    let rect = Rect::new(node.x, node.y, node.width, node.height)
        .with_radius(node.style.border_radius)
        .with_fill(node.style.background)
        .with_border(node.border);
    out.push(DrawCommand::Shape(ShapeCommand {
        node: index,
        rect,
        shadow,
        draw_fill,
        opacity,
    }));
}

fn push_text(index: usize, node: &WidgetSnapshotNode, opacity: f32, out: &mut Vec<DrawCommand>) {
    let Some(text) = &node.text else {
        return;
    };
    push_glyphs(index, node, text, 0, opacity, out);
}

fn push_glyphs(
    index: usize,
    node: &WidgetSnapshotNode,
    text: &TextLayout,
    shift: i32,
    opacity: f32,
    out: &mut Vec<DrawCommand>,
) {
    let offset = vertical_offset(node, text.size.height);
    for glyph in &text.glyphs {
        let slot = glyph.slot;
        if slot.width == 0 || slot.height == 0 {
            continue;
        }
        out.push(DrawCommand::Glyph(GlyphCommand {
            node: index,
            x: node.content_x.saturating_add(shift).saturating_add(glyph.x),
            y: node
                .content_y
                .saturating_add(glyph.y)
                .saturating_add(offset),
            slot,
            tint: glyph_tint(slot.format, node.style.text),
            opacity,
        }));
    }
}

fn push_selection(
    index: usize,
    node: &WidgetSnapshotNode,
    opacity: f32,
    out: &mut Vec<DrawCommand>,
) {
    let height = node.text.as_ref().map_or(0, |text| text.size.height);
    let offset = vertical_offset(node, height);
    let color = node.style.text.with_alpha(60);
    for rect in &node.selection {
        out.push(DrawCommand::Rect(RectCommand {
            node: index,
            rect: [
                node.content_x.saturating_add(rect.x),
                node.content_y.saturating_add(rect.y).saturating_add(offset),
                rect.width,
                rect.height,
            ],
            color,
            opacity,
        }));
    }
}

fn push_underline(
    index: usize,
    node: &WidgetSnapshotNode,
    preedit: &TextLayout,
    caret: CaretPosition,
    shift: i32,
    opacity: f32,
    out: &mut Vec<DrawCommand>,
) {
    let top = node
        .content_y
        .saturating_add(caret.y)
        .saturating_add(caret_offset(node, caret))
        .saturating_add(preedit.size.height as i32)
        .saturating_sub(1);
    out.push(DrawCommand::Rect(RectCommand {
        node: index,
        rect: [
            node.content_x.saturating_add(shift),
            top,
            preedit.size.width as i32,
            1,
        ],
        color: node.style.text,
        opacity,
    }));
}

fn push_caret(index: usize, node: &WidgetSnapshotNode, opacity: f32, out: &mut Vec<DrawCommand>) {
    let Some(caret) = node.caret else {
        return;
    };
    out.push(DrawCommand::Rect(RectCommand {
        node: index,
        rect: [
            node.content_x.saturating_add(caret.x),
            node.content_y
                .saturating_add(caret.y)
                .saturating_add(caret_offset(node, caret)),
            1,
            caret.height,
        ],
        color: node.style.text,
        opacity,
    }));
}

fn push_canvas(
    index: usize,
    node: &WidgetSnapshotNode,
    painter: &dyn CanvasPainter,
    opacity: f32,
    out: &mut Vec<DrawCommand>,
) {
    let ctx = PaintCtx {
        rect: Rect::new(
            node.content_x,
            node.content_y,
            node.content_width,
            node.content_height,
        ),
        style: &node.style,
    };
    for command in painter.paint(&ctx) {
        match command {
            PaintCommand::Shape(shape) => out.push(DrawCommand::Shape(ShapeCommand {
                node: index,
                rect: shape.rect,
                shadow: shape.shadow,
                draw_fill: shape.draw_fill,
                opacity,
            })),
            PaintCommand::Glyph(glyph) => out.push(DrawCommand::Glyph(GlyphCommand {
                node: index,
                x: glyph.x,
                y: glyph.y,
                slot: glyph.slot,
                tint: glyph.tint,
                opacity,
            })),
            PaintCommand::Rect(rect) => out.push(DrawCommand::Rect(RectCommand {
                node: index,
                rect: rect.rect,
                color: rect.color,
                opacity,
            })),
            PaintCommand::Image(image) => out.push(DrawCommand::Image(ImageCommand {
                node: index,
                texture: image.texture,
                tint: image.tint,
                rect: image.rect,
                opacity,
            })),
        }
    }
}

fn push_image(index: usize, node: &WidgetSnapshotNode, opacity: f32, out: &mut Vec<DrawCommand>) {
    let Some(image) = &node.image else {
        return;
    };
    let Some((texture, tint)) =
        resolved_image(image, node.content_width, node.content_height, &node.style)
    else {
        return;
    };
    out.push(DrawCommand::Image(ImageCommand {
        node: index,
        texture,
        tint,
        rect: [
            node.content_x,
            node.content_y,
            node.content_width as i32,
            node.content_height as i32,
        ],
        opacity,
    }));
}

fn resolved_image(
    image: &Image,
    width: u32,
    height: u32,
    style: &WidgetStyle,
) -> Option<(Rc<Texture>, Option<Color>)> {
    if width == 0 || height == 0 {
        return None;
    }
    match image {
        Image::Raster(texture) => {
            let tint = match (texture.tint(), style.tint) {
                (Some(texture), Some(style)) => Some(texture.multiply(style)),
                (Some(texture), None) => Some(texture),
                (None, Some(style)) => Some(style),
                (None, None) => None,
            };
            Some((Rc::clone(texture), tint))
        }
        Image::Vector(vector) => {
            let current = style.tint.unwrap_or(style.text);
            vector
                .raster(width, height, current)
                .map(|texture| (texture, None))
        }
    }
}

fn vertical_offset(node: &WidgetSnapshotNode, height: u32) -> i32 {
    node.style
        .vertical_align
        .offset(node.content_height, height) as i32
        - node.scroll
}

fn caret_offset(node: &WidgetSnapshotNode, caret: CaretPosition) -> i32 {
    let height = node
        .text
        .as_ref()
        .map_or(caret.y.max(0), |text| text.size.height as i32)
        .max(0) as u32;
    vertical_offset(node, height)
}

fn preedit_shift(caret_x: i32, width: u32) -> i32 {
    caret_x.saturating_sub(width as i32)
}

fn glyph_tint(format: AtlasFormat, color: Color) -> Option<Color> {
    match format {
        AtlasFormat::Mask => Some(color),
        AtlasFormat::Color => None,
    }
}

fn node_is_dirty(dirty: &Dirty, id: WidgetId) -> bool {
    dirty.full || dirty.ids.contains(&id)
}

fn opacity(value: f32) -> Option<f32> {
    let value = if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        1.0
    };
    (value > 0.0).then_some(value)
}

#[cfg(test)]
#[cfg(test)]
#[path = "draw/tests.rs"]
mod tests;
