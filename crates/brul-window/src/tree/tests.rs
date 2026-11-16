use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use brul_gui::{Frame, Size, TextEngine, Widget, WidgetError, WidgetEvent, WidgetTree};
use winit::event_loop::ControlFlow;

use crate::input::Input;
use crate::surface::{Renderer, Surface};
use crate::window::{ColorScheme, WindowCommand, WindowConfig, WindowKey};

use super::{EventHandler, Step, Timer, TreeApp, wakeup_at};

struct FakeSurface {
    scale: f64,
    size: (u32, u32),
    drags: Cell<usize>,
    redraws: Cell<usize>,
    visible: Cell<Option<bool>>,
    on_top: Cell<Option<bool>>,
    focus: Cell<usize>,
}

impl FakeSurface {
    fn new(scale: f64) -> Self {
        Self {
            scale,
            size: (8, 6),
            drags: Cell::new(0),
            redraws: Cell::new(0),
            visible: Cell::new(None),
            on_top: Cell::new(None),
            focus: Cell::new(0),
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

    fn set_visible(&self, visible: bool) {
        self.visible.set(Some(visible));
    }

    fn focus(&self) {
        self.focus.set(self.focus.get() + 1);
    }

    fn set_always_on_top(&self, on_top: bool) {
        self.on_top.set(Some(on_top));
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

    fn present(&mut self, _surface: &dyn Surface, _frame: &Frame) -> bool {
        self.calls.borrow_mut().push("present".to_owned());
        true
    }
}

fn renderable_app() -> (TreeApp, FakeSurface, Rc<RefCell<Vec<String>>>) {
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
    (app, surface, calls)
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
    assert_eq!(app.initial, vec![WindowCommand::Exit]);
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
fn a_close_request_waits_for_the_handler() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(Vec::new()));
    assert_eq!(
        app.dispatch_events(&surface, vec![WidgetEvent::Close]),
        Step::Continue
    );

    let mut app = tree_app(commands(vec![WindowCommand::Close]));
    assert_eq!(
        app.dispatch_events(&surface, vec![WidgetEvent::Close]),
        Step::Close
    );
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
fn a_hide_command_hides_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Hide]));
    let widget = Widget::text("x").id();

    let step = app.dispatch_events(&surface, vec![WidgetEvent::Press(widget)]);
    assert_eq!(step, Step::Continue);
    assert_eq!(surface.visible.get(), Some(false));
}

#[test]
fn a_show_command_shows_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Show]));
    let widget = Widget::text("x").id();

    app.dispatch_events(&surface, vec![WidgetEvent::Press(widget)]);
    assert_eq!(surface.visible.get(), Some(true));
}

#[test]
fn an_always_on_top_command_raises_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::AlwaysOnTop(true)]));
    let widget = Widget::text("x").id();

    app.dispatch_events(&surface, vec![WidgetEvent::Press(widget)]);
    assert_eq!(surface.on_top.get(), Some(true));
}

#[test]
fn a_focus_command_takes_the_focus() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Show, WindowCommand::Focus]));
    let widget = Widget::text("x").id();

    app.dispatch_events(&surface, vec![WidgetEvent::Press(widget)]);
    assert_eq!(surface.focus.get(), 1);
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
fn the_first_frame_shows_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(Vec::new()));
    app.engine = Some(TextEngine::new());
    let (renderer, _) = FakeRenderer::new();
    app.renderer = Some(Box::new(renderer));
    app.frame = Some(Frame::new(8, 6));

    assert_eq!(surface.visible.get(), None);
    assert!(app.render_frame(&surface).is_ok());
    assert_eq!(surface.visible.get(), Some(true));
}

#[test]
fn a_window_hidden_by_its_config_stays_hidden_after_the_first_frame() {
    let surface = FakeSurface::new(1.0);
    let mut app = super::tree_app(
        WindowConfig {
            visible: false,
            ..WindowConfig::default()
        },
        Rc::new(RefCell::new(WidgetTree::new(Widget::text("")))),
        Some(Box::new(|_| {
            Rc::new(RefCell::new(WidgetTree::new(Widget::text(""))))
        })),
        commands(Vec::new()),
        Rc::new(Cell::new(None)),
        Vec::new(),
    );
    app.engine = Some(TextEngine::new());
    let (renderer, _) = FakeRenderer::new();
    app.renderer = Some(Box::new(renderer));
    app.frame = Some(Frame::new(8, 6));

    assert!(app.render_frame(&surface).is_ok());
    assert_eq!(surface.visible.get(), Some(false));
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

#[test]
fn a_frame_without_changes_is_not_presented_twice() {
    let (mut app, surface, calls) = renderable_app();

    assert!(app.render_frame(&surface).is_ok());
    assert!(app.render_frame(&surface).is_ok());
    assert_eq!(calls.borrow().as_slice(), ["present"]);
}

#[test]
fn a_surface_without_the_frame_presents_it_again() {
    let (mut app, surface, calls) = renderable_app();

    assert!(app.render_frame(&surface).is_ok());
    app.presented = false;
    assert!(app.render_frame(&surface).is_ok());
    assert_eq!(calls.borrow().as_slice(), ["present", "present"]);
}

#[test]
fn a_resize_forgets_the_presented_frame() {
    let (mut app, surface, _calls) = renderable_app();

    assert!(app.render_frame(&surface).is_ok());
    assert!(app.presented);
    app.resize(&surface, 10, 8);
    assert!(!app.presented);
}
