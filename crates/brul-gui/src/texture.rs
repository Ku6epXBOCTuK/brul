use resvg::usvg;

use crate::Color;

#[derive(Debug, thiserror::Error)]
pub enum TextureError {
    #[error("failed to decode a PNG image")]
    Png(#[from] image::ImageError),
    #[error("image data is not valid UTF-8")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("failed to parse an SVG image")]
    Svg(#[from] usvg::Error),
    #[error("invalid RGBA data")]
    InvalidRgba,
}

pub struct Texture {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    tint: Option<Color>,
}

impl Texture {
    pub fn from_png(bytes: &[u8]) -> Result<Self, TextureError> {
        let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.to_rgba8();
        let (width, height) = image.dimensions();

        Ok(Self {
            width,
            height,
            pixels: image.into_raw(),
            tint: None,
        })
    }

    pub fn empty() -> Self {
        Self {
            width: 1,
            height: 1,
            pixels: vec![0, 0, 0, 0],
            tint: None,
        }
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

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::{Texture, TextureError};

    #[test]
    fn texture_error_keeps_the_decoder_source() {
        let Err(error) = Texture::from_png(b"not a png") else {
            panic!("invalid png decoded");
        };

        assert!(matches!(error, TextureError::Png(_)));
        assert!(
            error
                .source()
                .expect("source")
                .downcast_ref::<image::ImageError>()
                .is_some()
        );
        assert_ne!(error.to_string(), TextureError::InvalidRgba.to_string());
    }
}
