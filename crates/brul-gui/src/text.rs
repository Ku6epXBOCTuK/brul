use std::fmt;

use cosmic_text::{Align, Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, Weight};

use crate::atlas::{Atlas, AtlasSlot};

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub family: Option<String>,
    pub font_size: f32,
    pub line_height: f32,
    pub weight: u16,
    pub align: TextAlign,
}

impl TextStyle {
    pub fn new(font_size: f32) -> Self {
        Self {
            family: None,
            font_size,
            line_height: font_size * 1.2,
            weight: 400,
            align: TextAlign::Left,
        }
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::new(14.0)
    }
}

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
    pub(crate) fn offset(self, container: u32, content: u32) -> u32 {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextLayout {
    pub size: TextSize,
    pub glyphs: Vec<PlacedGlyph>,
    pub carets: Vec<CaretStop>,
}

impl TextLayout {
    pub fn caret(&self, byte: usize) -> CaretPosition {
        self.carets
            .iter()
            .rev()
            .find(|stop| stop.byte <= byte)
            .map_or_default(|stop| CaretPosition {
                x: stop.x,
                y: stop.y,
                height: stop.height,
            })
    }

    pub fn caret_at(&self, x: i32, y: i32) -> usize {
        self.nearest(self.line_at(y), x)
    }

    pub fn caret_above(&self, byte: usize) -> Option<usize> {
        self.caret_neighbour(byte, false)
    }

    pub fn caret_below(&self, byte: usize) -> Option<usize> {
        self.caret_neighbour(byte, true)
    }

    fn caret_neighbour(&self, byte: usize, down: bool) -> Option<usize> {
        let current = self.caret(byte);
        let lines = self.lines();
        let index = lines.iter().position(|line| *line == current.y)?;
        let neighbour = if down {
            lines.get(index + 1)
        } else {
            lines.get(index.checked_sub(1)?)
        }?;
        Some(self.nearest(*neighbour, current.x))
    }

    fn lines(&self) -> Vec<i32> {
        let mut lines: Vec<i32> = Vec::new();
        for stop in &self.carets {
            if lines.last() != Some(&stop.y) {
                lines.push(stop.y);
            }
        }
        lines
    }

    fn line_at(&self, y: i32) -> i32 {
        self.lines()
            .into_iter()
            .min_by_key(|line| (*line - y).abs())
            .unwrap_or(0)
    }

    fn nearest(&self, line: i32, x: i32) -> usize {
        self.carets
            .iter()
            .filter(|stop| stop.y == line)
            .min_by_key(|stop| ((stop.x - x).abs(), stop.byte))
            .map_or(0, |stop| stop.byte)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextError {
    InvalidMetrics,
    InvalidScale,
    InvalidWidth,
    AtlasFull,
}

impl fmt::Display for TextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidMetrics => "invalid font metrics",
            Self::InvalidScale => "invalid DPI scale",
            Self::InvalidWidth => "invalid max width",
            Self::AtlasFull => "glyph atlas is full",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for TextError {}

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

    Ok(TextLayout {
        size: TextSize {
            width: ceil_pixels(width * scale),
            height: ceil_pixels(height * scale),
        },
        glyphs,
        carets,
    })
}

fn ceil_pixels(value: f32) -> u32 {
    value.max(0.0).ceil() as u32
}

#[cfg(test)]
mod tests {
    use super::TextError;

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
