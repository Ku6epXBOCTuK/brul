use std::sync::Arc;

use crate::renderer::Renderer;
use brul_utils::{Color, Point, Result};
use tokio::{runtime::Handle, time::Instant};
use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::Window,
};

mod renderer;

#[non_exhaustive]
pub struct GuiBackend {
    // event_loop: EventLoop<()>,
    event_loop_proxy: Option<EventLoopProxy<()>>,
    window: Option<Arc<Window>>,
    tasks: Vec<Box<dyn Fn() -> () + 'static>>,
    start_time: Instant,
    last_task_time: Instant,
    runtime: Handle,
    renderer: Option<Renderer>,
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
            renderer: None,
        }
    }

    pub async fn create_renderer(&mut self) {
        if self.renderer.is_some() {
            tracing::info!("Renderer already created");
            return;
        }
        if let Some(window) = &self.window {
            let window = Arc::clone(window);
            let renderer = Renderer::new(window).await;
            self.renderer = Some(renderer);
            tracing::info!("Renderer created");
        } else {
            tracing::info!("Window dont exist");
        }
    }

    pub fn create_window(&self, title: &str, width: u32, height: u32) -> Window {
        // TODO: create window (primary window already exist)
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
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();
        let window = Arc::new(window);
        self.window = Some(Arc::clone(&window));
        let renderer = self.runtime.block_on(Renderer::new(window));
        self.renderer = Some(renderer);
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

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let now = Instant::now();

        let elapsed = now.duration_since(self.last_task_time);

        // TODO: add rate for each task (eg 60 per second\100 per second)
        if elapsed.as_millis() < 100 {
            return;
        }

        self.last_task_time = now;
        tracing::info!("Try run tasks");
        for task in self.tasks.iter() {
            task();
        }

        if let Some(renderer) = &mut self.renderer {
            let elapsed = self.start_time.elapsed().as_secs_f32();

            let color = Color {
                r: (elapsed.sin() * 0.5 + 0.5) as f32,
                g: ((elapsed + 2.0).sin() * 0.5 + 0.5) as f32,
                b: ((elapsed + 4.0).sin() * 0.5 + 0.5) as f32,
                a: 1.0,
            };
            renderer.clear(color);
        }
    }
}
