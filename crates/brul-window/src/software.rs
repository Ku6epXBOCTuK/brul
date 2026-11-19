use std::num::NonZeroU32;
use std::sync::Arc;

use brul_gui::{Atlas, Dirty, Frame, WidgetSnapshot};
use softbuffer::{Context, Surface as BufferSurface};
use tracing::warn;
use winit::window::Window;

use crate::surface::{Presenter, Surface};

#[cfg(windows)]
use crate::layered::LayeredSurface;
use crate::window::{CLEAR, WindowError};

pub(crate) struct SoftwareRenderer {
    presenter: Presenter,
    frame: Frame,
}

impl SoftwareRenderer {
    pub(crate) fn capture(&mut self) -> Option<Vec<u8>> {
        Some(self.frame.pixels().to_vec())
    }

    pub(crate) fn new(window: &Arc<dyn Window>, transparent: bool) -> Result<Self, WindowError> {
        let presenter = make_presenter(window, transparent)?;
        Ok(Self {
            presenter,
            frame: Frame::new(1, 1),
        })
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.presenter.resize(width, height);
    }

    pub(crate) fn clear_buffer(&mut self) -> bool {
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
        self.presenter.present(self.frame.pixels())
    }
}

pub(crate) struct SoftbufferPresenter {
    _context: Context<Arc<dyn Window>>,
    surface: BufferSurface<Arc<dyn Window>, Arc<dyn Window>>,
    size: (u32, u32),
}

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
        let background = pack(&[0, 0, 0, 0]);
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
            *target = pack(pixel);
        }
        buffer.present().is_ok()
    }
}

#[cfg(windows)]
fn make_presenter(window: &Arc<dyn Window>, transparent: bool) -> Result<Presenter, WindowError> {
    if transparent {
        return LayeredSurface::new(window).map(Presenter::Layered);
    }
    SoftbufferPresenter::new(window).map(Presenter::Softbuffer)
}

#[cfg(not(windows))]
fn make_presenter(window: &Arc<dyn Window>, _transparent: bool) -> Result<Presenter, WindowError> {
    SoftbufferPresenter::new(window).map(Presenter::Softbuffer)
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
