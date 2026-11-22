use std::rc::Rc;

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Revision(u64);

impl Revision {
    pub const START: Self = Self(0);

    pub fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

#[derive(Clone, Copy)]
pub enum TextureFrame<'a> {
    Rgba(&'a [u8]),
    Yuv420 {
        y: &'a [u8],
        u: &'a [u8],
        v: &'a [u8],
        strides: [usize; 3],
    },
}

pub trait ExternalTexture {
    fn revision(&self) -> Revision;
    fn frame(&self) -> TextureFrame<'_>;
}

enum Source {
    Rgba(Vec<u8>),
    External(Rc<dyn ExternalTexture>),
}

pub struct Texture {
    width: u32,
    height: u32,
    source: Source,
    tint: Option<Color>,
}

impl Texture {
    pub fn from_png(bytes: &[u8]) -> Result<Self, TextureError> {
        let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.to_rgba8();
        let (width, height) = image.dimensions();

        Ok(Self {
            width,
            height,
            source: Source::Rgba(image.into_raw()),
            tint: None,
        })
    }

    pub fn empty() -> Self {
        Self {
            width: 1,
            height: 1,
            source: Source::Rgba(vec![0, 0, 0, 0]),
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
            source: Source::Rgba(pixels.to_vec()),
            tint: None,
        })
    }

    pub fn external(
        width: u32,
        height: u32,
        source: Rc<dyn ExternalTexture>,
    ) -> Result<Self, TextureError> {
        if width == 0 || height == 0 {
            return Err(TextureError::InvalidRgba);
        }

        Ok(Self {
            width,
            height,
            source: Source::External(source),
            tint: None,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn rgba(&self) -> Option<&[u8]> {
        match &self.source {
            Source::Rgba(pixels) => Some(pixels),
            Source::External(_) => None,
        }
    }

    pub fn external_source(&self) -> Option<&Rc<dyn ExternalTexture>> {
        match &self.source {
            Source::Rgba(_) => None,
            Source::External(source) => Some(source),
        }
    }

    pub fn revision(&self) -> Revision {
        match &self.source {
            Source::Rgba(_) => Revision::START,
            Source::External(source) => source.revision(),
        }
    }

    pub fn frame(&self) -> TextureFrame<'_> {
        match &self.source {
            Source::Rgba(pixels) => TextureFrame::Rgba(pixels),
            Source::External(source) => source.frame(),
        }
    }

    pub fn is_yuv(&self) -> bool {
        matches!(self.frame(), TextureFrame::Yuv420 { .. })
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

        let color = match self.frame() {
            TextureFrame::Rgba(pixels) => {
                let index = ((y * self.width + x) * 4) as usize;
                Color::rgba(
                    pixels[index],
                    pixels[index + 1],
                    pixels[index + 2],
                    pixels[index + 3],
                )
            }
            TextureFrame::Yuv420 {
                y: plane,
                u,
                v,
                strides,
            } => {
                let luma = plane[y as usize * strides[0] + x as usize];
                let chroma_x = (x / 2) as usize;
                let chroma_y = (y / 2) as usize;
                let blue = u[chroma_y * strides[1] + chroma_x];
                let red = v[chroma_y * strides[2] + chroma_x];
                yuv_to_color(luma, blue, red)
            }
        };

        Some(match self.tint {
            Some(tint) => color.multiply(tint),
            None => color,
        })
    }
}

fn yuv_to_color(luma: u8, blue: u8, red: u8) -> Color {
    let luma = (f32::from(luma) - 16.0) * (255.0 / 219.0);
    let blue = (f32::from(blue) - 128.0) * (255.0 / 224.0);
    let red = (f32::from(red) - 128.0) * (255.0 / 224.0);
    let r = (luma + red * 1.5748).clamp(0.0, 255.0) as u8;
    let g = (luma - blue * 0.1873 - red * 0.4681).clamp(0.0, 255.0) as u8;
    let b = (luma + blue * 1.8556).clamp(0.0, 255.0) as u8;
    Color::rgba(r, g, b, 255)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::error::Error as _;
    use std::rc::Rc;

    use super::{ExternalTexture, Revision, Texture, TextureError, TextureFrame};

    struct Source {
        revision: Cell<Revision>,
    }

    impl ExternalTexture for Source {
        fn revision(&self) -> Revision {
            self.revision.get()
        }

        fn frame(&self) -> TextureFrame<'_> {
            TextureFrame::Yuv420 {
                y: &[16, 16, 16, 16],
                u: &[128],
                v: &[128],
                strides: [2, 1, 1],
            }
        }
    }

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

    #[test]
    fn an_external_texture_reports_its_revision_and_frame() {
        let source = Rc::new(Source {
            revision: Cell::new(Revision::START),
        });
        let texture = Texture::external(2, 2, Rc::clone(&source) as Rc<dyn ExternalTexture>)
            .expect("external texture");

        assert!(texture.rgba().is_none());
        assert_eq!(texture.revision(), Revision::START);
        assert!(matches!(texture.frame(), TextureFrame::Yuv420 { .. }));
        let color = texture.color_at(0, 0).expect("pixel");
        assert!(color.r < 10 && color.g < 10 && color.b < 10);

        source.revision.set(Revision::START.next());
        assert_eq!(texture.revision(), Revision::START.next());
    }
}
