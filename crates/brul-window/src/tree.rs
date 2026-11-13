use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use brul_gui::{Frame, Size, TextEngine, WidgetEvent, WidgetTree};
use tracing::{debug, error, info, trace};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

use crate::gpu::Gpu;
use crate::input::{Input, event_input, point, system_scheme, window_input};
use crate::surface::{Renderer, Surface};
use crate::window::{
    ColorScheme, WindowCommand, WindowConfig, WindowError, WindowKey, create_window,
};

type EventHandler = Box<dyn FnMut(&[WidgetEvent]) -> Vec<WindowCommand>>;
type TreeBuilder = Box<dyn FnMut(ColorScheme) -> Rc<RefCell<WidgetTree>>>;

const FRAME_INTERVAL: Duration = Duration::from_millis(16);

pub(crate) struct Timer {
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

pub(crate) fn due(timers: &mut [Timer], now: Instant) -> Vec<WidgetEvent> {
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

#[cfg(test)]
fn wakeup(timers: &[Timer], frame: Option<Instant>) -> ControlFlow {
    let timer = timers.iter().map(|timer| timer.next).min();
    wakeup_at(earliest(timer, frame))
}

pub(crate) fn wakeup_at(next: Option<Instant>) -> ControlFlow {
    match next {
        Some(next) => ControlFlow::WaitUntil(next),
        None => ControlFlow::Wait,
    }
}

fn earliest(first: Option<Instant>, second: Option<Instant>) -> Option<Instant> {
    match (first, second) {
        (Some(first), Some(second)) => Some(first.min(second)),
        (first, second) => first.or(second),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Continue,
    Close,
    Open(WindowKey),
    Exit,
}

pub(crate) struct TreeApp {
    config: WindowConfig,
    tree: Rc<RefCell<WidgetTree>>,
    build: Option<TreeBuilder>,
    handler: EventHandler,
    window: Option<Arc<dyn Window>>,
    renderer: Option<Box<dyn Renderer>>,
    engine: Option<TextEngine>,
    frame: Option<Frame>,
    error: Rc<Cell<Option<WindowError>>>,
    pub(crate) timers: Vec<Timer>,
    cursor: (i32, i32),
    tick: Instant,
    next_frame: Instant,
    clipboard: Option<String>,
    paste: bool,
}

impl ApplicationHandler for TreeApp {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let step = self.open(event_loop, None);
        self.finish(event_loop, step);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let step = self.event(window_id, &event);
        self.finish(event_loop, step);
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        let step = self.pump(Instant::now());
        self.finish(event_loop, step);
        event_loop.set_control_flow(wakeup_at(self.deadline()));
    }
}

pub(crate) fn tree_app(
    config: WindowConfig,
    tree: Rc<RefCell<WidgetTree>>,
    build: Option<TreeBuilder>,
    handler: EventHandler,
    error: Rc<Cell<Option<WindowError>>>,
    initial: Vec<WindowCommand>,
) -> TreeApp {
    let mut app = TreeApp {
        config,
        tree,
        build,
        handler,
        window: None,
        renderer: None,
        engine: None,
        frame: None,
        error,
        timers: Vec::new(),
        cursor: (0, 0),
        tick: Instant::now(),
        next_frame: Instant::now(),
        clipboard: None,
        paste: false,
    };
    for command in initial {
        if let WindowCommand::Every { interval, function } = command {
            app.timers.push(Timer::new(interval, function));
        }
    }
    app
}
impl TreeApp {
    pub(crate) fn open(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        tree: Option<Rc<RefCell<WidgetTree>>>,
    ) -> Step {
        if self.window.is_some() {
            return Step::Continue;
        }

        let window = match create_window(event_loop, &self.config) {
            Ok(window) => window,
            Err(error) => return self.fail(error),
        };
        let gpu = match Gpu::new(&window) {
            Ok(gpu) => gpu,
            Err(error) => return self.fail(error),
        };
        self.tree = match tree {
            Some(tree) => tree,
            None => match self.build.as_mut() {
                Some(build) => {
                    let scheme = system_scheme(event_loop.system_theme().as_ref());
                    build(scheme)
                }
                None => {
                    return self.fail(WindowError::Stage {
                        stage: "window",
                        message: String::from("window has no content"),
                    });
                }
            },
        };
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
            return self.fail(WindowError::stage("widgets", error));
        }

        self.window = Some(Arc::clone(&window));
        self.renderer = Some(Box::new(gpu));
        self.engine = Some(engine);
        self.frame = Some(Frame::new(size.width.max(1), size.height.max(1)));
        info!(title = %self.config.title, width = size.width, height = size.height, "window opened");
        window.set_visible(true);
        window.request_redraw();
        Step::Continue
    }

    pub(crate) fn event(&mut self, window_id: WindowId, event: &WindowEvent) -> Step {
        let Some(window) = self
            .window
            .as_ref()
            .filter(|window| window.id() == window_id)
        else {
            return Step::Continue;
        };
        let window = Arc::clone(window);

        let input = window_input(event).or_else(|| event_input(event));
        match &input {
            Some(Input::Moved(..)) | Some(Input::Redraw) => trace!(?input, "window input"),
            Some(_) => debug!(?input, "window input"),
            None => {}
        }
        match input {
            Some(Input::Exit) => Step::Close,
            Some(Input::Redraw) => self.render(&window),
            Some(input) => self.handle(&window, input),
            None => Step::Continue,
        }
    }

    pub(crate) fn pump(&mut self, now: Instant) -> Step {
        brul_reactive::flush();
        if self.tree.borrow().snapshot_dirty()
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }

        let mut step = Step::Continue;
        let events = due(&mut self.timers, now);
        if !events.is_empty()
            && let Some(window) = self.window.clone()
        {
            step = self.dispatch_events(&window, events);
        }

        if self.tree.borrow().animating() && now >= self.next_frame {
            self.next_frame = now + FRAME_INTERVAL;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        step
    }

    pub(crate) fn deadline(&self) -> Option<Instant> {
        let timer = self.timers.iter().map(|timer| timer.next).min();
        let frame = if self.tree.borrow().animating() {
            Some(self.next_frame)
        } else {
            None
        };
        earliest(timer, frame)
    }

    pub(crate) fn window_id(&self) -> Option<WindowId> {
        self.window.as_ref().map(|window| window.id())
    }

    pub(crate) fn focus(&self) {
        if let Some(window) = &self.window {
            window.focus_window();
        }
    }

    fn handle(&mut self, surface: &dyn Surface, input: Input) -> Step {
        if self.engine.is_some() && !self.tree.borrow().has_layout() {
            let step = self.resize_to_surface(surface);
            if !matches!(step, Step::Continue) {
                return step;
            }
            if !self.tree.borrow().has_layout() {
                debug!(?input, "input dropped until the tree is laid out");
                return Step::Continue;
            }
        }
        match input {
            Input::Key(code) => self.press_key(surface, code),
            Input::Modifiers(modifiers) => {
                self.tree.borrow_mut().set_modifiers(modifiers);
                Step::Continue
            }
            Input::Paste(text) => {
                let events = self.tree.borrow_mut().paste_text(&text);
                self.dispatch_events(surface, events)
            }
            Input::Moved(x, y) => self.move_pointer(surface, x, y),
            Input::Left => self.leave_pointer(surface),
            Input::Down(x, y) => self.press_pointer(surface, x, y),
            Input::Up(x, y) => self.release_pointer(surface, x, y),
            Input::Wheel(pixels) => self.wheel(surface, pixels),
            other => self.window_action(surface, other),
        }
    }

    fn window_action(&mut self, surface: &dyn Surface, input: Input) -> Step {
        match input {
            Input::Exit => Step::Close,
            Input::Theme(scheme) => self.change_theme(surface, scheme),
            _ => self.resize_to_surface(surface),
        }
    }

    pub(crate) fn take_clipboard(&mut self) -> Option<String> {
        self.clipboard.take()
    }

    pub(crate) fn take_paste(&mut self) -> bool {
        std::mem::take(&mut self.paste)
    }

    fn press_key(&mut self, surface: &dyn Surface, code: String) -> Step {
        let events = self.tree.borrow_mut().press_key(&code);
        self.dispatch_events(surface, events)
    }

    fn move_pointer(&mut self, surface: &dyn Surface, x: f64, y: f64) -> Step {
        let (x, y) = point(x, y);
        self.cursor = (x, y);
        let events = self.tree.borrow_mut().pointer_moved(x, y);
        self.dispatch(surface, events)
    }

    fn wheel(&mut self, surface: &dyn Surface, pixels: f64) -> Step {
        if !pixels.is_finite() {
            return Step::Continue;
        }
        let (x, y) = self.cursor;
        let dy = pixels.round() as i32;
        if self.tree.borrow_mut().wheel(x, y, dy) {
            surface.redraw();
        }
        Step::Continue
    }

    fn leave_pointer(&mut self, surface: &dyn Surface) -> Step {
        let events = self.tree.borrow_mut().pointer_leave();
        self.dispatch_events(surface, events)
    }

    fn press_pointer(&mut self, surface: &dyn Surface, x: f64, y: f64) -> Step {
        if self.config.is_header(y, surface.scale_factor()) {
            surface.drag();
            return Step::Continue;
        }
        let (x, y) = point(x, y);
        let events = self.tree.borrow_mut().pointer_down(x, y);
        self.dispatch(surface, events)
    }

    fn release_pointer(&mut self, surface: &dyn Surface, x: f64, y: f64) -> Step {
        let (x, y) = point(x, y);
        let events = self.tree.borrow_mut().pointer_up(x, y);
        self.dispatch(surface, events)
    }

    fn change_theme(&mut self, surface: &dyn Surface, scheme: ColorScheme) -> Step {
        let Some(build) = self.build.as_mut() else {
            return Step::Continue;
        };
        info!(?scheme, "system theme changed");
        self.tree = build(scheme);
        self.resize_to_surface(surface)
    }

    pub(crate) fn replace_tree(&mut self, tree: Rc<RefCell<WidgetTree>>) -> Step {
        debug!("replacing the widget tree");
        self.tree = tree;
        match self.window.clone() {
            Some(window) => self.resize_to_surface(&window),
            None => Step::Continue,
        }
    }

    fn resize_to_surface(&mut self, surface: &dyn Surface) -> Step {
        let (width, height) = surface.size();
        self.resize(surface, width, height)
    }

    fn fail(&mut self, error: WindowError) -> Step {
        error!(%error, "window failed");
        self.error.set(Some(error));
        Step::Exit
    }

    fn finish(&mut self, event_loop: &dyn ActiveEventLoop, step: Step) {
        if matches!(step, Step::Close | Step::Exit) {
            event_loop.exit();
        }
    }

    fn dispatch(
        &mut self,
        surface: &dyn Surface,
        events: Result<Vec<WidgetEvent>, brul_gui::WidgetError>,
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
        for event in &events {
            match event {
                WidgetEvent::Copy(text) => {
                    self.clipboard = (!text.is_empty()).then(|| text.clone());
                }
                WidgetEvent::Paste => self.paste = true,
                _ => {}
            }
        }
        debug!(?events, "widget events");
        let commands = (self.handler)(&events);
        debug!(?commands, "window commands");
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
                WindowCommand::Close => return Step::Close,
                WindowCommand::Open(key) => return Step::Open(key),
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
        debug!(width, height, "window resized");
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.resize(width, height);
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

    fn render(&mut self, surface: &dyn Surface) -> Step {
        brul_reactive::flush();
        match self.render_frame(surface) {
            Ok(()) => Step::Continue,
            Err(error) => self.fail(WindowError::stage("widgets", error)),
        }
    }

    fn render_frame(&mut self, surface: &dyn Surface) -> Result<(), brul_gui::WidgetError> {
        trace!("rendering a frame");
        let now = Instant::now();
        let delta = now.saturating_duration_since(self.tick);
        self.tick = now;
        self.tree.borrow_mut().advance(delta);
        if self.tree.borrow().layout_dirty()
            && let Some(engine) = self.engine.as_mut()
        {
            if self.tree.borrow().has_layout() {
                self.tree.borrow_mut().relayout(engine)?;
            } else {
                let (width, height) = surface.size();
                self.tree.borrow_mut().layout(
                    engine,
                    Size { width, height },
                    surface.scale_factor() as f32,
                )?;
            }
        }
        let (dirty, rebuild) = {
            let mut tree = self.tree.borrow_mut();
            (tree.take_dirty(), tree.snapshot_dirty())
        };
        let Some(renderer) = self.renderer.as_mut() else {
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

        renderer.present(surface, frame);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use brul_gui::{Frame, Size, TextEngine, Widget, WidgetError, WidgetEvent, WidgetTree};

    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use winit::event_loop::ControlFlow;

    use super::ColorScheme;
    use super::WindowCommand;
    use super::WindowKey;
    use super::{
        EventHandler, Input, Renderer, Step, Surface, Timer, TreeApp, WindowConfig, due, wakeup,
        wakeup_at,
    };

    struct FakeSurface {
        scale: f64,
        size: (u32, u32),
        drags: Cell<usize>,
        redraws: Cell<usize>,
    }

    impl FakeSurface {
        fn new(scale: f64) -> Self {
            Self {
                scale,
                size: (8, 6),
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

        fn size(&self) -> (u32, u32) {
            self.size
        }

        fn pre_present_notify(&self) {}
    }

    struct FakeRenderer {
        calls: Rc<RefCell<Vec<String>>>,
    }

    impl FakeRenderer {
        fn new() -> (Self, Rc<RefCell<Vec<String>>>) {
            let calls = Rc::new(RefCell::new(Vec::new()));
            (
                Self {
                    calls: Rc::clone(&calls),
                },
                calls,
            )
        }
    }

    impl Renderer for FakeRenderer {
        fn resize(&mut self, width: u32, height: u32) {
            self.calls
                .borrow_mut()
                .push(format!("resize {width}x{height}"));
        }

        fn present(&mut self, _surface: &dyn Surface, _frame: &Frame) {
            self.calls.borrow_mut().push("present".to_owned());
        }
    }

    fn tree_app(handler: EventHandler) -> TreeApp {
        super::tree_app(
            WindowConfig::default(),
            Rc::new(RefCell::new(WidgetTree::new(Widget::text("")))),
            Some(Box::new(|_| {
                Rc::new(RefCell::new(WidgetTree::new(Widget::text(""))))
            })),
            handler,
            Rc::new(Cell::new(None)),
            Vec::new(),
        )
    }

    #[test]
    fn the_initial_timers_are_started_at_launch() {
        let app = super::tree_app(
            WindowConfig::default(),
            Rc::new(RefCell::new(WidgetTree::new(Widget::text("")))),
            Some(Box::new(|_| {
                Rc::new(RefCell::new(WidgetTree::new(Widget::text(""))))
            })),
            commands(Vec::new()),
            Rc::new(Cell::new(None)),
            vec![
                WindowCommand::Exit,
                WindowCommand::Every {
                    interval: Duration::from_millis(250),
                    function: "tick".to_owned(),
                },
            ],
        );

        assert_eq!(app.timers.len(), 1);
    }
    fn commands(commands: Vec<WindowCommand>) -> EventHandler {
        Box::new(move |_| commands.clone())
    }

    #[test]
    fn an_input_after_a_rebuild_lays_the_tree_out_instead_of_failing() {
        let mut app = tree_app(commands(Vec::new()));
        app.engine = Some(TextEngine::new());
        let surface = FakeSurface::new(1.0);
        assert!(!app.tree.borrow().has_layout());

        let step = app.handle(&surface, Input::Moved(1.0, 1.0));

        assert!(matches!(step, Step::Continue));
        assert!(app.tree.borrow().has_layout());
    }

    #[test]
    fn an_input_on_a_zero_size_window_is_dropped() {
        let mut app = tree_app(commands(Vec::new()));
        app.engine = Some(TextEngine::new());
        let mut surface = FakeSurface::new(1.0);
        surface.size = (0, 0);

        let step = app.handle(&surface, Input::Down(1.0, 1.0));

        assert!(matches!(step, Step::Continue));
        assert!(!app.tree.borrow().has_layout());
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
        assert_eq!(wakeup(&[], None), ControlFlow::Wait);

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
            wakeup(&timers, None),
            ControlFlow::WaitUntil(now + Duration::from_secs(1))
        );

        timers[0].next = now + Duration::from_secs(1);
        assert_eq!(
            wakeup(&timers, None),
            ControlFlow::WaitUntil(now + Duration::from_secs(1))
        );
    }

    #[test]
    fn a_frame_deadline_wakes_the_loop_before_a_later_timer() {
        let now = Instant::now();
        let frame = now + Duration::from_millis(16);
        let timers = vec![Timer {
            next: now + Duration::from_secs(1),
            interval: Duration::from_secs(1),
            function: "late".to_owned(),
        }];

        assert_eq!(wakeup(&timers, Some(frame)), ControlFlow::WaitUntil(frame));
        assert_eq!(wakeup(&[], Some(frame)), ControlFlow::WaitUntil(frame));
    }

    #[test]
    fn a_missing_deadline_waits_for_an_event() {
        assert_eq!(wakeup_at(None), ControlFlow::Wait);
        let app = tree_app(commands(Vec::new()));
        assert_eq!(app.deadline(), None);
    }

    #[test]
    fn a_window_deadline_is_its_next_timer() {
        let now = Instant::now();
        let mut app = tree_app(commands(Vec::new()));
        app.timers.push(Timer {
            next: now,
            interval: Duration::from_secs(1),
            function: "tick".to_owned(),
        });

        assert_eq!(app.deadline(), Some(now));
        assert_eq!(wakeup_at(app.deadline()), ControlFlow::WaitUntil(now));
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
    fn the_close_command_closes_the_window() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(vec![WindowCommand::Close]));
        let widget = Widget::text("x").id();

        let step = app.dispatch_events(&surface, vec![WidgetEvent::Press(widget)]);
        assert_eq!(step, Step::Close);
    }

    #[test]
    fn the_open_command_names_the_window() {
        let surface = FakeSurface::new(1.0);
        let key = WindowKey(7);
        let mut app = tree_app(commands(vec![WindowCommand::Open(key)]));
        let widget = Widget::text("x").id();

        let step = app.dispatch_events(&surface, vec![WidgetEvent::Press(widget)]);
        assert_eq!(step, Step::Open(key));
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
        assert!(app.error.take().is_some());
    }
    #[test]
    fn a_key_reaches_the_tree() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(Vec::new()));

        let code = "Enter".to_owned();
        assert_eq!(app.handle(&surface, Input::Key(code)), Step::Continue);
    }

    #[test]
    fn pointer_input_reaches_the_tree() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(Vec::new()));
        let mut engine = TextEngine::new();
        let size = Size {
            width: 8,
            height: 6,
        };
        app.tree
            .borrow_mut()
            .layout(&mut engine, size, 1.0)
            .unwrap();
        app.engine = Some(engine);

        assert_eq!(app.handle(&surface, Input::Moved(1.5, 2.5)), Step::Continue);
        assert_eq!(app.handle(&surface, Input::Left), Step::Continue);
        assert_eq!(app.handle(&surface, Input::Up(1.0, 2.0)), Step::Continue);
        assert_eq!(app.handle(&surface, Input::Down(1.0, 40.0)), Step::Continue);
    }

