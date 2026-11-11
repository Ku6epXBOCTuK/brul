use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use brul_gui::{WidgetEvent, WidgetTree};

use crate::gpu::Gpu;
use crate::input::{Input, event_input, window_input};

const CLEAR: wgpu::Color = wgpu::Color {
    r: 0.08,
    g: 0.09,
    b: 0.11,
    a: 0.85,
};

#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub header_height: u32,
    pub frameless: bool,
    pub transparent: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: String::from("BRUL"),
            width: 960,
            height: 640,
            header_height: 48,
            frameless: true,
            transparent: true,
        }
    }
}

impl WindowConfig {
    pub fn validate(&self) -> Result<(), WindowError> {
        if self.width == 0 || self.height == 0 {
            return Err(WindowError::EmptySize);
        }
        if self.header_height > self.height {
            return Err(WindowError::HeaderTooTall);
        }
        Ok(())
    }

    pub(crate) fn is_header(&self, y: f64, scale: f64) -> bool {
        y >= 0.0 && y < f64::from(self.header_height) * scale
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorScheme {
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowKey(pub u64);

impl std::fmt::Display for WindowKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "window {}", self.0)
    }
}

pub struct WindowSpec {
    pub config: WindowConfig,
    pub tree: Rc<RefCell<WidgetTree>>,
    pub commands: Vec<WindowCommand>,
}

#[derive(Debug)]
pub enum WindowError {
    EmptySize,
    HeaderTooTall,
    NoSurfaceConfig,
    Stage {
        stage: &'static str,
        message: String,
    },
}

impl WindowError {
    pub(crate) fn stage(stage: &'static str, error: impl std::fmt::Display) -> Self {
        Self::Stage {
            stage,
            message: error.to_string(),
        }
    }
}

impl std::fmt::Display for WindowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySize => formatter.write_str("window size must be greater than zero"),
            Self::HeaderTooTall => formatter.write_str("header is taller than the window"),
            Self::NoSurfaceConfig => formatter.write_str("no suitable surface configuration"),
            Self::Stage { stage, message } => write!(formatter, "{stage}: {message}"),
        }
    }
}

impl std::error::Error for WindowError {}

pub fn run(config: WindowConfig) -> Result<(), WindowError> {
    config.validate()?;
    let event_loop = EventLoop::new().map_err(|error| WindowError::stage("event loop", error))?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let error = Rc::new(Cell::new(None));
    let app = App {
        config,
        window: None,
        gpu: None,
        error: Rc::clone(&error),
    };
    let result = event_loop.run_app(app);

    if let Some(error) = error.take() {
        return Err(error);
    }
    result.map_err(|error| WindowError::stage("event loop", error))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowCommand {
    Exit,
    Close,
    Open(WindowKey),
    Drag,
    Every {
        interval: Duration,
        function: String,
    },
    SetTheme(String),
}

pub fn run_tree(
    config: WindowConfig,
    tree: Rc<RefCell<WidgetTree>>,
    initial: Vec<WindowCommand>,
    handler: impl FnMut(&[WidgetEvent]) -> Vec<WindowCommand> + 'static,
) -> Result<(), WindowError> {
    run_tree_themed(config, initial, move |_| Rc::clone(&tree), handler)
}

pub fn run_tree_themed(
    config: WindowConfig,
    initial: Vec<WindowCommand>,
    mut build: impl FnMut(ColorScheme) -> Rc<RefCell<WidgetTree>> + 'static,
    handler: impl FnMut(&[WidgetEvent]) -> Vec<WindowCommand> + 'static,
) -> Result<(), WindowError> {
    config.validate()?;
    let event_loop = EventLoop::new().map_err(|error| WindowError::stage("event loop", error))?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let error = Rc::new(Cell::new(None));
    let tree = build(ColorScheme::Dark);
    let app = crate::tree::tree_app(
        config,
        tree,
        Some(Box::new(build)),
        Box::new(handler),
        Rc::clone(&error),
        initial,
    );
    let result = event_loop.run_app(app);

    if let Some(error) = error.take() {
        return Err(error);
    }
    result.map_err(|error| WindowError::stage("event loop", error))
}

pub fn run_windows(
    initial: WindowKey,
    create: impl FnMut(WindowKey, ColorScheme) -> Result<WindowSpec, String> + 'static,
    handler: impl FnMut(WindowKey, &[WidgetEvent]) -> Vec<WindowCommand> + 'static,
    closed: impl FnMut(WindowKey) + 'static,
) -> Result<(), WindowError> {
    crate::windows::run(initial, create, handler, closed)
}

pub(crate) fn create_window(
    event_loop: &dyn ActiveEventLoop,
    config: &WindowConfig,
) -> Result<Arc<dyn Window>, WindowError> {
    event_loop
        .create_window(attributes(config))
        .map(Arc::from)
        .map_err(|error| WindowError::stage("window creation", error))
}

pub(crate) fn attributes(config: &WindowConfig) -> WindowAttributes {
    let size = LogicalSize::new(f64::from(config.width), f64::from(config.height));
    let mut attributes = WindowAttributes::default()
        .with_title(config.title.clone())
        .with_surface_size(size)
        .with_resizable(!config.frameless)
        .with_decorations(!config.frameless)
        .with_transparent(config.transparent)
        .with_visible(false);
    if config.frameless {
        attributes = attributes
            .with_min_surface_size(size)
            .with_max_surface_size(size);
    }
    attributes
}

struct App {
    config: WindowConfig,
    window: Option<Arc<dyn Window>>,
    gpu: Option<Gpu>,
    error: Rc<Cell<Option<WindowError>>>,
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = match create_window(event_loop, &self.config) {
            Ok(window) => window,
            Err(error) => return self.fail(event_loop, error),
        };
        let gpu = match Gpu::new(&window) {
            Ok(gpu) => gpu,
            Err(error) => return self.fail(event_loop, error),
        };

        self.window = Some(Arc::clone(&window));
        self.gpu = Some(gpu);
        window.set_visible(true);
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self
            .window
            .as_ref()
            .filter(|window| window.id() == window_id)
        else {
            return;
        };
        let window = Arc::clone(window);

        let input = window_input(&event).or_else(|| event_input(&event));
        match input {
            Some(Input::Exit) => event_loop.exit(),
            Some(Input::Down(_, y)) if self.config.is_header(y, window.scale_factor()) => {
                let _ = window.drag_window();
            }
            Some(Input::Surface) => {
                let size = window.surface_size();
                self.resize(size.width, size.height);
                window.request_redraw();
            }
            Some(Input::Redraw) => self.render(&window),
            _ => {}
        }
    }
}

