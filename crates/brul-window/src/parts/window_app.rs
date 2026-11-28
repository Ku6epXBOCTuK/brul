use super::pointer::pointer_position;
use super::*;
use tracing::warn;
use winit::monitor::MonitorHandle;

#[derive(Debug, thiserror::Error)]
pub(super) enum PlacementError {
    #[error("no monitor is available")]
    NoMonitor,
    #[error("monitor origin is unavailable")]
    NoOrigin,
    #[error("monitor video mode is unavailable")]
    NoVideoMode,
    #[error("monitor scale factor must be positive and finite, got {0}")]
    InvalidScale(f64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct MonitorRect {
    pub(super) origin: PhysicalPosition<i32>,
    pub(super) size: PhysicalSize<u32>,
}

impl TryFrom<&MonitorHandle> for MonitorRect {
    type Error = PlacementError;

    fn try_from(monitor: &MonitorHandle) -> Result<Self, Self::Error> {
        Ok(Self {
            origin: monitor.position().ok_or(PlacementError::NoOrigin)?,
            size: monitor
                .current_video_mode()
                .ok_or(PlacementError::NoVideoMode)?
                .size(),
        })
    }
}

impl MonitorRect {
    fn contains(&self, point: PhysicalPosition<i32>) -> bool {
        let (left, right, top, bottom) = self.bounds();
        let x = i64::from(point.x);
        let y = i64::from(point.y);
        x >= left && x < right && y >= top && y < bottom
    }

    fn distance(&self, point: PhysicalPosition<i32>) -> i64 {
        let (left, right, top, bottom) = self.bounds();
        let x = i64::from(point.x);
        let y = i64::from(point.y);
        let dx = (left - x).max(x - right).max(0);
        let dy = (top - y).max(y - bottom).max(0);
        dx * dx + dy * dy
    }

    fn bounds(&self) -> (i64, i64, i64, i64) {
        let left = i64::from(self.origin.x);
        let top = i64::from(self.origin.y);
        (
            left,
            left + i64::from(self.size.width),
            top,
            top + i64::from(self.size.height),
        )
    }
}

pub(super) fn monitor_at_point<T>(
    monitors: impl Iterator<Item = (T, MonitorRect)>,
    point: PhysicalPosition<i32>,
) -> Option<T> {
    monitors
        .min_by_key(|(_, rect)| (!rect.contains(point), rect.distance(point)))
        .map(|(monitor, _)| monitor)
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
    let candidates = event_loop.available_monitors().filter_map(|monitor| {
        MonitorRect::try_from(&monitor)
            .ok()
            .map(|rect| (monitor, rect))
    });
    let monitor = pointer_position()
        .and_then(|point| monitor_at_point(candidates, point))
        .or_else(|| event_loop.primary_monitor())
        .or_else(|| event_loop.available_monitors().next())
        .ok_or(PlacementError::NoMonitor)?;
    let rect = MonitorRect::try_from(&monitor)?;
    let scale = monitor.scale_factor();
    if !scale.is_finite() || scale <= 0.0 {
        return Err(PlacementError::InvalidScale(scale));
    }
    let window =
        LogicalSize::new(f64::from(config.width), f64::from(config.height)).to_physical(scale);
    Ok(Position::Physical(centered_position(
        rect.origin,
        rect.size,
        window,
    )))
}

pub(super) fn centered_position(
    origin: PhysicalPosition<i32>,
    area: PhysicalSize<u32>,
    window: PhysicalSize<u32>,
) -> PhysicalPosition<i32> {
    let x = origin.x + (area.width as i32 - window.width as i32) / 2;
    let y = origin.y + (area.height as i32 - window.height as i32) / 2;
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
