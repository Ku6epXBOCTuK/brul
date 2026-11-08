use std::fmt;

use crate::Color;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TextureError {
    Decode(String),
    InvalidRgba,
}

impl fmt::Display for TextureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(message) => write!(formatter, "failed to decode PNG: {message}"),
            Self::InvalidRgba => formatter.write_str("invalid RGBA data"),
        }
    }
}

impl std::error::Error for TextureError {}

pub struct Texture {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    tint: Option<Color>,
}

impl Texture {
    pub fn from_png(bytes: &[u8]) -> Result<Self, TextureError> {
        let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
            .map_err(|error| TextureError::Decode(error.to_string()))?
            .to_rgba8();
        let (width, height) = image.dimensions();

        Ok(Self {
            width,
            height,
            pixels: image.into_raw(),
            tint: None,
        })
    }

    pub fn from_rgba(width: u32, height: u32, pixels: &[u8]) -> Result<Self, TextureError> {
        if width == 0 || height == 0 {
            return Err(TextureError::InvalidRgba);
        }

        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|count| count.checked_mul(4));
        if expected != Some(pixels.len()) {
            return Err(TextureError::InvalidRgba);
        }

        Ok(Self {
            width,
            height,
            pixels: pixels.to_vec(),
            tint: None,
        })
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

    pub fn tint(&self) -> Option<Color> {
        self.tint
    }

    pub fn with_tint(mut self, tint: Color) -> Self {
        self.tint = Some(tint);
        self
    }

    pub fn without_tint(mut self) -> Self {
        self.tint = None;
        self
    }

    pub fn color_at(&self, x: u32, y: u32) -> Option<Color> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let index = ((y * self.width + x) * 4) as usize;
        let color = Color::rgba(
            self.pixels[index],
            self.pixels[index + 1],
            self.pixels[index + 2],
            self.pixels[index + 3],
        );

        Some(match self.tint {
            Some(tint) => color.multiply(tint),
            None => color,
        })
    }
}
