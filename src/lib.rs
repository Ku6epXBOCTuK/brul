use winit::event_loop::{ControlFlow, EventLoop};

mod app;
pub mod core;
mod event_loop;
mod events;
mod render;
mod widget;

pub use app::App;
pub use core::{Color, Edges, Point, Rect, Size};
pub use events::{Event, EventContext};
pub use widget::{Rectangle, Widget};

pub fn start_app() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    let _ = event_loop.run_app(&mut app);
}