impl App {
    fn fail(&mut self, event_loop: &dyn ActiveEventLoop, error: WindowError) {
        self.error.set(Some(error));
        event_loop.exit();
    }

    fn resize(&mut self, width: u32, height: u32) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        gpu.resize(width, height);
    }

    fn render(&mut self, window: &Arc<dyn Window>) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        let frame = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                gpu.surface.configure(&gpu.device, &gpu.config);
                window.request_redraw();
                return;
            }
            _ => return,
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("brul-window"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("brul-clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        gpu.queue.submit(Some(encoder.finish()));
        window.pre_present_notify();
        gpu.queue.present(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::{WindowConfig, WindowError};

    #[test]
    fn default_config_is_valid() {
        assert!(WindowConfig::default().validate().is_ok());
    }

    #[test]
    fn zero_size_is_rejected() {
        let config = WindowConfig {
            width: 0,
            ..WindowConfig::default()
        };
        assert!(matches!(config.validate(), Err(WindowError::EmptySize)));
    }

    #[test]
    fn header_taller_than_window_is_rejected() {
        let config = WindowConfig {
            height: 40,
            header_height: 41,
            ..WindowConfig::default()
        };
        assert!(matches!(config.validate(), Err(WindowError::HeaderTooTall)));
    }

    #[test]
    fn window_error_keeps_the_stage_names_apart() {
        let messages = [
            WindowError::EmptySize.to_string(),
            WindowError::HeaderTooTall.to_string(),
            WindowError::NoSurfaceConfig.to_string(),
        ];
        let mut unique = messages.to_vec();
        unique.sort();
        unique.dedup();

        assert_eq!(unique.len(), messages.len());
        assert_eq!(
            WindowError::stage("surface", "boom").to_string(),
            "surface: boom"
        );
    }

    #[test]
    fn a_frameless_window_has_no_decorations_and_a_fixed_size() {
        let config = WindowConfig {
            width: 488,
            height: 800,
            transparent: true,
            ..WindowConfig::default()
        };

        let attributes = super::attributes(&config);

        assert!(!attributes.decorations);
        assert!(!attributes.resizable);
        assert!(attributes.transparent);
        assert!(attributes.min_surface_size.is_some());
        assert!(attributes.max_surface_size.is_some());
    }

    #[test]
    fn a_decorated_window_is_resizable_without_a_size_cap() {
        let config = WindowConfig {
            title: "Settings".to_owned(),
            width: 760,
            height: 560,
            frameless: false,
            transparent: false,
            ..WindowConfig::default()
        };

        let attributes = super::attributes(&config);

        assert_eq!(attributes.title, "Settings");
        assert!(attributes.decorations);
        assert!(attributes.resizable);
        assert!(!attributes.transparent);
        assert!(attributes.min_surface_size.is_none());
        assert!(attributes.max_surface_size.is_none());
    }

    #[test]
    fn header_hit_test_uses_physical_pixels() {
        let config = WindowConfig {
            header_height: 40,
            ..WindowConfig::default()
        };
        assert!(config.is_header(0.0, 1.0));
        assert!(config.is_header(39.9, 1.0));
        assert!(!config.is_header(40.0, 1.0));
        assert!(!config.is_header(-1.0, 1.0));
        assert!(config.is_header(79.0, 2.0));
        assert!(!config.is_header(80.0, 2.0));
    }
}
