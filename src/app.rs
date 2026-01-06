use winit::{application::ApplicationHandler, window::Window};

use crate::events::Event;

#[derive(Default)]
pub struct App {
    window: Option<Window>,
    mouse_pos: Option<(f32, f32)>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            let win_attr = Window::default_attributes().with_title("BRUL test");
            let window = event_loop
                .create_window(win_attr)
                .expect("Create window error");
            self.window = Some(window);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let event: Event = event.into();
        if let Event::MouseMove { x, y } = event {
            self.mouse_pos = Some((x, y));
        }
        match event {
            Event::MouseUp { button, x, y } => {
                println!("Mouse up: button: {:?}, x: {}, y: {}", button, x, y);
            }
            Event::MouseDown { button, x, y } => {
                println!("Mouse down: button: {:?}, x: {}, y: {}", button, x, y);
            }
            Event::WindowClose => {
                event_loop.exit();
            }
            Event::KeyDown { key } => {
                if key == winit::keyboard::KeyCode::Escape {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}
