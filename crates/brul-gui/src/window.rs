use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use brul_reactive;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::gpu::Gpu;
use crate::input::{Input, event_input, point, system_scheme, window_input};
use crate::layout::Size;
use crate::paint::Frame;
use crate::text::TextEngine;
use crate::widget::{WidgetEvent, WidgetTree};

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
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: String::from("BRUL"),
            width: 960,
            height: 640,
            header_height: 48,
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

    fn is_header(&self, y: f64, scale: f64) -> bool {
        y >= 0.0 && y < f64::from(self.header_height) * scale
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorScheme {
    Light,
    Dark,
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

    let error = Rc::new(RefCell::new(None));
    let app = App {
        config,
        window: None,
        gpu: None,
        error: Rc::clone(&error),
    };
    let result = event_loop.run_app(app);

    if let Some(error) = error.borrow_mut().take() {
        return Err(error);
    }
    result.map_err(|error| WindowError::stage("event loop", error))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowCommand {
    Exit,
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

    let error = Rc::new(RefCell::new(None));
    let tree = build(ColorScheme::Dark);
    let mut app = TreeApp {
        config,
        tree,
        build: Box::new(build),
        handler: Box::new(handler),
        window: None,
        gpu: None,
        engine: None,
        frame: None,
        error: Rc::clone(&error),
        timers: Vec::new(),
    };
    for command in initial {
        if let WindowCommand::Every { interval, function } = command {
            app.timers.push(Timer {
                next: Instant::now() + interval,
                interval,
                function,
            });
        }
    }
    let result = event_loop.run_app(app);

    if let Some(error) = error.borrow_mut().take() {
        return Err(error);
    }
    result.map_err(|error| WindowError::stage("event loop", error))
}

fn create_window(
    event_loop: &dyn ActiveEventLoop,
    config: &WindowConfig,
) -> Result<Arc<dyn Window>, WindowError> {
    let size = LogicalSize::new(f64::from(config.width), f64::from(config.height));
    let attributes = WindowAttributes::default()
        .with_title(config.title.clone())
        .with_surface_size(size)
        .with_min_surface_size(size)
        .with_max_surface_size(size)
        .with_resizable(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_visible(false);

    event_loop
        .create_window(attributes)
        .map(Arc::from)
        .map_err(|error| WindowError::stage("window creation", error))
}

struct App {
    config: WindowConfig,
    window: Option<Arc<dyn Window>>,
    gpu: Option<Gpu>,
    error: Rc<RefCell<Option<WindowError>>>,
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
            Some(Input::Key { exit: true, .. }) => event_loop.exit(),
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

type EventHandler = Box<dyn FnMut(&[WidgetEvent]) -> Vec<WindowCommand>>;
type TreeBuilder = Box<dyn FnMut(ColorScheme) -> Rc<RefCell<WidgetTree>>>;

struct Timer {
    next: Instant,
    interval: Duration,
    function: String,
}

impl Timer {
    fn new(interval: Duration, function: String) -> Self {
        Self {
            next: Instant::now() + interval,
            interval,
            function,
        }
    }
}

fn due(timers: &mut [Timer], now: Instant) -> Vec<WidgetEvent> {
    let mut events = Vec::new();
    for timer in timers {
        if timer.next > now {
            continue;
        }
        events.push(WidgetEvent::Timer(timer.function.clone()));
        while timer.next <= now {
            timer.next += timer.interval;
        }
    }
    events
}

fn wakeup(timers: &[Timer]) -> ControlFlow {
    match timers.iter().map(|timer| timer.next).min() {
        Some(next) => ControlFlow::WaitUntil(next),
        None => ControlFlow::Wait,
    }
}

trait Surface {
    fn scale_factor(&self) -> f64;
    fn drag(&self);
    fn redraw(&self);
}

impl Surface for Arc<dyn Window> {
    fn scale_factor(&self) -> f64 {
        Window::scale_factor(self.as_ref())
    }

    fn drag(&self) {
        let _ = self.as_ref().drag_window();
    }

    fn redraw(&self) {
        self.as_ref().request_redraw();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Continue,
    Exit,
}

struct TreeApp {
    config: WindowConfig,
    tree: Rc<RefCell<WidgetTree>>,
    build: TreeBuilder,
    handler: EventHandler,
    window: Option<Arc<dyn Window>>,
    gpu: Option<Gpu>,
    engine: Option<TextEngine>,
    frame: Option<Frame>,
    error: Rc<RefCell<Option<WindowError>>>,
    timers: Vec<Timer>,
}

impl ApplicationHandler for TreeApp {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = match create_window(event_loop, &self.config) {
            Ok(window) => window,
            Err(error) => {
                let step = self.fail(error);
                return self.finish(event_loop, step);
            }
        };
        let gpu = match Gpu::new(&window) {
            Ok(gpu) => gpu,
            Err(error) => {
                let step = self.fail(error);
                return self.finish(event_loop, step);
            }
        };
        let scheme = system_scheme(event_loop.system_theme().as_ref());
        self.tree = (self.build)(scheme);
        let mut engine = TextEngine::new();
        let size = window.surface_size();
        let viewport = Size {
            width: size.width,
            height: size.height,
        };
        let layout =
            self.tree
                .borrow_mut()
                .layout(&mut engine, viewport, window.scale_factor() as f32);
        if let Err(error) = layout {
            let step = self.fail(WindowError::stage("widgets", error));
            return self.finish(event_loop, step);
        }

        self.window = Some(Arc::clone(&window));
        self.gpu = Some(gpu);
        self.engine = Some(engine);
        self.frame = Some(Frame::new(size.width.max(1), size.height.max(1)));
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
            Some(Input::Key { code, exit }) => {
                let step = self.dispatch_events(&window, vec![WidgetEvent::Key(code)]);
                self.finish(event_loop, step);
                if exit {
                    event_loop.exit();
                }
            }
            Some(Input::Moved(x, y)) => {
                let (x, y) = point(x, y);
                let events = self.tree.borrow_mut().pointer_moved(x, y);
                let step = self.dispatch(&window, events);
                self.finish(event_loop, step);
            }
            Some(Input::Left) => {
                let events = self.tree.borrow_mut().pointer_leave();
                let step = self.dispatch_events(&window, events);
                self.finish(event_loop, step);
            }
            Some(Input::Down(x, y)) => {
                if self.config.is_header(y, window.scale_factor()) {
                    let _ = window.drag_window();
                    return;
                }
                let (x, y) = point(x, y);
                let events = self.tree.borrow_mut().pointer_down(x, y);
                let step = self.dispatch(&window, events);
                self.finish(event_loop, step);
            }
            Some(Input::Up(x, y)) => {
                let (x, y) = point(x, y);
                let events = self.tree.borrow_mut().pointer_up(x, y);
                let step = self.dispatch(&window, events);
                self.finish(event_loop, step);
            }
            Some(Input::Theme(scheme)) => {
                self.tree = (self.build)(scheme);
                let size = window.surface_size();
                let step = self.resize(&window, size.width, size.height);
                self.finish(event_loop, step);
            }
            Some(Input::Surface) => {
                let size = window.surface_size();
                let step = self.resize(&window, size.width, size.height);
                self.finish(event_loop, step);
            }
            Some(Input::Redraw) => self.render(event_loop, &window),
            None => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        brul_reactive::flush();
        if self.tree.borrow().snapshot_dirty()
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }

        let events = due(&mut self.timers, Instant::now());
        if !events.is_empty()
            && let Some(window) = self.window.clone()
        {
            let step = self.dispatch_events(&window, events);
            self.finish(event_loop, step);
        }

        event_loop.set_control_flow(wakeup(&self.timers));
    }
}

impl TreeApp {
    fn fail(&mut self, error: WindowError) -> Step {
        *self.error.borrow_mut() = Some(error);
        Step::Exit
    }

    fn finish(&mut self, event_loop: &dyn ActiveEventLoop, step: Step) {
        if step == Step::Exit {
            event_loop.exit();
        }
    }

    fn dispatch(
        &mut self,
        surface: &dyn Surface,
        events: Result<Vec<WidgetEvent>, crate::WidgetError>,
    ) -> Step {
        match events {
            Ok(events) => self.dispatch_events(surface, events),
            Err(error) => self.fail(WindowError::stage("widgets", error)),
        }
    }

    fn dispatch_events(&mut self, surface: &dyn Surface, events: Vec<WidgetEvent>) -> Step {
        if events.is_empty() {
            return Step::Continue;
        }
        let drag = events
            .iter()
            .any(|event| matches!(event, WidgetEvent::Drag(_)));
        if drag {
            surface.drag();
        }
        let commands = (self.handler)(&events);
        let step = self.run_commands(surface, commands);
        brul_reactive::flush();
        if self.tree.borrow().snapshot_dirty() {
            surface.redraw();
        }
        step
    }

    fn run_commands(&mut self, surface: &dyn Surface, commands: Vec<WindowCommand>) -> Step {
        for command in commands {
            match command {
                WindowCommand::Exit => return Step::Exit,
                WindowCommand::Drag => surface.drag(),
                WindowCommand::Every { interval, function } => {
                    self.timers.push(Timer::new(interval, function));
                }
                WindowCommand::SetTheme(_) => {}
            }
        }
        Step::Continue
    }

    fn resize(&mut self, surface: &dyn Surface, width: u32, height: u32) -> Step {
        if width == 0 || height == 0 {
            return Step::Continue;
        }
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.resize(width, height);
        }
        let layout = match self.engine.as_mut() {
            Some(engine) => self.tree.borrow_mut().layout(
                engine,
                Size { width, height },
                surface.scale_factor() as f32,
            ),
            None => Ok(()),
        };
        if let Err(error) = layout {
            return self.fail(WindowError::stage("widgets", error));
        }
        if let Some(frame) = self.frame.as_mut() {
            frame.resize(width, height);
        } else {
            self.frame = Some(Frame::new(width, height));
        }
        surface.redraw();
        Step::Continue
    }

    fn render(&mut self, event_loop: &dyn ActiveEventLoop, window: &Arc<dyn Window>) {
        brul_reactive::flush();
        if let Err(error) = self.render_frame(window) {
            let step = self.fail(WindowError::stage("widgets", error));
            self.finish(event_loop, step);
        }
    }

    fn render_frame(&mut self, window: &Arc<dyn Window>) -> Result<(), crate::WidgetError> {
        if self.tree.borrow().layout_dirty()
            && let Some(engine) = self.engine.as_mut()
        {
            self.tree.borrow_mut().relayout(engine)?;
        }
        let (dirty, rebuild) = {
            let mut tree = self.tree.borrow_mut();
            (tree.take_dirty(), tree.snapshot_dirty())
        };
        let Some(gpu) = self.gpu.as_mut() else {
            return Ok(());
        };
        let Some(frame) = self.frame.as_mut() else {
            return Ok(());
        };
        let Some(engine) = self.engine.as_ref() else {
            return Ok(());
        };

        if rebuild || !dirty.is_empty() {
            let mut tree = self.tree.borrow_mut();
            let snapshot = tree.snapshot()?;
            frame.paint(snapshot, engine.atlas(), &dirty);
        }

        gpu.present_frame(window, frame);
        Ok(())
    }
}

impl App {
    fn fail(&mut self, event_loop: &dyn ActiveEventLoop, error: WindowError) {
        *self.error.borrow_mut() = Some(error);
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
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use winit::event_loop::ControlFlow;

    use super::WindowCommand;
    use super::{
        EventHandler, Step, Surface, Timer, TreeApp, WindowConfig, WindowError, due, wakeup,
    };
    use crate::widget::{Widget, WidgetEvent, WidgetTree};
    use crate::{TextEngine, WidgetError};

    struct FakeSurface {
        scale: f64,
        drags: Cell<usize>,
        redraws: Cell<usize>,
    }

    impl FakeSurface {
        fn new(scale: f64) -> Self {
            Self {
                scale,
                drags: Cell::new(0),
                redraws: Cell::new(0),
            }
        }
    }

    impl Surface for FakeSurface {
        fn scale_factor(&self) -> f64 {
            self.scale
        }

        fn drag(&self) {
            self.drags.set(self.drags.get() + 1);
        }

        fn redraw(&self) {
            self.redraws.set(self.redraws.get() + 1);
        }
    }

    fn tree_app(handler: EventHandler) -> TreeApp {
        TreeApp {
            config: WindowConfig::default(),
            tree: Rc::new(RefCell::new(WidgetTree::new(Widget::text("")))),
            build: Box::new(|_| Rc::new(RefCell::new(WidgetTree::new(Widget::text(""))))),
            handler,
            window: None,
            gpu: None,
            engine: None,
            frame: None,
            error: Rc::new(RefCell::new(None)),
            timers: Vec::new(),
        }
    }

    fn commands(commands: Vec<WindowCommand>) -> EventHandler {
        Box::new(move |_| commands.clone())
    }

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

    #[test]
    fn a_new_timer_waits_for_its_interval() {
        let timer = Timer::new(Duration::from_secs(5), "later".to_owned());
        assert!(timer.next > Instant::now());
        assert_eq!(timer.interval, Duration::from_secs(5));
    }

    #[test]
    fn a_due_timer_fires_once_and_moves_past_now() {
        let now = Instant::now();
        let mut timers = vec![Timer {
            next: now,
            interval: Duration::from_millis(10),
            function: "tick".to_owned(),
        }];

        let events = due(&mut timers, now);

        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], WidgetEvent::Timer(name) if name == "tick"));
        assert!(timers[0].next > now);
        assert!(due(&mut timers, now).is_empty());
    }

    #[test]
    fn a_timer_in_the_future_stays_silent() {
        let now = Instant::now();
        let mut timers = vec![Timer {
            next: now + Duration::from_secs(1),
            interval: Duration::from_secs(1),
            function: "late".to_owned(),
        }];
        assert!(due(&mut timers, now).is_empty());
    }

    #[test]
    fn the_loop_waits_for_the_earliest_timer() {
        let now = Instant::now();
        assert_eq!(wakeup(&[]), ControlFlow::Wait);

        let mut timers = vec![
            Timer {
                next: now + Duration::from_secs(2),
                interval: Duration::from_secs(1),
                function: "far".to_owned(),
            },
            Timer {
                next: now + Duration::from_secs(1),
                interval: Duration::from_secs(1),
                function: "near".to_owned(),
            },
        ];
        assert_eq!(
            wakeup(&timers),
            ControlFlow::WaitUntil(now + Duration::from_secs(1))
        );

        timers[0].next = now + Duration::from_secs(1);
        assert_eq!(
            wakeup(&timers),
            ControlFlow::WaitUntil(now + Duration::from_secs(1))
        );
    }

    #[test]
    fn an_empty_event_list_changes_nothing() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(Vec::new()));

        assert_eq!(app.dispatch_events(&surface, Vec::new()), Step::Continue);
        assert_eq!(surface.drags.get(), 0);
        assert_eq!(surface.redraws.get(), 0);
    }

    #[test]
    fn a_drag_event_drags_the_window() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(Vec::new()));
        let widget = Widget::text("x").id();

        let step = app.dispatch_events(&surface, vec![WidgetEvent::Drag(widget)]);
        assert_eq!(step, Step::Continue);
        assert_eq!(surface.drags.get(), 1);
    }

    #[test]
    fn the_exit_command_finishes_the_loop() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(vec![WindowCommand::Exit]));
        let widget = Widget::text("x").id();

        let step = app.dispatch_events(&surface, vec![WidgetEvent::Press(widget)]);
        assert_eq!(step, Step::Exit);
    }

    #[test]
    fn a_drag_command_drags_the_window() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(vec![WindowCommand::Drag]));
        let widget = Widget::text("x").id();

        app.dispatch_events(&surface, vec![WidgetEvent::Press(widget)]);
        assert_eq!(surface.drags.get(), 1);
    }

    #[test]
    fn a_timer_command_is_kept() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(vec![WindowCommand::Every {
            interval: Duration::from_millis(100),
            function: "tick".to_owned(),
        }]));
        let widget = Widget::text("x").id();

        app.dispatch_events(&surface, vec![WidgetEvent::Press(widget)]);
        assert_eq!(app.timers.len(), 1);
    }

    #[test]
    fn a_size_of_zero_is_ignored() {
        let surface = FakeSurface::new(2.0);
        let mut app = tree_app(commands(Vec::new()));

        assert_eq!(app.resize(&surface, 0, 10), Step::Continue);
        assert_eq!(surface.redraws.get(), 0);
        assert!(app.frame.is_none());
    }

    #[test]
    fn a_resize_makes_the_frame_and_asks_for_a_redraw() {
        let surface = FakeSurface::new(2.0);
        let mut app = tree_app(commands(Vec::new()));

        assert_eq!(app.resize(&surface, 4, 5), Step::Continue);
        assert!(app.frame.is_some());
        assert_eq!(surface.redraws.get(), 1);
    }

    #[test]
    fn an_engine_lays_the_tree_out_on_a_resize() {
        let surface = FakeSurface::new(2.0);
        let mut app = tree_app(commands(Vec::new()));
        app.engine = Some(TextEngine::new());

        assert_eq!(app.resize(&surface, 4, 5), Step::Continue);
        assert_eq!(surface.redraws.get(), 1);
        assert!(!app.tree.borrow().layout_dirty());
    }

    #[test]
    fn an_invalid_scale_ends_the_loop() {
        let surface = FakeSurface::new(0.0);
        let mut app = tree_app(commands(Vec::new()));
        app.engine = Some(TextEngine::new());

        assert_eq!(app.resize(&surface, 4, 5), Step::Exit);
        assert_eq!(surface.redraws.get(), 0);
        assert!(app.error.borrow().is_some());
    }
    #[test]
    fn a_failing_widget_event_is_reported_and_stops_the_loop() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(Vec::new()));

        let step = app.dispatch(&surface, Err(WidgetError::NoLayout));
        assert_eq!(step, Step::Exit);
        assert!(app.error.borrow().is_some());
    }
}
