#[path = "parts/window_app.rs"]
mod app;
#[path = "parts/pointer.rs"]
mod pointer;
use app::App;
pub(crate) use app::create_window;
#[cfg(test)]
use app::{MonitorRect, attributes, centered_position, monitor_at_point};

#[cfg(test)]
#[path = "parts/window_tests.rs"]
mod tests;

use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::PoisonError;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tracing::error;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Position};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::icon::{Icon, RgbaIcon};
use winit::window::{Window, WindowAttributes, WindowId};

use brul_gpu::effect::Effects;
use brul_gui::{Color, DispatchEvent, Image, WidgetTree};
use brul_ident::{FunctionName, TimerId, TimerKind, TimerOwner};

use crate::input::{Input, event_input, window_input};
use crate::platform::PlatformSpec;
use crate::surface::{Renderer, create_renderer};

pub(crate) const CLEAR: wgpu::Color = wgpu::Color {
    r: 0.08,
    g: 0.09,
    b: 0.11,
    a: 0.85,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowPosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Backend {
    #[default]
    Software,
    Gpu,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown backend: {0}")]
pub struct UnknownBackend(pub String);

impl FromStr for Backend {
    type Err = UnknownBackend;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "software" => Ok(Self::Software),
            "gpu" => Ok(Self::Gpu),
            other => Err(UnknownBackend(other.to_owned())),
        }
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Software => f.write_str("software"),
            Self::Gpu => f.write_str("gpu"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub header_height: u32,
    pub frameless: bool,
    pub transparent: bool,
    pub visible: bool,
    pub centered: bool,
    pub position: Option<WindowPosition>,
    pub scroll: bool,
    pub icon: Option<Arc<[u8]>>,
    pub backend: Backend,
    pub effects: Effects,
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
            visible: true,
            centered: false,
            position: None,
            scroll: true,
            icon: None,
            backend: Backend::Software,
            effects: Effects::default(),
        }
    }
}

impl WindowConfig {
    pub fn with_effects(mut self, effects: Effects) -> Self {
        self.effects = effects;
        self
    }

