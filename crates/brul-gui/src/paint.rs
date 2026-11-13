use crate::atlas::{Atlas, AtlasFormat};
use crate::rect::{ClipRect, Rect, RectSample};
use crate::widget::Dirty;
use crate::{Color, Image, WidgetSnapshot, WidgetSnapshotNode};

use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Frame {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Frame {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; pixel_len(width, height)],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;
        self.pixels = vec![0; pixel_len(width, height)];
    }

    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }

    pub fn paint(&mut self, snapshot: &WidgetSnapshot, atlas: &Atlas, dirty: &Dirty) {
        if self.width == 0 || self.height == 0 {
            return;
        }

        let full = dirty.full || dirty.ids.iter().any(|id| snapshot.node(*id).is_none());
        let clip = if full {
            Some(Clip::frame(self.width, self.height))
        } else {
            let Some(region) = dirty_region(snapshot, dirty) else {
                return;
            };
            region.intersect(Clip::frame(self.width, self.height))
        };
        let Some(clip) = clip else {
            return;
        };

        if full {
            self.clear();
        } else {
            self.clear_region(clip);
        }

        for node in &snapshot.nodes {
            if paint_clip(node, clip).is_some() {
                self.paint_node(node, atlas, clip);
            }
        }
    }

    fn paint_node(&mut self, node: &WidgetSnapshotNode, atlas: &Atlas, clip: Clip) {
        let opacity = opacity(node.style.opacity);
        let rect = Rect::new(node.x, node.y, node.width, node.height)
            .with_radius(node.style.radius)
            .with_fill(node.style.background)
            .with_border(node.border_width, node.style.border);
        self.paint_shadow(node, &rect, opacity, clip);
        self.paint_rect(&rect, opacity, clip, &node.clips);

        match node.kind {
            crate::WidgetKind::Text => {
                if let Some(text) = &node.text {
                    self.paint_text(node, text, atlas, opacity, clip);
                }
            }
            crate::WidgetKind::Image => {
                if let Some(image) = &node.image {
                    self.paint_image(node, image, opacity, clip);
                }
            }
            crate::WidgetKind::Input => {
                self.paint_selection(node, &node.selection, opacity, clip);
                if let Some(text) = &node.text {
                    self.paint_text(node, text, atlas, opacity, clip);
                }
                if let Some(preedit) = &node.preedit
                    && let Some(caret) = node.caret
                {
                    self.paint_preedit(node, preedit, caret, atlas, opacity, clip);
                }
                if let Some(caret) = node.caret {
                    self.paint_caret(node, caret, opacity, clip);
                }
            }
            crate::WidgetKind::View => {}
        }
    }

    fn paint_shadow(&mut self, node: &WidgetSnapshotNode, rect: &Rect, opacity: f32, clip: Clip) {
        let shadow = node.style.shadow;
        let Some(area) = paint_clip(node, clip) else {
            return;
        };
        for y in area.top..area.bottom {
            for x in area.left..area.right {
                if !visible(&node.clips, x as i32, y as i32) {
                    continue;
                }
                let coverage = shadow.coverage(x as i32, y as i32, rect);
                if coverage > 0.0 {
                    self.blend(x as i32, y as i32, shadow.color, opacity * coverage);
                }
            }
        }
    }

    fn paint_rect(&mut self, rect: &Rect, opacity: f32, clip: Clip, clips: &[ClipRect]) {
        let Some(area) = Clip::new(
            i64::from(rect.x),
            i64::from(rect.y),
            i64::from(rect.x) + i64::from(rect.width),
            i64::from(rect.y) + i64::from(rect.height),
        )
        .and_then(|bounds| bounds.intersect(clip)) else {
            return;
        };

        for y in area.top..area.bottom {
            for x in area.left..area.right {
                if !visible(clips, x as i32, y as i32) {
                    continue;
                }
                let color = match rect.sample(x as i32, y as i32) {
                    RectSample::Outside => continue,
                    RectSample::Fill(color) | RectSample::Border(color) => color,
                };
                self.blend(x as i32, y as i32, color, opacity);
            }
        }
    }

    fn paint_text(
        &mut self,
        node: &WidgetSnapshotNode,
        text: &crate::TextLayout,
        atlas: &Atlas,
        opacity: f32,
        clip: Clip,
    ) {
        self.paint_glyphs(node, text, 0, atlas, opacity, clip);
    }

    fn paint_glyphs(
        &mut self,
        node: &WidgetSnapshotNode,
        text: &crate::TextLayout,
        shift: i32,
        atlas: &Atlas,
        opacity: f32,
        clip: Clip,
    ) {
        let Some(clip) = node_clip(node, clip) else {
            return;
        };
        let color = node.style.text;
        let offset = node
            .style
            .vertical_align
            .offset(node.content_height, text.size.height) as i32
            - node.scroll;
        for glyph in &text.glyphs {
            let slot = glyph.slot;
            let origin_x = node.content_x.saturating_add(shift).saturating_add(glyph.x);
            let origin_y = node
                .content_y
                .saturating_add(glyph.y)
                .saturating_add(offset);
            for row in 0..slot.height {
                for column in 0..slot.width {
                    let x = origin_x.saturating_add(column as i32);
                    let y = origin_y.saturating_add(row as i32);
                    if !clip.contains(i64::from(x), i64::from(y)) || !visible(&node.clips, x, y) {
                        continue;
                    }
                    let index = ((slot.y + row) as usize * atlas.width() as usize
                        + (slot.x + column) as usize)
                        * 4;
                    let pixels = atlas.pixels();
                    let (red, green, blue, alpha) = match slot.format {
                        AtlasFormat::Mask => (
                            color.r,
                            color.g,
                            color.b,
                            (u16::from(pixels[index + 3]) * u16::from(color.a) / 255) as u8,
                        ),
                        AtlasFormat::Color => (
                            pixels[index],
                            pixels[index + 1],
                            pixels[index + 2],
                            pixels[index + 3],
                        ),
                    };
                    self.blend_rgba(x, y, Color::rgba(red, green, blue, alpha), opacity);
                }
            }
        }
    }

    fn paint_selection(
        &mut self,
        node: &WidgetSnapshotNode,
        selection: &[crate::SelectionRect],
        opacity: f32,
        clip: Clip,
    ) {
        let Some(clip) = node_clip(node, clip) else {
            return;
        };
        let height = node.text.as_ref().map_or(0, |text| text.size.height as i32);
        let offset = node
            .style
            .vertical_align
            .offset(node.content_height, height as u32) as i32
            - node.scroll;
        let color = node.style.text.with_alpha(60);
        for rect in selection {
            let origin_x = node.content_x.saturating_add(rect.x);
            let right = origin_x.saturating_add(rect.width);
            let top = node.content_y.saturating_add(rect.y).saturating_add(offset);
            let bottom = top.saturating_add(rect.height);
            for y in top..bottom {
                for x in origin_x..right {
                    if !clip.contains(i64::from(x), i64::from(y)) || !visible(&node.clips, x, y) {
                        continue;
                    }
                    self.blend_rgba(x, y, color, opacity);
                }
            }
        }
    }

    fn paint_preedit(
        &mut self,
        node: &WidgetSnapshotNode,
        preedit: &crate::TextLayout,
        caret: crate::CaretPosition,
        atlas: &Atlas,
        opacity: f32,
        clip: Clip,
    ) {
        let shift = caret.x.saturating_sub(preedit.size.width as i32);
        self.paint_glyphs(node, preedit, shift, atlas, opacity, clip);
        let Some(clip) = node_clip(node, clip) else {
            return;
        };
        let height = node
            .text
            .as_ref()
            .map_or(caret.y, |text| text.size.height as i32);
        let offset = node
            .style
            .vertical_align
            .offset(node.content_height, height as u32) as i32
            - node.scroll;
        let top = node
            .content_y
            .saturating_add(caret.y)
            .saturating_add(offset)
            .saturating_add(preedit.size.height as i32)
            .saturating_sub(1);
        let left = node.content_x.saturating_add(shift);
        let right = left.saturating_add(preedit.size.width as i32);
        for x in left..right {
            if !clip.contains(i64::from(x), i64::from(top)) || !visible(&node.clips, x, top) {
                continue;
            }
            self.blend_rgba(x, top, node.style.text, opacity);
        }
    }

    fn paint_caret(
        &mut self,
        node: &WidgetSnapshotNode,
        caret: crate::CaretPosition,
        opacity: f32,
        clip: Clip,
    ) {
        let Some(clip) = node_clip(node, clip) else {
            return;
        };
        let color = node.style.text;
        let height = node
            .text
            .as_ref()
            .map_or(caret.y, |text| text.size.height as i32);
        let offset = node
            .style
            .vertical_align
            .offset(node.content_height, height as u32) as i32
            - node.scroll;
        let origin_x = node.content_x.saturating_add(caret.x);
        let top = node
            .content_y
            .saturating_add(caret.y)
            .saturating_add(offset);
        let bottom = top.saturating_add(caret.height);
        for y in top..bottom {
            if !clip.contains(i64::from(origin_x), i64::from(y))
                || !visible(&node.clips, origin_x, y)
            {
                continue;
            }
            self.blend_rgba(origin_x, y, color, opacity);
        }
    }

    fn paint_image(&mut self, node: &WidgetSnapshotNode, image: &Image, opacity: f32, clip: Clip) {
        if node.content_width == 0 || node.content_height == 0 {
            return;
        }
        let (texture, tint) = match image {
            Image::Raster(texture) => (Rc::clone(texture), node.style.tint),
            Image::Vector(vector) => {
                let current = node.style.tint.unwrap_or(node.style.text);
                match vector.raster(node.content_width, node.content_height, current) {
                    Some(texture) => (texture, None),
                    None => return,
                }
            }
        };
        let Some(area) = Clip::new(
            i64::from(node.content_x),
            i64::from(node.content_y),
            i64::from(node.content_x) + i64::from(node.content_width),
            i64::from(node.content_y) + i64::from(node.content_height),
        )
        .and_then(|bounds| bounds.intersect(clip)) else {
            return;
        };

        for y in area.top..area.bottom {
            for x in area.left..area.right {
                if !visible(&node.clips, x as i32, y as i32) {
                    continue;
                }
                let source_x = ((x - i64::from(node.content_x)) as u64 * u64::from(texture.width())
                    / u64::from(node.content_width)) as u32;
                let source_y = ((y - i64::from(node.content_y)) as u64
                    * u64::from(texture.height())
                    / u64::from(node.content_height)) as u32;
                let Some(mut color) = texture.color_at(source_x, source_y) else {
                    continue;
                };
                if let Some(tint) = tint {
                    color = color.multiply(tint);
                }
                self.blend(x as i32, y as i32, color, opacity);
            }
        }
    }

    fn clear_region(&mut self, clip: Clip) {
        let width = self.width as usize;
        for y in clip.top..clip.bottom {
            let start = (y as usize * width + clip.left as usize) * 4;
            let end = (y as usize * width + clip.right as usize) * 4;
            self.pixels[start..end].fill(0);
        }
    }

    fn blend(&mut self, x: i32, y: i32, color: Color, opacity: f32) {
        self.blend_rgba(x, y, color, opacity);
    }

    fn blend_rgba(&mut self, x: i32, y: i32, color: Color, opacity: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let alpha = f32::from(color.a) / 255.0 * opacity;
        if alpha <= 0.0 {
            return;
        }

        let index = ((y as usize * self.width as usize) + x as usize) * 4;
        let inverse = 1.0 - alpha;
        let source_red = f32::from(color.r) / 255.0 * alpha;
        let source_green = f32::from(color.g) / 255.0 * alpha;
        let source_blue = f32::from(color.b) / 255.0 * alpha;

        let destination_red = f32::from(self.pixels[index]) / 255.0;
        let destination_green = f32::from(self.pixels[index + 1]) / 255.0;
        let destination_blue = f32::from(self.pixels[index + 2]) / 255.0;
        let destination_alpha = f32::from(self.pixels[index + 3]) / 255.0;

        self.pixels[index] =
            ((source_red + destination_red * inverse).clamp(0.0, 1.0) * 255.0).round() as u8;
        self.pixels[index + 1] =
            ((source_green + destination_green * inverse).clamp(0.0, 1.0) * 255.0).round() as u8;
        self.pixels[index + 2] =
            ((source_blue + destination_blue * inverse).clamp(0.0, 1.0) * 255.0).round() as u8;
        self.pixels[index + 3] =
            ((alpha + destination_alpha * inverse).clamp(0.0, 1.0) * 255.0).round() as u8;
    }
}

