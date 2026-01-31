use crate::renderer::Renderer;
use brul_utils::{AppControlMessage, Color, GuiControlMessage, Result};
use std::{
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{self, ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

mod renderer;

#[non_exhaustive]
pub struct GuiBackend {
    event_loop: Option<EventLoop<GuiControlMessage>>,
    event_loop_proxy: EventLoopProxy<GuiControlMessage>,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    app_tx: mpsc::Sender<AppControlMessage>,
    next_frame_time: Instant,
}

impl GuiBackend {
    pub fn new(app_tx: mpsc::Sender<AppControlMessage>) -> Result<Self> {
        let event_loop = EventLoop::<GuiControlMessage>::with_user_event().build()?;
        let event_loop_proxy = event_loop.create_proxy();
        event_loop.set_control_flow(ControlFlow::Wait);
        Ok(Self {
            event_loop: Some(event_loop),
            event_loop_proxy,
            window: None,
            renderer: None,
            app_tx,
            next_frame_time: Instant::now(),
        })
    }

    pub fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.as_ref().request_redraw();
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

    // pub fn create_window(&self, title: &str, width: u32, height: u32) -> Window {
    //     // TODO: create window (primary window already exist)
    //     todo!()
    // }

    pub fn get_proxy(&self) -> EventLoopProxy<GuiControlMessage> {
        self.event_loop_proxy.clone()
    }

    pub fn run(mut self) -> Result<()> {
        let event_loop = self.event_loop.take().unwrap();
        event_loop.run_app(&mut self)?;
        Ok(())
    }
}

impl ApplicationHandler<GuiControlMessage> for GuiBackend {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();
        let window = Arc::new(window);
        self.window = Some(Arc::clone(&window));
        let renderer = pollster::block_on(Renderer::new(window));
        self.renderer = Some(renderer);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // let window = self.window.as_ref().unwrap();

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                tracing::trace!("RedrawRequested");
                let renderer = self.renderer.as_mut().unwrap();

                let color = Color {
                    r: 1.0,
                    g: 0.0,
                    b: 1.0,
                    a: 1.0,
                };
                renderer.clear(color);
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                    let send_result = self.app_tx.send(AppControlMessage::RequestShutdown);
                    if let Err(_) = send_result {
                        tracing::error!("Send message error");
                    }
                }
                tracing::info!("KeyEvent: {:?}", event);
            }
            _ => {
                // tracing::info!("Window({:?}) event: {:?}", window_id, event);
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: GuiControlMessage) {
        match event {
            GuiControlMessage::Shutdown => {
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        const FRAME_DURATION: Duration = Duration::from_nanos(16_666_667);

        // let command = self.gui_rx.try_recv();
        // if let Ok(ControlMessage::Shutdown) = command {
        //     event_loop.exit();
        // }

        let now = Instant::now();
        if now >= self.next_frame_time {
            while now >= self.next_frame_time {
                self.next_frame_time += FRAME_DURATION;
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame_time));
            self.request_redraw();
        }
    }
}
