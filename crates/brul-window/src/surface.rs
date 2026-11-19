use std::sync::Arc;

use brul_gui::{Atlas, Dirty, WidgetSnapshot};
use winit::window::{Window, WindowLevel};

use crate::gpu::Gpu;
use crate::software::SoftwareRenderer;
use crate::window::{Backend, WindowError};

pub(crate) trait Surface {
    fn scale_factor(&self) -> f64;
    fn drag(&self);
    fn redraw(&self);
    fn set_visible(&self, visible: bool);
    fn focus(&self);
    fn size(&self) -> (u32, u32);
    fn pre_present_notify(&self);
    fn set_always_on_top(&self, on_top: bool);
}

impl Surface for Arc<dyn Window> {
    fn scale_factor(&self) -> f64 {
        Window::scale_factor(self.as_ref())
    }

    fn drag(&self) {
        let _ = self.as_ref().drag_window();
    }

    fn redraw(&self) {
        self.as_ref().request_redraw();
    }

    fn set_visible(&self, visible: bool) {
        self.as_ref().set_visible(visible);
        #[cfg(windows)]
        crate::layered::restore_layered_style(self);
    }

    fn focus(&self) {
        // NOTE: winit steals the foreground with a synthetic Alt via SendInput, which resets the system idle timer
        self.as_ref().focus_window();
    }

    fn size(&self) -> (u32, u32) {
        let size = self.as_ref().surface_size();
        (size.width, size.height)
    }

    fn pre_present_notify(&self) {
        self.as_ref().pre_present_notify();
    }

    fn set_always_on_top(&self, on_top: bool) {
        let level = if on_top {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        };
        self.as_ref().set_window_level(level);
    }
}

pub(crate) enum Renderer {
    Gpu(Box<Gpu>),
    Software(SoftwareRenderer),
    #[cfg(test)]
    Test(Box<dyn TestRenderer>),
}

#[cfg(test)]
pub(crate) trait TestRenderer {
    fn resize(&mut self, width: u32, height: u32);
    fn clear(&mut self) -> bool;
    fn render(&mut self, snapshot: &WidgetSnapshot, atlas: &Atlas, dirty: &Dirty) -> bool;
    fn capture(&mut self) -> Option<Vec<u8>>;
}

impl Renderer {
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        match self {
            Self::Gpu(gpu) => gpu.resize(width, height),
            Self::Software(software) => software.resize(width, height),
            #[cfg(test)]
            Self::Test(renderer) => renderer.resize(width, height),
        }
    }

    pub(crate) fn clear(&mut self, surface: &dyn Surface) -> bool {
        match self {
            Self::Gpu(gpu) => gpu.clear_surface(surface),
            Self::Software(software) => software.clear_buffer(),
            #[cfg(test)]
            Self::Test(renderer) => renderer.clear(),
        }
    }

    pub(crate) fn render(
        &mut self,
        surface: &dyn Surface,
        snapshot: &WidgetSnapshot,
        atlas: &Atlas,
        dirty: &Dirty,
    ) -> bool {
        match self {
            Self::Gpu(gpu) => gpu.render(surface, snapshot, atlas, dirty),
            Self::Software(software) => software.render(surface, snapshot, atlas, dirty),
            #[cfg(test)]
            Self::Test(renderer) => renderer.render(snapshot, atlas, dirty),
        }
    }

    pub(crate) fn capture(&mut self) -> Option<Vec<u8>> {
        match self {
            Self::Gpu(gpu) => gpu.capture(),
            Self::Software(software) => software.capture(),
            #[cfg(test)]
            Self::Test(renderer) => renderer.capture(),
        }
    }
}

pub(crate) fn create_renderer(
    backend: Backend,
    window: &Arc<dyn Window>,
    transparent: bool,
) -> Result<Renderer, WindowError> {
    match backend {
        Backend::Software => SoftwareRenderer::new(window, transparent).map(Renderer::Software),
        Backend::Gpu => Gpu::new(window, transparent).map(|gpu| Renderer::Gpu(Box::new(gpu))),
    }
}
