use winit::{application::ApplicationHandler, event_loop::ActiveEventLoop, window::Window};

use crate::render::Renderer;

pub struct App {
    title: String,
}

impl App {
    pub fn new<T: Into<String>>(title: T) -> Self {
        Self {
            title: title.into(),
        }
    }

    pub fn run<F>(self, mut render_callback: F)
    where
        F: FnMut(&mut Renderer, f32) + 'static,
    {
        let event_loop = ActiveEventLoop::new().unwrap();

        let window = event_loop.create_window(window_attributes).unwrap();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
    }
}