#[derive(Clone, Copy)]
struct Clip {
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
}

impl Clip {
    fn frame(width: u32, height: u32) -> Self {
        Self {
            left: 0,
            top: 0,
            right: i64::from(width),
            bottom: i64::from(height),
        }
    }

    fn new(left: i64, top: i64, right: i64, bottom: i64) -> Option<Self> {
        (right > left && bottom > top).then_some(Self {
            left,
            top,
            right,
            bottom,
        })
    }

    fn intersect(self, other: Self) -> Option<Self> {
        Self::new(
            self.left.max(other.left),
            self.top.max(other.top),
            self.right.min(other.right),
            self.bottom.min(other.bottom),
        )
    }

    fn contains(self, x: i64, y: i64) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

fn visible(clips: &[ClipRect], x: i32, y: i32) -> bool {
    clips.iter().all(|clip| clip.contains(x, y))
}

fn node_clip(node: &WidgetSnapshotNode, clip: Clip) -> Option<Clip> {
    Clip::new(
        i64::from(node.x),
        i64::from(node.y),
        i64::from(node.x) + i64::from(node.width),
        i64::from(node.y) + i64::from(node.height),
    )
    .and_then(|bounds| bounds.intersect(clip))
}

fn paint_bounds(node: &WidgetSnapshotNode) -> Option<Clip> {
    let shadow = node.style.shadow;
    let left = i64::from(node.x);
    let top = i64::from(node.y);
    let right = left + i64::from(node.width);
    let bottom = top + i64::from(node.height);
    let offset_x = i64::from(shadow.x);
    let offset_y = i64::from(shadow.y);
    let blur = i64::from(shadow.blur);
    Clip::new(
        left.min(left + offset_x - blur),
        top.min(top + offset_y - blur),
        right.max(right + offset_x + blur),
        bottom.max(bottom + offset_y + blur),
    )
}

fn paint_clip(node: &WidgetSnapshotNode, clip: Clip) -> Option<Clip> {
    paint_bounds(node).and_then(|bounds| bounds.intersect(clip))
}

fn dirty_region(snapshot: &WidgetSnapshot, dirty: &Dirty) -> Option<Clip> {
    let mut region: Option<Clip> = None;
    for id in &dirty.ids {
        let Some(node) = snapshot.node(*id) else {
            continue;
        };
        let Some(bounds) = paint_bounds(node) else {
            continue;
        };
        region = Some(match region {
            Some(region) => Clip {
                left: region.left.min(bounds.left),
                top: region.top.min(bounds.top),
                right: region.right.max(bounds.right),
                bottom: region.bottom.max(bounds.bottom),
            },
            None => bounds,
        });
    }
    region
}

fn opacity(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn pixel_len(width: u32, height: u32) -> usize {
    width as usize * height as usize * 4
}

#[cfg(test)]
mod tests {
    use super::Frame;

    #[test]
    fn resize_keeps_the_buffer_of_the_same_size() {
        let mut frame = Frame::new(2, 3);

        frame.resize(2, 3);

        assert_eq!((frame.width(), frame.height()), (2, 3));
        assert_eq!(frame.pixels().len(), 24);
    }

    #[test]
    fn resize_reallocates_the_buffer() {
        let mut frame = Frame::new(2, 3);

        frame.resize(0, 0);
        assert_eq!((frame.width(), frame.height()), (0, 0));
        assert!(frame.pixels().is_empty());

        frame.resize(4, 1);
        assert_eq!((frame.width(), frame.height()), (4, 1));
        assert_eq!(frame.pixels().len(), 16);
        assert!(frame.pixels().iter().all(|byte| *byte == 0));
    }
}
