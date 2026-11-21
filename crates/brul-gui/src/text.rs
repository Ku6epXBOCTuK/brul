use cosmic_text::{Align, Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, Weight};

use crate::atlas::{Atlas, AtlasSlot};

include!(concat!(env!("OUT_DIR"), "/style_text.rs"));

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl TextAlign {
    fn cosmic(self) -> Align {
        match self {
            Self::Left => Align::Left,
            Self::Center => Align::Center,
            Self::Right => Align::Right,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum VerticalAlign {
    #[default]
    Top,
    Center,
    Bottom,
}

impl VerticalAlign {
    pub fn offset(self, container: u32, content: u32) -> u32 {
        let free = container.saturating_sub(content);
        match self {
            Self::Top => 0,
            Self::Center => free / 2,
            Self::Bottom => free,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlacedGlyph {
    pub x: i32,
    pub y: i32,
    pub slot: AtlasSlot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaretStop {
    pub byte: usize,
    pub x: i32,
    pub y: i32,
    pub height: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CaretPosition {
    pub x: i32,
    pub y: i32,
    pub height: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SelectionRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TextLine {
    y: i32,
    height: i32,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextLayout {
    pub size: TextSize,
    pub glyphs: Vec<PlacedGlyph>,
    pub carets: Vec<CaretStop>,
    lines: Vec<TextLine>,
}

impl TextLayout {
    fn new(size: TextSize, glyphs: Vec<PlacedGlyph>, mut carets: Vec<CaretStop>) -> Self {
        carets.sort_by_key(|stop| stop.byte);
        let mut lines: Vec<TextLine> = Vec::new();
        for (index, stop) in carets.iter().enumerate() {
            match lines.last_mut() {
                Some(line) if line.y == stop.y => line.end = index + 1,
                _ => lines.push(TextLine {
                    y: stop.y,
                    height: stop.height,
                    start: index,
                    end: index + 1,
                }),
            }
        }
        Self {
            size,
            glyphs,
            carets,
            lines,
        }
    }

    pub fn caret(&self, byte: usize) -> CaretPosition {
        self.carets[..self.carets.partition_point(|stop| stop.byte <= byte)]
            .last()
            .map_or_default(|stop| CaretPosition {
                x: stop.x,
                y: stop.y,
                height: stop.height,
            })
    }

    pub fn caret_at(&self, x: i32, y: i32) -> usize {
        self.nearest(self.line_at(y), x)
    }

    pub fn line_start(&self, byte: usize) -> usize {
        let line = self.caret(byte).y;
        self.line(line)
            .map_or(0, |line| self.carets[line.start].byte)
    }

    pub fn line_end(&self, byte: usize) -> usize {
        let line = self.caret(byte).y;
        self.line(line)
            .map_or(0, |line| self.carets[line.end - 1].byte)
    }

    pub fn selection(&self, anchor: usize, caret: usize) -> Vec<SelectionRect> {
        let (start, end) = if anchor <= caret {
            (anchor, caret)
        } else {
            (caret, anchor)
        };
        let mut rects = Vec::new();
        for line in &self.lines {
            let stops = &self.carets[line.start..line.end];
            let line_start = stops.first().map_or(0, |stop| stop.byte);
            let line_end = stops.last().map_or(0, |stop| stop.byte);
            let from = start.max(line_start);
            let to = end.min(line_end);
            if from >= to {
                continue;
            }
            let x = self.caret(from).x;
            let width = self.caret(to).x.saturating_sub(x);
            if width <= 0 {
                continue;
            }
            rects.push(SelectionRect {
                x,
                y: line.y,
                width,
                height: line.height,
            });
        }
        rects
    }

    pub fn caret_above(&self, byte: usize) -> Option<usize> {
        self.caret_neighbour(byte, false)
    }

    pub fn caret_below(&self, byte: usize) -> Option<usize> {
        self.caret_neighbour(byte, true)
    }

    fn caret_neighbour(&self, byte: usize, down: bool) -> Option<usize> {
        let current = self.caret(byte);
        let index = self.lines.iter().position(|line| line.y == current.y)?;
        let neighbour = if down {
            self.lines.get(index + 1)
        } else {
            self.lines.get(index.checked_sub(1)?)
        }?;
        Some(self.nearest(neighbour.y, current.x))
    }

    fn line(&self, y: i32) -> Option<&TextLine> {
        self.lines.iter().find(|line| line.y == y)
    }

    fn line_at(&self, y: i32) -> i32 {
        self.lines
            .iter()
            .min_by_key(|line| (line.y - y).abs())
            .map_or(0, |line| line.y)
    }

    fn nearest(&self, line_y: i32, x: i32) -> usize {
        let Some(line) = self.line(line_y) else {
            return 0;
        };
        self.carets[line.start..line.end]
            .iter()
            .min_by_key(|stop| ((stop.x - x).abs(), stop.byte))
            .map_or(0, |stop| stop.byte)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TextError {
    #[error("invalid font metrics")]
    InvalidMetrics,
    #[error("invalid DPI scale")]
    InvalidScale,
    #[error("invalid max width")]
    InvalidWidth,
    #[error("glyph atlas is full")]
    AtlasFull,
}

pub struct TextEngine {
    font_system: FontSystem,
    swash: SwashCache,
    atlas: Atlas,
    buffer: Option<Buffer>,
}

impl TextEngine {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash: SwashCache::new(),
            atlas: Atlas::new(),
            buffer: None,
        }
    }

    pub fn atlas(&self) -> &Atlas {
        &self.atlas
    }

    pub fn layout(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f32>,
        scale: f32,
    ) -> Result<TextLayout, TextError> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(TextError::InvalidScale);
        }
        if !style.font_size.is_finite()
            || style.font_size <= 0.0
            || !style.line_height.is_finite()
            || style.line_height <= 0.0
        {
            return Err(TextError::InvalidMetrics);
        }
        if let Some(width) = max_width
            && (!width.is_finite() || width < 0.0)
        {
            return Err(TextError::InvalidWidth);
        }

        let metrics = Metrics::new(style.font_size, style.line_height);
        let mut buffer = match self.buffer.take() {
            Some(buffer) => buffer,
            None => Buffer::new(&mut self.font_system, metrics),
        };
        buffer.set_metrics(metrics);
        buffer.set_size(max_width, None);

        let mut attrs = Attrs::new().weight(Weight(style.weight));
        if let Some(family) = style.family.as_deref() {
            attrs = attrs.family(Family::Name(family));
        }
        buffer.set_text(text, &attrs, Shaping::Advanced, Some(style.align.cosmic()));
        buffer.shape_until_scroll(&mut self.font_system, false);

        let result = {
            let Self {
                font_system,
                swash,
                atlas,
                ..
            } = self;
            place_glyphs(font_system, swash, atlas, &buffer, style, scale)
        };

        self.buffer = Some(buffer);
        result
    }
}

impl Default for TextEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn place_glyphs(
    font_system: &mut FontSystem,
    swash: &mut SwashCache,
    atlas: &mut Atlas,
    buffer: &Buffer,
    style: &TextStyle,
    scale: f32,
) -> Result<TextLayout, TextError> {
    let mut width: f32 = 0.0;
    let mut height: f32 = 0.0;
    let mut glyphs = Vec::new();
    let mut carets = Vec::new();
    let mut line = 0;
    let mut line_start = 0;
    let mut previous_len = 0;

    for run in buffer.layout_runs() {
        width = width.max(run.line_w);
        height = height.max(run.line_top + run.line_height);
        if run.line_i != line {
            line_start += previous_len + 1 + (run.line_i - line - 1);
            line = run.line_i;
        }
        previous_len = run.text.len();
        let stop = CaretStop {
            byte: 0,
            x: (run.line_w * scale).round() as i32,
            y: (run.line_top * scale).round() as i32,
            height: (run.line_height * scale).round() as i32,
        };

        for glyph in run.glyphs {
            let physical = glyph.physical((0.0, run.line_y * scale), scale);
            carets.push(CaretStop {
                byte: line_start + glyph.start,
                x: physical.x,
                ..stop
            });
            let Some(image) = swash.get_image(font_system, physical.cache_key) else {
                continue;
            };
            let Some(slot) = atlas.insert(physical.cache_key, image) else {
                return Err(TextError::AtlasFull);
            };
            if slot.width == 0 || slot.height == 0 {
                continue;
            }
            glyphs.push(PlacedGlyph {
                x: physical.x + image.placement.left,
                y: physical.y - image.placement.top,
                slot,
            });
        }
        carets.push(CaretStop {
            byte: line_start + run.text.len(),
            ..stop
        });
    }

    if height == 0.0 {
        height = style.line_height;
    }

    Ok(TextLayout::new(
        TextSize {
            width: ceil_pixels(width * scale),
            height: ceil_pixels(height * scale),
        },
        glyphs,
        carets,
    ))
}

fn ceil_pixels(value: f32) -> u32 {
    value.max(0.0).ceil() as u32
}

#[cfg(test)]
mod tests {
    use super::{CaretStop, SelectionRect, TextError, TextLayout, TextSize};

    fn stop(byte: usize, x: i32, y: i32) -> CaretStop {
        CaretStop {
            byte,
            x,
            y,
            height: 20,
        }
    }

    fn layout() -> TextLayout {
        TextLayout::new(
            TextSize {
                width: 60,
                height: 40,
            },
            Vec::new(),
            vec![
                stop(3, 0, 20),
                stop(0, 0, 0),
                stop(5, 20, 20),
                stop(1, 10, 0),
                stop(4, 10, 20),
                stop(2, 20, 0),
            ],
        )
    }

    #[test]
    fn caret_lookup_uses_sorted_stops() {
        let layout = layout();
        assert_eq!(layout.caret(0).x, 0);
        assert_eq!(layout.caret(1).x, 10);
        assert_eq!(layout.caret(2).x, 20);
        assert_eq!(layout.caret(3).x, 0);
        assert_eq!(layout.caret(usize::MAX).x, 20);
        assert_eq!(layout.caret_at(12, 21), 4);
    }

    #[test]
    fn line_queries_use_cached_line_starts() {
        let layout = layout();
        assert_eq!(layout.line_start(1), 0);
        assert_eq!(layout.line_end(1), 2);
        assert_eq!(layout.line_start(4), 3);
        assert_eq!(layout.line_end(4), 5);
        assert_eq!(layout.caret_above(4), Some(1));
        assert_eq!(layout.caret_below(1), Some(4));
    }

    #[test]
    fn selection_spans_only_intersecting_cached_lines() {
        let layout = layout();
        assert_eq!(
            layout.selection(1, 4),
            vec![
                SelectionRect {
                    x: 10,
                    y: 0,
                    width: 10,
                    height: 20,
                },
                SelectionRect {
                    x: 0,
                    y: 20,
                    width: 10,
                    height: 20,
                },
            ]
        );
        assert!(layout.selection(4, 4).is_empty());
    }

    #[test]
    fn every_text_error_has_its_own_message() {
        let messages = [
            TextError::InvalidMetrics,
            TextError::InvalidScale,
            TextError::InvalidWidth,
            TextError::AtlasFull,
        ]
        .map(|error| error.to_string());

        assert!(messages.iter().all(|message| !message.is_empty()));
        let mut unique = messages.to_vec();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), messages.len());
    }
}
