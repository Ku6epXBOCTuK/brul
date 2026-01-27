use brul_utils::{Point, Result};
use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::Window,
};

// pub trait Window {
//     fn set_title(&self, title: &str);
//     fn set_size(&self, width: u32, height: u32);
//     fn show(&self);
//     fn hide(&self);
//     fn is_visible(&self) -> bool;
// }

pub trait Renderer {
    fn clear(&self, color: [f32; 4]);
    fn draw_text(&self, texture: &str, position: Point);
}

#[non_exhaustive]
pub struct GuiBackend {
    // event_loop: EventLoop<()>,
    event_loop_proxy: Option<EventLoopProxy<()>>,
    window: Option<Window>,
}

impl GuiBackend {
    pub fn new() -> Self {
        Self {
            // event_loop,
            event_loop_proxy: None,
            window: None,
        }
    }

    pub fn create_window(&self, title: &str, width: u32, height: u32) -> Window {
        // TODO: create window
        todo!()
    }

    pub fn create_renderer(&self) -> Box<dyn Renderer> {
        // TODO: create renderer
        todo!()
    }

    pub fn run(mut self) -> Result<()> {
        let event_loop = EventLoop::new()?;
        self.event_loop_proxy = Some(event_loop.create_proxy());

        event_loop.run_app(&mut self)?;

        Ok(())
    }
}

impl ApplicationHandler for GuiBackend {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        // let window = self.window.as_ref().unwrap();

        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            event => {
                tracing::info!("Window({:?}) event: {:?}", window_id, event);
            }
        }
    }
}
