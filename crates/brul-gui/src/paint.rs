use crate::atlas::Atlas;
use crate::draw::{
    DrawCommand, GlyphCommand, ImageCommand, RectCommand, ShapeCommand, draw_commands,
};
use crate::rect::{ClipRect, Rect};
use crate::widget::Dirty;
use crate::{Color, Shadow, WidgetSnapshot, WidgetSnapshotNode};

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

    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
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
        self.paint_impl(snapshot, atlas, dirty, true, true);
    }

    pub fn paint_content(&mut self, snapshot: &WidgetSnapshot, atlas: &Atlas, dirty: &Dirty) {
        self.paint_impl(snapshot, atlas, dirty, false, false);
    }

    fn paint_impl(
        &mut self,
        snapshot: &WidgetSnapshot,
        atlas: &Atlas,
        dirty: &Dirty,
        shapes: bool,
        content: bool,
    ) {
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

        let mut commands = Vec::new();
        draw_commands(snapshot, dirty, &mut commands);
        for command in &commands {
            match command {
                DrawCommand::Shape(shape) if shapes => self.paint_shape(snapshot, shape, clip),
                DrawCommand::Glyph(glyph) if content => {
                    self.paint_glyph(snapshot, glyph, atlas, clip)
                }
                DrawCommand::Image(image) if content => self.paint_image(snapshot, image, clip),
                DrawCommand::Rect(decoration) => self.paint_decoration(snapshot, decoration, clip),
                _ => {}
            }
        }
    }

    fn paint_shape(&mut self, snapshot: &WidgetSnapshot, shape: &ShapeCommand, clip: Clip) {
        let node = &snapshot.nodes[shape.node];
        if let Some(shadow) = shape.shadow {
            self.paint_shadow(node, &shape.rect, shadow, shape.opacity, clip);
        }
        if shape.draw_fill {
            self.paint_rect(&shape.rect, shape.opacity, clip, &node.clips);
        }
    }

    fn paint_shadow(
        &mut self,
        node: &WidgetSnapshotNode,
        rect: &Rect,
        shadow: Shadow,
        opacity: f32,
        clip: Clip,
    ) {
        let Some(area) = paint_clip(node, clip) else {
            return;
        };
        for y in area.top..area.bottom {
            for x in area.left..area.right {
                let coverage = shadow.coverage(x as i32, y as i32, rect);
                if coverage <= 0.0 || !visible(&node.clips, x as i32, y as i32) {
                    continue;
                }
                self.blend(x as i32, y as i32, shadow.color, opacity * coverage);
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
                let sample = rect.sample(x as i32, y as i32);
                if sample.border_coverage > 0.0 {
                    self.blend(
                        x as i32,
                        y as i32,
                        rect.border_color,
                        opacity * sample.border_coverage,
                    );
                }
                if sample.fill_coverage > 0.0 {
                    let color = rect.fill.at(rect, x as i32, y as i32);
                    self.blend(x as i32, y as i32, color, opacity * sample.fill_coverage);
                }
            }
        }
    }

    fn paint_glyph(
        &mut self,
        snapshot: &WidgetSnapshot,
        glyph: &GlyphCommand,
        atlas: &Atlas,
        clip: Clip,
    ) {
        let node = &snapshot.nodes[glyph.node];
        let Some(clip) = node_clip(node, clip) else {
            return;
        };
        let slot = glyph.slot;
        let pixels = atlas.pixels();
        for row in 0..slot.height {
            for column in 0..slot.width {
                let x = glyph.x.saturating_add(column as i32);
                let y = glyph.y.saturating_add(row as i32);
                if !clip.contains(i64::from(x), i64::from(y)) || !visible(&node.clips, x, y) {
                    continue;
                }
                let index = ((slot.y + row) as usize * atlas.width() as usize
                    + (slot.x + column) as usize)
                    * 4;
                let color = match glyph.tint {
                    Some(tint) => Color::rgba(
                        tint.r,
                        tint.g,
                        tint.b,
                        (u16::from(pixels[index + 3]) * u16::from(tint.a) / 255) as u8,
                    ),
                    None => Color::rgba(
                        pixels[index],
                        pixels[index + 1],
                        pixels[index + 2],
                        pixels[index + 3],
                    ),
                };
                self.blend_rgba(x, y, color, glyph.opacity);
            }
        }
    }

    fn paint_decoration(
        &mut self,
        snapshot: &WidgetSnapshot,
        decoration: &RectCommand,
        clip: Clip,
    ) {
        let node = &snapshot.nodes[decoration.node];
        let Some(clip) = node_clip(node, clip) else {
            return;
        };
        let [left, top, width, height] = decoration.rect;
        let right = left.saturating_add(width);
        let bottom = top.saturating_add(height);
        for y in top..bottom {
            for x in left..right {
                if !clip.contains(i64::from(x), i64::from(y)) || !visible(&node.clips, x, y) {
                    continue;
                }
                self.blend_rgba(x, y, decoration.color, decoration.opacity);
            }
        }
    }

    fn paint_image(&mut self, snapshot: &WidgetSnapshot, image: &ImageCommand, clip: Clip) {
        let node = &snapshot.nodes[image.node];
        let [left, top, raw_width, raw_height] = image.rect;
        let width = raw_width.max(0) as u32;
        let height = raw_height.max(0) as u32;
        let Some(area) = Clip::new(
            i64::from(left),
            i64::from(top),
            i64::from(left) + i64::from(width),
            i64::from(top) + i64::from(height),
        )
        .and_then(|bounds| bounds.intersect(clip)) else {
            return;
        };

        let texture = &image.texture;
        for y in area.top..area.bottom {
            for x in area.left..area.right {
                if !visible(&node.clips, x as i32, y as i32) {
                    continue;
                }
                let source_x = ((x - i64::from(left)) as u64 * u64::from(texture.width())
                    / u64::from(width)) as u32;
                let source_y = ((y - i64::from(top)) as u64 * u64::from(texture.height())
                    / u64::from(height)) as u32;
                let Some(mut color) = texture.color_at(source_x, source_y) else {
                    continue;
                };
                if let Some(tint) = image.tint {
                    color = color.multiply(tint);
                }
                self.blend(x as i32, y as i32, color, image.opacity);
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
