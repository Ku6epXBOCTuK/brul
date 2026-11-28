use super::*;
use tracing::warn;

#[derive(Debug, thiserror::Error)]
enum PlacementError {
    #[error("no primary monitor")]
    NoMonitor,
    #[error("monitor origin is unavailable")]
    NoOrigin,
    #[error("monitor video mode is unavailable")]
    NoVideoMode,
    #[error("monitor scale factor must be positive and finite, got {0}")]
    InvalidScale(f64),
}

pub(crate) fn create_window(
    event_loop: &dyn ActiveEventLoop,
    config: &WindowConfig,
) -> Result<Arc<dyn Window>, WindowError> {
    let attributes = attributes(config);
    let attributes = match placement(event_loop, config) {
        Some(placement) => attributes.with_position(placement),
        None => attributes,
    };
    event_loop
        .create_window(attributes)
        .map(Arc::from)
        .map_err(WindowError::window)
}

fn placement(event_loop: &dyn ActiveEventLoop, config: &WindowConfig) -> Option<Position> {
    if let Some(position) = config.position {
        return Some(Position::Logical(LogicalPosition::new(
            position.x, position.y,
        )));
    }
    if !config.centered {
        return None;
    }
    match center_on_monitor(event_loop, config) {
        Ok(position) => Some(position),
        Err(error) => {
            warn!(%error, "window cannot be centered");
            None
        }
    }
}

fn center_on_monitor(
    event_loop: &dyn ActiveEventLoop,
    config: &WindowConfig,
) -> Result<Position, PlacementError> {
    let monitor = event_loop
        .primary_monitor()
        .ok_or(PlacementError::NoMonitor)?;
    let origin = monitor.position().ok_or(PlacementError::NoOrigin)?;
    let area = monitor
        .current_video_mode()
        .ok_or(PlacementError::NoVideoMode)?
        .size();
    let scale = monitor.scale_factor();
    if !scale.is_finite() || scale <= 0.0 {
        return Err(PlacementError::InvalidScale(scale));
    }
    let width = (f64::from(config.width) * scale).round() as i32;
    let height = (f64::from(config.height) * scale).round() as i32;
    Ok(Position::Physical(centered_position(
        origin,
        area,
        (width, height),
    )))
}

pub(super) fn centered_position(
    origin: PhysicalPosition<i32>,
    area: PhysicalSize<u32>,
    window: (i32, i32),
) -> PhysicalPosition<i32> {
    let x = origin.x + (area.width as i32 - window.0) / 2;
    let y = origin.y + (area.height as i32 - window.1) / 2;
    PhysicalPosition::new(x, y)
}

fn winit_transparent(config: &WindowConfig) -> bool {
    #[cfg(windows)]
    {
        config.transparent && config.backend != Backend::Software
    }
    #[cfg(not(windows))]
    {
        config.transparent
    }
}

fn window_icon(bytes: &[u8]) -> Option<Icon> {
    let texture = match Image::from_bytes(bytes).ok()? {
        Image::Raster(texture) => texture,
        Image::Vector(vector) => vector.raster(32, 32, Color::WHITE)?,
    };
    RgbaIcon::new(
        texture.rgba().unwrap_or(&[]).to_vec(),
        texture.width(),
        texture.height(),
    )
    .ok()
    .map(Icon::from)
}

pub(crate) fn attributes(config: &WindowConfig) -> WindowAttributes {
    let size = LogicalSize::new(f64::from(config.width), f64::from(config.height));
    let mut attributes = WindowAttributes::default()
        .with_title(config.title.clone())
        .with_surface_size(size)
        .with_resizable(!config.frameless)
        .with_decorations(!config.frameless)
        .with_transparent(winit_transparent(config))
        .with_visible(false)
        .with_window_icon(config.icon.as_deref().and_then(window_icon));
    if config.frameless {
        attributes = attributes
            .with_min_surface_size(size)
            .with_max_surface_size(size);
    }
    attributes
}

pub(super) struct App {
    pub(super) config: WindowConfig,
    pub(super) window: Option<Arc<dyn Window>>,
    pub(super) renderer: Option<Renderer>,
    pub(super) error: Rc<Cell<Option<WindowError>>>,
    pub(super) revealed: bool,
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
        let renderer = match create_renderer(self.config.backend, &window, self.config.transparent)
        {
            Ok(renderer) => renderer,
            Err(error) => return self.fail(event_loop, error),
        };

        self.window = Some(Arc::clone(&window));
        self.renderer = Some(renderer);
        self.render(&window);
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
        error!(%error, "window failed");
        self.error.set(Some(error));
        event_loop.exit();
    }

    fn resize(&mut self, width: u32, height: u32) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        renderer.resize(width, height);
    }

    fn render(&mut self, window: &Arc<dyn Window>) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let size = window.surface_size();
        renderer.resize(size.width, size.height);
        if !renderer.clear(window) {
            window.request_redraw();
            return;
        }
        if !self.revealed {
            self.revealed = true;
            window.set_visible(self.config.visible);
        }
    }
}
