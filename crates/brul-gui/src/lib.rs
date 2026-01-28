use brul_utils::{Point, Result};
use tokio::{runtime::Handle, time::Instant};
use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
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
    tasks: Vec<Box<dyn Fn() -> () + 'static>>,
    start_time: Instant,
    last_task_time: Instant,
    runtime: Handle,
}

impl GuiBackend {
    pub fn new(runtime: Handle, tasks: Vec<Box<dyn Fn() -> () + 'static>>) -> Self {
        let start_time = Instant::now();
        let last_task_time = Instant::now();
        Self {
            // event_loop,
            event_loop_proxy: None,
            window: None,
            start_time,
            tasks,
            last_task_time,
            runtime,
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
        event_loop.set_control_flow(ControlFlow::Poll);
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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();

        let elapsed = now.duration_since(self.last_task_time);

        if elapsed.as_millis() > 100 {
            self.last_task_time = now;
            tracing::info!("Try run tasks");
            for task in self.tasks.iter() {
                task();
            }
        }
    }
}
