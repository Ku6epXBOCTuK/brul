#[cfg(not(windows))]
use std::num::NonZeroU32;
use std::sync::Arc;

use brul_gui::{Atlas, Color, Dirty, Frame, WidgetSnapshot};
#[cfg(not(windows))]
use softbuffer::{Context, Surface as BufferSurface};
#[cfg(not(windows))]
use tracing::warn;
use winit::window::Window;

#[cfg(windows)]
use crate::layered::LayeredSurface;
use crate::surface::Surface;
use crate::window::{CLEAR, WindowError};

#[cfg(windows)]
type SurfacePresenter = LayeredSurface;
#[cfg(not(windows))]
type SurfacePresenter = SoftbufferPresenter;

#[cfg(windows)]
const SOFTWARE_ALPHA: bool = true;
#[cfg(not(windows))]
const SOFTWARE_ALPHA: bool = false;

pub(crate) struct SoftwareRenderer {
    presenter: SurfacePresenter,
    frame: Frame,
    transparent: bool,
}

impl SoftwareRenderer {
    pub(crate) fn capture(&mut self) -> Option<Vec<u8>> {
        Some(self.frame.pixels().to_vec())
    }

    pub(crate) fn new(window: &Arc<dyn Window>, transparent: bool) -> Result<Self, WindowError> {
        let presenter = SurfacePresenter::new(window)?;
        Ok(Self {
            presenter,
            frame: Frame::new(1, 1),
            transparent,
        })
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.presenter.resize(width, height);
    }

    pub(crate) fn clear_buffer(&mut self) -> bool {
        #[cfg(windows)]
        {
            if !self.transparent {
                return self.presenter.clear_opaque();
            }
        }
        self.presenter.clear()
    }

    pub(crate) fn render(
        &mut self,
        surface: &dyn Surface,
        snapshot: &WidgetSnapshot,
        atlas: &Atlas,
        dirty: &Dirty,
    ) -> bool {
        let (width, height) = surface.size();
        self.frame.resize(width.max(1), height.max(1));
        self.presenter.resize(width, height);
        self.frame.paint(snapshot, atlas, dirty);
        if !self.transparent || !SOFTWARE_ALPHA {
            self.frame.composite(clear_color());
        }
        self.presenter.present(self.frame.pixels())
    }
}

#[cfg(not(windows))]
pub(crate) struct SoftbufferPresenter {
    _context: Context<Arc<dyn Window>>,
    surface: BufferSurface<Arc<dyn Window>, Arc<dyn Window>>,
    size: (u32, u32),
}

#[cfg(not(windows))]
impl SoftbufferPresenter {
    fn new(window: &Arc<dyn Window>) -> Result<Self, WindowError> {
        let context = Context::new(Arc::clone(window)).map_err(WindowError::window)?;
        let surface =
            BufferSurface::new(&context, Arc::clone(window)).map_err(WindowError::window)?;
        Ok(Self {
            _context: context,
            surface,
            size: (0, 0),
        })
    }
}

#[cfg(not(windows))]
impl SoftbufferPresenter {
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

    pub(crate) fn clear(&mut self) -> bool {
        let Ok(mut buffer) = self.surface.buffer_mut() else {
            return false;
        };
        let background = clear_rgb();
        for pixel in buffer.iter_mut() {
            *pixel = background;
        }
        buffer.present().is_ok()
    }

    pub(crate) fn present(&mut self, pixels: &[u8]) -> bool {
        let Ok(mut buffer) = self.surface.buffer_mut() else {
            return false;
        };
        for (target, pixel) in buffer.iter_mut().zip(pixels.as_chunks::<4>().0) {
            *target = rgb(pixel);
        }
        buffer.present().is_ok()
    }
}

fn clear_color() -> Color {
    let channel = |value: f64| (value * 255.0).round() as u8;
    Color::rgb(channel(CLEAR.r), channel(CLEAR.g), channel(CLEAR.b))
}

#[cfg(not(windows))]
fn clear_rgb() -> u32 {
    let Color { r, g, b, .. } = clear_color();
    (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

#[cfg(not(windows))]
fn rgb(pixel: &[u8; 4]) -> u32 {
    (u32::from(pixel[0]) << 16) | (u32::from(pixel[1]) << 8) | u32::from(pixel[2])
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;

    #[test]
    fn a_pixel_keeps_its_rgb_when_alpha_is_dropped() {
        assert_eq!(rgb(&[0x12, 0x34, 0x56, 255]), 0x00_12_34_56);
    }
}
