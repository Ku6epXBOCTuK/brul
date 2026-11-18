use std::sync::Arc;

use brul_gui::Frame;
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
    fn present(&mut self, surface: &dyn Surface, frame: &Frame) -> bool;
}

impl Renderer for Gpu {
    fn resize(&mut self, width: u32, height: u32) {
        Gpu::resize(self, width, height);
    }

    fn present(&mut self, surface: &dyn Surface, frame: &Frame) -> bool {
        self.present_frame(surface, frame)
    }
}

impl Renderer for SoftwareRenderer {
    fn resize(&mut self, width: u32, height: u32) {
        SoftwareRenderer::resize(self, width, height);
    }

    fn present(&mut self, surface: &dyn Surface, frame: &Frame) -> bool {
        self.present(surface, frame)
    }
}

pub(crate) fn create_renderer(
    backend: Backend,
    window: &Arc<dyn Window>,
) -> Result<Box<dyn Renderer>, WindowError> {
    let window = Arc::clone(window);
    create_renderer_with(
        backend,
        || Ok(Box::new(Gpu::new(&window)?)),
        || {
            Ok(Box::new(
                SoftwareRenderer::new(&window).map_err(WindowError::window)?,
            ))
        },
    )
}

fn create_renderer_with(
    backend: Backend,
    gpu: impl FnOnce() -> Result<Box<dyn Renderer>, WindowError>,
    software: impl FnOnce() -> Result<Box<dyn Renderer>, WindowError>,
) -> Result<Box<dyn Renderer>, WindowError> {
    match backend {
        Backend::Software => software(),
        Backend::Gpu => gpu(),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    struct Stub;

    impl Renderer for Stub {
        fn resize(&mut self, _width: u32, _height: u32) {}

        fn present(&mut self, _surface: &dyn Surface, _frame: &Frame) -> bool {
            true
        }
    }

    #[test]
    fn the_software_backend_does_not_create_a_gpu() {
        let calls = RefCell::new(Vec::new());
        let renderer = create_renderer_with(
            Backend::Software,
            || {
                calls.borrow_mut().push("gpu");
                Err(WindowError::NoContent)
            },
            || {
                calls.borrow_mut().push("software");
                Ok(Box::new(Stub))
            },
        );
        assert!(renderer.is_ok());
        assert_eq!(calls.into_inner(), ["software"]);
    }

    #[test]
    fn the_gpu_backend_does_not_create_a_software_surface() {
        let calls = RefCell::new(Vec::new());
        let renderer = create_renderer_with(
            Backend::Gpu,
            || {
                calls.borrow_mut().push("gpu");
                Ok(Box::new(Stub))
            },
            || {
                calls.borrow_mut().push("software");
                Err(WindowError::NoContent)
            },
        );
        assert!(renderer.is_ok());
        assert_eq!(calls.into_inner(), ["gpu"]);
    }
}
