use std::sync::Arc;

use brul_gui::Frame;
use winit::window::Window;

use crate::gpu::Gpu;

pub(crate) trait Surface {
    fn scale_factor(&self) -> f64;
    fn drag(&self);
    fn redraw(&self);
    fn size(&self) -> (u32, u32);
    fn pre_present_notify(&self);
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

    fn size(&self) -> (u32, u32) {
        let size = self.as_ref().surface_size();
        (size.width, size.height)
    }

    fn pre_present_notify(&self) {
        self.as_ref().pre_present_notify();
    }
}

pub(crate) trait Renderer {
    fn resize(&mut self, width: u32, height: u32);
    fn present(&mut self, surface: &dyn Surface, frame: &Frame);
}

impl Renderer for Gpu {
    fn resize(&mut self, width: u32, height: u32) {
        Gpu::resize(self, width, height);
    }

    fn present(&mut self, surface: &dyn Surface, frame: &Frame) {
        self.present_frame(surface, frame);
    }
}
