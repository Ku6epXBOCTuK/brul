use std::{sync::Arc, time::Duration};

use crate::{Color, events::Event, render::Renderer};
use tokio::{runtime::Runtime, time::Instant};
use winit::{application::ApplicationHandler, event_loop::ActiveEventLoop, window::Window};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    mouse_pos: Option<(f32, f32)>,
    last_render_time: Option<Instant>,
    start_time: Option<Instant>,
    runtime: Option<Runtime>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        if self.runtime.is_none() {
            self.runtime = Some(Runtime::new().unwrap());
        }

        let win_attr = Window::default_attributes().with_title("BRUL test");
        let window = event_loop
            .create_window(win_attr)
            .expect("Create window error");
        let window = Arc::new(window);
        let window_clone = Arc::clone(&window);
        if let Some(runtime) = &self.runtime {
            let renderer = runtime.block_on(async { Renderer::new(window_clone).await });
            self.renderer = Some(renderer);
        }

        self.window = Some(window);
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

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }
        if self.renderer.is_none() {
            return;
        }

        let now = Instant::now();

        if let Some(last) = self.last_render_time
            && now.duration_since(last) < Duration::from_millis(16)
        {
            return;
        }
        self.last_render_time = Some(now);

        let renderer = self.renderer.as_mut().unwrap();

        let elapsed = self.start_time.unwrap().elapsed().as_secs_f32();

        let color = Color {
            r: (elapsed.sin() * 0.5 + 0.5) as f32,
            g: ((elapsed + 2.0).sin() * 0.5 + 0.5) as f32,
            b: ((elapsed + 4.0).sin() * 0.5 + 0.5) as f32,
            a: 1.0,
        };

        renderer.clear(color);
    }
}
