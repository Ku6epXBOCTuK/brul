use std::sync::Arc;

use brul_gui::Frame;
use winit::window::{Window, WindowLevel};

use crate::gpu::Gpu;

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
