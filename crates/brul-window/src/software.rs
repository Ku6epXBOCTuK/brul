use std::num::NonZeroU32;
use std::sync::Arc;

use brul_gui::Frame;
use softbuffer::{Context, SoftBufferError, Surface as BufferSurface};
use tracing::warn;
use winit::window::Window;

use crate::surface::Surface;
use crate::window::CLEAR;

pub(crate) struct SoftwareRenderer {
    _context: Context<Arc<dyn Window>>,
    surface: BufferSurface<Arc<dyn Window>, Arc<dyn Window>>,
    size: (u32, u32),
}

impl SoftwareRenderer {
    pub(crate) fn new(window: &Arc<dyn Window>) -> Result<Self, SoftBufferError> {
        let context = Context::new(Arc::clone(window))?;
        let surface = BufferSurface::new(&context, Arc::clone(window))?;
        Ok(Self {
            _context: context,
            surface,
            size: (0, 0),
        })
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 || self.size == (width, height) {
            return;
        }
        let (Some(width), Some(height)) = (NonZeroU32::new(width), NonZeroU32::new(height)) else {
            return;
        };
        if let Err(error) = self.surface.resize(width, height) {
            warn!(?error, "software surface resize failed");
            return;
        }
        self.size = (width.get(), height.get());
    }

    pub(crate) fn clear_buffer(&mut self) -> bool {
        let Ok(mut buffer) = self.surface.buffer_mut() else {
            return false;
        };
        let background = pack(&[0, 0, 0, 0]);
        for pixel in buffer.iter_mut() {
            *pixel = background;
        }
        buffer.present().is_ok()
    }

    pub(crate) fn present(&mut self, _surface: &dyn Surface, frame: &Frame) -> bool {
        self.resize(frame.width(), frame.height());
        if self.size != (frame.width(), frame.height()) {
            return false;
        }
        let Ok(mut buffer) = self.surface.buffer_mut() else {
            return false;
        };
        for (target, pixel) in buffer.iter_mut().zip(frame.pixels().as_chunks::<4>().0) {
            *target = pack(pixel);
        }
        buffer.present().is_ok()
    }
}

fn pack(pixel: &[u8; 4]) -> u32 {
    let alpha = u32::from(pixel[3]);
    let inverse = 255 - alpha;
    let channel = |value: u8, background: f64| -> u32 {
        let back = (background * 255.0).round() as u32;
        (u32::from(value) * alpha + back * inverse) / 255
    };
    let red = channel(pixel[0], CLEAR.r);
    let green = channel(pixel[1], CLEAR.g);
    let blue = channel(pixel[2], CLEAR.b);
    (red << 16) | (green << 8) | blue
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_opaque_pixel_keeps_its_channels() {
        assert_eq!(pack(&[0x12, 0x34, 0x56, 255]), 0x00_12_34_56);
    }

    #[test]
    fn a_transparent_pixel_uses_the_clear_color() {
        let background = ((CLEAR.r * 255.0).round() as u32) << 16
            | ((CLEAR.g * 255.0).round() as u32) << 8
            | (CLEAR.b * 255.0).round() as u32;
        assert_eq!(pack(&[255, 255, 255, 0]), background);
    }
}
