use std::rc::Rc;

use crate::{
    AtlasFormat, AtlasSlot, CaretPosition, Color, Dirty, Gradient, Image, Paint, Rect, Shadow,
    TextLayout, Texture, WidgetId, WidgetKind, WidgetSnapshot, WidgetSnapshotNode, WidgetStyle,
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
        push_shape(index, node, opacity, out);
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
            WidgetKind::View | WidgetKind::Structure => {}
        }
    }
}

fn push_shape(index: usize, node: &WidgetSnapshotNode, opacity: f32, out: &mut Vec<DrawCommand>) {
    let shadow = (node.style.shadow.color.a > 0).then_some(node.style.shadow);
    let draw_fill = match node.style.background {
        Paint::Color(color) => color.a > 0,
        Paint::Gradient(Gradient::Linear { angle, from, .. }) => angle != 0.0 && from.a > 0,
    };
    if shadow.is_none() && !draw_fill {
        return;
    }
    let rect = Rect::new(node.x, node.y, node.width, node.height)
        .with_radius(node.style.border_radius)
        .with_fill(node.style.background)
        .with_border(node.border_width, node.style.border);
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
mod tests {
    use super::*;
    use crate::{CaretPosition, SelectionRect, Size, TextEngine, TextStyle, Widget, WidgetTree};

    fn input_snapshot() -> WidgetSnapshot {
        let root = Widget::column(vec![Widget::input("ab")]);
        let mut engine = TextEngine::new();
        let mut tree = WidgetTree::new(root);
        tree.layout(
            &mut engine,
            Size {
                width: 120,
                height: 80,
            },
            1.0,
        )
        .expect("layout");
        tree.snapshot().expect("snapshot").clone()
    }

    fn input_node(snapshot: &mut WidgetSnapshot) -> &mut WidgetSnapshotNode {
        snapshot.nodes.last_mut().expect("input")
    }

    #[test]
    fn vertical_offset_and_scroll_apply_to_glyphs_and_selection() {
        let mut snapshot = input_snapshot();
        let node = input_node(&mut snapshot);
        node.content_x = 5;
        node.content_y = 7;
        node.content_height = 40;
        node.scroll = 3;
        node.style.vertical_align = crate::VerticalAlign::Center;
        node.selection = vec![SelectionRect {
            x: 2,
            y: 4,
            width: 6,
            height: 8,
        }];
        let text = node.text.as_ref().expect("text");
        let first = text.glyphs[0];
        let height = text.size.height;

        let mut commands = Vec::new();
        draw_commands(&snapshot, &Dirty::all(), &mut commands);

        let offset = crate::VerticalAlign::Center.offset(40, height) as i32 - 3;
        let glyph = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Glyph(glyph) => Some(*glyph),
                _ => None,
            })
            .expect("glyph");
        assert_eq!((glyph.x, glyph.y), (5 + first.x, 7 + first.y + offset));
        let selection = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Rect(rect) => Some(*rect),
                _ => None,
            })
            .expect("selection");
        assert_eq!(selection.rect[1], 7 + 4 + offset);
    }

    #[test]
    fn preedit_shift_uses_the_caret_x() {
        let mut snapshot = input_snapshot();
        let mut engine = TextEngine::new();
        let preedit = Rc::new(
            engine
                .layout("abc", &TextStyle::new(10.0), None, 1.0)
                .expect("preedit"),
        );
        let node = input_node(&mut snapshot);
        node.content_x = 10;
        node.caret = Some(CaretPosition {
            x: 30,
            y: 2,
            height: 7,
        });
        node.preedit = Some(Rc::clone(&preedit));
        let last = preedit.glyphs.last().expect("preedit glyph");
        let shift = 30 - preedit.size.width as i32;

        let mut commands = Vec::new();
        draw_commands(&snapshot, &Dirty::all(), &mut commands);

        let glyph = commands
            .iter()
            .rev()
            .find_map(|command| match command {
                DrawCommand::Glyph(glyph) => Some(*glyph),
                _ => None,
            })
            .expect("preedit glyph");
        assert_eq!(glyph.x, 10 + shift + last.x);
        let underline = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Rect(rect) if rect.rect[2] == preedit.size.width as i32 => Some(*rect),
                _ => None,
            })
            .expect("underline");
        assert_eq!(underline.rect[0], 10 + shift);
    }

    #[test]
    fn caret_without_text_uses_a_clamped_height() {
        let mut snapshot = input_snapshot();
        let node = input_node(&mut snapshot);
        node.text = None;
        node.content_y = 5;
        node.content_height = 30;
        node.scroll = 2;
        node.style.vertical_align = crate::VerticalAlign::Bottom;
        node.caret = Some(CaretPosition {
            x: 1,
            y: -4,
            height: 6,
        });

        let mut commands = Vec::new();
        draw_commands(&snapshot, &Dirty::all(), &mut commands);

        let caret = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Rect(rect) => Some(*rect),
                _ => None,
            })
            .expect("caret");
        let offset = crate::VerticalAlign::Bottom.offset(30, 0) as i32 - 2;
        assert_eq!(caret.rect[1], 5 - 4 + offset);
    }

    #[test]
    fn preedit_without_caret_emits_no_composition_commands() {
        let mut snapshot = input_snapshot();
        let node = input_node(&mut snapshot);
        let mut engine = TextEngine::new();
        node.preedit = Some(Rc::new(
            engine
                .layout("abc", &TextStyle::new(10.0), None, 1.0)
                .expect("preedit"),
        ));
        node.caret = None;
        let glyph_count = node.text.as_ref().expect("text").glyphs.len();

        let mut commands = Vec::new();
        draw_commands(&snapshot, &Dirty::all(), &mut commands);

        assert_eq!(
            commands
                .iter()
                .filter(|command| matches!(command, DrawCommand::Glyph(_)))
                .count(),
            glyph_count
        );
        assert!(
            commands
                .iter()
                .all(|command| !matches!(command, DrawCommand::Rect(_)))
        );
    }

    #[test]
    fn dirty_limits_commands_to_the_listed_node() {
        let snapshot = input_snapshot();
        let first = snapshot.nodes[0].id;
        let dirty = Dirty {
            full: false,
            ids: vec![first],
        };

        let mut commands = Vec::new();
        draw_commands(&snapshot, &dirty, &mut commands);

        assert!(commands.iter().all(|command| match command {
            DrawCommand::Shape(shape) => shape.node == 0,
            DrawCommand::Glyph(glyph) => glyph.node == 0,
            DrawCommand::Rect(rect) => rect.node == 0,
            DrawCommand::Image(image) => image.node == 0,
        }));
    }

    #[test]
    fn image_tint_is_resolved_once() {
        let texture = Texture::from_rgba(1, 1, &[255, 255, 255, 255])
            .expect("texture")
            .with_tint(Color::rgb(10, 20, 30));
        let mut snapshot = input_snapshot();
        let node = input_node(&mut snapshot);
        node.kind = WidgetKind::Image;
        node.content_width = 4;
        node.content_height = 5;
        node.style.tint = Some(Color::rgb(40, 50, 60));
        node.image = Some(Image::from(texture));

        let mut commands = Vec::new();
        draw_commands(&snapshot, &Dirty::all(), &mut commands);

        let image = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Image(image) => Some(image),
                _ => None,
            })
            .expect("image");
        assert_eq!(
            image.tint,
            Some(Color::rgb(10, 20, 30).multiply(Color::rgb(40, 50, 60)))
        );
    }
}