    pub fn validate(&self) -> Result<(), WindowError> {
        if self.width == 0 || self.height == 0 {
            return Err(WindowError::EmptySize);
        }
        if self.header_height > self.height {
            return Err(WindowError::HeaderTooTall);
        }
        if self
            .position
            .is_some_and(|position| !position.x.is_finite() || !position.y.is_finite())
        {
            return Err(WindowError::InvalidPosition);
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopedCommand {
    pub window: WindowKey,
    pub command: WindowCommand,
}

/// Wakes the event loop from another thread: [`Waker::wake`] is safe after the
/// loop exits and then does nothing. [`Waker::command`] addresses a command to
/// one window; a command with an unknown key is dropped by the loop.
#[derive(Clone, Debug)]
pub struct Waker {
    proxy: EventLoopProxy,
    commands: Arc<Mutex<Vec<ScopedCommand>>>,
}

impl Waker {
    pub(crate) fn new(proxy: EventLoopProxy) -> Self {
        Self {
            proxy,
            commands: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn wake(&self) {
        self.proxy.wake_up();
    }

    pub fn command(&self, window: WindowKey, command: WindowCommand) {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(ScopedCommand { window, command });
        self.wake();
    }

    pub(crate) fn take_commands(&self) -> Vec<ScopedCommand> {
        let mut commands = self.commands.lock().unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut *commands)
    }
}

impl std::fmt::Display for WindowKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "window {}", self.0)
    }
}

pub struct WindowSpec {
    pub config: WindowConfig,
    pub tree: Rc<RefCell<WidgetTree>>,
    pub commands: Vec<WindowCommand>,
    pub platform: Option<PlatformSpec>,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WindowError {
    #[error("window size must be greater than zero")]
    EmptySize,
    #[error("header is taller than the window")]
    HeaderTooTall,
    #[error("window position must be finite")]
    InvalidPosition,
    #[error("no suitable surface configuration")]
    NoSurfaceConfig,
    #[error("window has no content")]
    NoContent,
    #[error("{0} is not declared")]
    Undeclared(WindowKey),
    #[error("event loop: {0}")]
    EventLoop(#[source] Box<dyn std::error::Error>),
    #[error("window creation: {0}")]
    Window(#[source] Box<dyn std::error::Error>),
    #[error("gpu: {0}")]
    Gpu(#[source] Box<dyn std::error::Error>),
    #[error("widgets: {0}")]
    Widgets(#[from] brul_gui::WidgetError),
    #[error("{0}")]
    App(#[source] Box<dyn std::error::Error>),
}

impl WindowError {
    pub fn app(error: impl std::error::Error + 'static) -> Self {
        Self::App(Box::new(error))
    }

    pub(crate) fn event_loop(error: impl std::error::Error + 'static) -> Self {
        Self::EventLoop(Box::new(error))
    }

    pub(crate) fn window(error: impl std::error::Error + 'static) -> Self {
        Self::Window(Box::new(error))
    }

    pub(crate) fn gpu(error: impl std::error::Error + 'static) -> Self {
        Self::Gpu(Box::new(error))
    }
}

pub fn run(config: WindowConfig) -> Result<(), WindowError> {
    crate::perf::start();
    config.validate()?;
    let event_loop = EventLoop::new().map_err(WindowError::event_loop)?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let error = Rc::new(Cell::new(None));
    let app = App {
        config,
        window: None,
        renderer: None,
        error: Rc::clone(&error),
        revealed: false,
    };
    let result = event_loop.run_app(app);

    if let Some(error) = error.take() {
        return Err(error);
    }
    result.map_err(WindowError::event_loop)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerCommand {
    Start {
        id: TimerId,
        owner: TimerOwner,
        function: FunctionName,
        interval: Duration,
        kind: TimerKind,
    },
    Cancel {
        id: TimerId,
        owner: TimerOwner,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowCommand {
    Exit,
    Close,
    Open(WindowKey),
    Drag,
    Show,
    Hide,
    Focus,
    Timer(TimerCommand),
    SetTheme(String),
    AlwaysOnTop(bool),
}

pub fn run_tree(
    config: WindowConfig,
    tree: Rc<RefCell<WidgetTree>>,
    initial: Vec<WindowCommand>,
    handler: impl FnMut(&[DispatchEvent]) -> Vec<WindowCommand> + 'static,
) -> Result<(), WindowError> {
    run_tree_themed(config, initial, move |_| Rc::clone(&tree), handler)
}

pub fn run_tree_themed(
    config: WindowConfig,
    initial: Vec<WindowCommand>,
    mut build: impl FnMut(ColorScheme) -> Rc<RefCell<WidgetTree>> + 'static,
    handler: impl FnMut(&[DispatchEvent]) -> Vec<WindowCommand> + 'static,
) -> Result<(), WindowError> {
    crate::perf::start();
    config.validate()?;
    let event_loop = EventLoop::new().map_err(WindowError::event_loop)?;
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
        None,
    );
    let result = event_loop.run_app(app);

    if let Some(error) = error.take() {
        return Err(error);
    }
    result.map_err(WindowError::event_loop)
}

pub fn run_windows(
    initial: WindowKey,
    create: impl FnMut(WindowKey, ColorScheme) -> Result<WindowSpec, WindowError> + 'static,
    handler: impl FnMut(WindowKey, &[DispatchEvent]) -> Vec<WindowCommand> + 'static,
    closed: impl FnMut(WindowKey) + 'static,
    ready: impl FnOnce(Waker) + 'static,
    wake: impl FnMut() + 'static,
) -> Result<(), WindowError> {
    crate::perf::start();
    crate::windows::run(initial, create, handler, closed, ready, wake)
}
