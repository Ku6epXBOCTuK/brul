use std::sync::{Arc, LazyLock, Mutex};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// === mouse events ===
    MouseMove {
        x: f32,
        y: f32,
    },
    MouseDown {
        button: MouseButton,
        x: f32,
        y: f32,
    },
    MouseUp {
        button: MouseButton,
        x: f32,
        y: f32,
    },
    MouseEnter,
    MouseLeave,

    /// === keyboard events ===
    KeyDown {
        key: KeyCode,
    },
    KeyUp {
        key: KeyCode,
    },

    /// === window events ===
    WindowResize {
        width: u32,
        height: u32,
    },
    WindowClose,

    /// === other events ===
    RequestRedraw,
    UnusedEvent,
}

#[non_exhaustive]
pub struct EventContext {
    x: f32,
    y: f32,
}

static EVENT_CONTEXT: LazyLock<Arc<Mutex<EventContext>>> =
    LazyLock::new(|| Arc::new(Mutex::new(EventContext::new())));

impl EventContext {
    pub fn new() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn update(&mut self, event: &Event) {
        if let Event::MouseMove { x, y } = event {
            self.x = *x;
            self.y = *y;
        }
    }
}

impl From<WindowEvent> for Event {
    fn from(event: WindowEvent) -> Self {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                EVENT_CONTEXT.lock().unwrap().update(&Event::MouseMove {
                    x: position.x as f32,
                    y: position.y as f32,
                });
                Event::MouseMove {
                    x: position.x as f32,
                    y: position.y as f32,
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                let EventContext { x, y } = *EVENT_CONTEXT.lock().unwrap();
                match state {
                    ElementState::Pressed => Event::MouseDown { button, x: x, y: y },
                    ElementState::Released => Event::MouseUp { button, x: x, y: y },
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    return match event.state {
                        ElementState::Pressed => Event::KeyDown { key: keycode },
                        ElementState::Released => Event::KeyUp { key: keycode },
                    };
                }
                Event::UnusedEvent
            }
            WindowEvent::Resized(size) => Event::WindowResize {
                width: size.width,
                height: size.height,
            },
            WindowEvent::CloseRequested => Event::WindowClose,
            WindowEvent::CursorEntered { .. } => Event::MouseEnter,
            WindowEvent::CursorLeft { .. } => Event::MouseLeave,
            WindowEvent::RedrawRequested => Event::RequestRedraw,
            _ => Event::UnusedEvent,
        }
    }
}