    #[test]
    fn a_press_in_the_header_drags_the_window() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(Vec::new()));

        assert_eq!(app.handle(&surface, Input::Down(1.0, 1.0)), Step::Continue);
        assert_eq!(surface.drags.get(), 1);
    }

    #[test]
    fn a_theme_and_a_surface_change_rebuild_and_resize() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(Vec::new()));

        let step = app.handle(&surface, Input::Theme(ColorScheme::Light));
        assert_eq!(step, Step::Continue);
        assert_eq!(app.handle(&surface, Input::Surface), Step::Continue);
        assert_eq!(surface.redraws.get(), 2);
    }

    #[test]
    fn the_window_action_reports_a_close() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(Vec::new()));

        assert_eq!(app.window_action(&surface, Input::Exit), Step::Close);
    }

    #[test]
    fn a_failing_widget_event_is_reported_and_stops_the_loop() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(Vec::new()));

        let step = app.dispatch(&surface, Err(WidgetError::NoLayout));
        assert_eq!(step, Step::Exit);
        assert!(app.error.take().is_some());
    }

    #[test]
    fn a_resize_reaches_the_renderer() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(Vec::new()));
        let (renderer, calls) = FakeRenderer::new();
        app.renderer = Some(Box::new(renderer));

        assert_eq!(app.resize(&surface, 4, 5), Step::Continue);
        assert_eq!(calls.borrow().as_slice(), ["resize 4x5"]);
    }

    #[test]
    fn a_replaced_tree_is_laid_out_again_before_the_frame() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(Vec::new()));
        app.engine = Some(TextEngine::new());
        let (renderer, _) = FakeRenderer::new();
        app.renderer = Some(Box::new(renderer));

        assert_eq!(app.resize(&surface, 8, 6), Step::Continue);
        app.tree.borrow_mut().replace_root(Widget::text(""));

        assert!(app.render_frame(&surface).is_ok());
        assert!(!app.tree.borrow().layout_dirty());
    }

    #[test]
    fn a_frame_without_a_renderer_is_skipped() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(Vec::new()));

        assert!(app.render_frame(&surface).is_ok());
    }

    #[test]
    fn a_frame_is_painted_and_presented() {
        let surface = FakeSurface::new(1.0);
        let mut app = tree_app(commands(Vec::new()));
        let (renderer, calls) = FakeRenderer::new();
        app.renderer = Some(Box::new(renderer));
        let mut engine = TextEngine::new();
        app.tree
            .borrow_mut()
            .layout(
                &mut engine,
                Size {
                    width: 8,
                    height: 6,
                },
                1.0,
            )
            .expect("layout");
        app.engine = Some(engine);
        app.frame = Some(Frame::new(8, 6));

        assert!(app.render_frame(&surface).is_ok());
        assert_eq!(calls.borrow().as_slice(), ["present"]);
    }
}
