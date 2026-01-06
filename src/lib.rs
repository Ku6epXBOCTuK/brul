use winit::event_loop::{ControlFlow, EventLoop};

use crate::app::App;

mod app;
mod event_loop;
mod events;

pub fn start_app() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    let _ = event_loop.run_app(&mut app);
}
