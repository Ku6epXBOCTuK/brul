#[path = "tests/commands.rs"]
mod event_cases;
#[path = "tests/platform.rs"]
mod platform;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use brul_gui::{
    Atlas, Cursor, Dimension, Dirty, DispatchEvent, Event, KeyCode, KeyModifiers, KeyPress,
    LayoutStyle, PlatformViewId, RawEvent, RawUiEvent, Size, StateId, StyleSet, TextEngine,
    TimerOwner, UiEvent, Widget, WidgetError, WidgetSnapshot, WidgetStyle, WidgetTree,
};

use winit::event_loop::ControlFlow;

use crate::input::Input;
use crate::platform::{PlatformViewBackend, PlatformViewBounds};
use crate::surface::{Renderer, Surface, TestRenderer};
use crate::window::{ColorScheme, TimerCommand, WindowCommand, WindowConfig, WindowKey};

use super::{EventHandler, Step, Timer, TreeApp, wakeup_at};

struct FakeSurface {
    scale: f64,
    size: (u32, u32),
    drags: Cell<usize>,
    redraws: Cell<usize>,
    visible: Cell<Option<bool>>,
    on_top: Cell<Option<bool>>,
    focus: Cell<usize>,
    cursors: RefCell<Vec<Cursor>>,
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
            cursors: RefCell::new(Vec::new()),
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

    fn set_cursor(&self, cursor: Cursor) {
        self.cursors.borrow_mut().push(cursor);
    }
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

impl TestRenderer for FakeRenderer {
    fn resize(&mut self, width: u32, height: u32) {
        self.calls
            .borrow_mut()
            .push(format!("resize {width}x{height}"));
    }

    fn clear(&mut self) -> bool {
        self.calls.borrow_mut().push("clear".to_owned());
        true
    }

    fn render(
        &mut self,
        _snapshot: &WidgetSnapshot,
        _atlas: &Atlas,
        _dirty: &Dirty,
        _info: crate::surface::FrameInfo,
        _effects: &brul_gpu::effect::Effects,
    ) -> bool {
        self.calls.borrow_mut().push("render".to_owned());
        true
    }

    fn capture(&mut self) -> Option<Vec<u8>> {
        None
    }
}

fn renderable_app() -> (TreeApp, FakeSurface, Rc<RefCell<Vec<String>>>) {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(Vec::new()));
    let (renderer, calls) = FakeRenderer::new();
    app.renderer = Some(Renderer::Test(Box::new(renderer)));
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
        None,
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
            WindowCommand::Timer(TimerCommand::Start {
                id: brul_ident::TimerId::new(1),
                owner: TimerOwner::new(0),
                function: "tick".into(),
                interval: Duration::from_millis(250),
                kind: brul_ident::TimerKind::Every,
            }),
        ],
        None,
    );

    assert_eq!(app.timers.len(), 1);
    assert_eq!(app.initial, vec![WindowCommand::Exit]);
}
fn commands(commands: Vec<WindowCommand>) -> EventHandler {
    Box::new(move |_| commands.clone())
}

fn cursor_app(cursor: Cursor) -> (TreeApp, FakeSurface) {
    let mut app = tree_app(commands(Vec::new()));
    let widget = Widget::column(vec![])
        .with_layout(LayoutStyle {
            width: Dimension::Px(8),
            height: Dimension::Px(6),
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            cursor,
            ..WidgetStyle::default()
        }));
    *app.tree.borrow_mut() = WidgetTree::new(widget);
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
    (app, FakeSurface::new(1.0))
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
        owner: TimerOwner::new(0),
        function: "tick".into(),
        id: brul_ident::TimerId::new(1),
        kind: brul_ident::TimerKind::Every,
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
fn a_hovered_cursor_is_installed_once_and_restored_on_leave() {
    let (mut app, surface) = cursor_app(Cursor::Text);

    assert_eq!(app.handle(&surface, Input::Moved(1.0, 1.0)), Step::Continue);
    assert_eq!(surface.cursors.borrow().as_slice(), &[Cursor::Text]);
    app.handle(&surface, Input::Moved(2.0, 2.0));
    assert_eq!(surface.cursors.borrow().len(), 1);
    app.handle(&surface, Input::Left);
    assert_eq!(
        surface.cursors.borrow().as_slice(),
        &[Cursor::Text, Cursor::Default]
    );
}

#[test]
fn a_default_cursor_does_not_touch_the_window_cursor() {
    let (mut app, surface) = cursor_app(Cursor::Default);

    app.handle(&surface, Input::Moved(1.0, 1.0));
    app.handle(&surface, Input::Down(1.0, 1.0));
    app.handle(&surface, Input::Left);

    assert!(surface.cursors.borrow().is_empty());
}

#[test]
fn a_drag_event_drags_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(Vec::new()));
    let widget = Widget::text("x").id();

    let step = app.dispatch_events(
        &surface,
        vec![RawUiEvent::widget(widget, RawEvent::drag(0, 0)).into()],
    );
    assert_eq!(step, Step::Continue);
    assert_eq!(surface.drags.get(), 1);
}

#[test]
fn a_close_request_waits_for_the_handler() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(Vec::new()));
    assert_eq!(
        app.dispatch_events(&surface, vec![UiEvent::window(Event::close()).into()]),
        Step::Continue
    );

    let mut app = tree_app(commands(vec![WindowCommand::Close]));
    assert_eq!(
        app.dispatch_events(&surface, vec![UiEvent::window(Event::close()).into()]),
        Step::Close
    );
}

#[test]
fn the_exit_command_finishes_the_loop() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Exit]));
    let widget = Widget::text("x").id();

    let step = app.dispatch_events(
        &surface,
        vec![RawUiEvent::widget(widget, RawEvent::press(0, 0, KeyModifiers::default())).into()],
    );
    assert_eq!(step, Step::Exit);
}

#[test]
fn the_close_command_closes_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Close]));
    let widget = Widget::text("x").id();

    let step = app.dispatch_events(
        &surface,
        vec![RawUiEvent::widget(widget, RawEvent::press(0, 0, KeyModifiers::default())).into()],
    );
    assert_eq!(step, Step::Close);
}

#[test]
fn the_open_command_names_the_window() {
    let surface = FakeSurface::new(1.0);
    let key = WindowKey(7);
    let mut app = tree_app(commands(vec![WindowCommand::Open(key)]));
    let widget = Widget::text("x").id();

    let step = app.dispatch_events(
        &surface,
        vec![RawUiEvent::widget(widget, RawEvent::press(0, 0, KeyModifiers::default())).into()],
    );
    assert_eq!(step, Step::Open(key));
}

#[test]
fn a_drag_command_drags_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Drag]));
    let widget = Widget::text("x").id();

    app.dispatch_events(
        &surface,
        vec![RawUiEvent::widget(widget, RawEvent::press(0, 0, KeyModifiers::default())).into()],
    );
    assert_eq!(surface.drags.get(), 1);
}

#[test]
fn a_hide_command_hides_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Hide]));
    let widget = Widget::text("x").id();

    let step = app.dispatch_events(
        &surface,
        vec![RawUiEvent::widget(widget, RawEvent::press(0, 0, KeyModifiers::default())).into()],
    );
    assert_eq!(step, Step::Continue);
    assert_eq!(surface.visible.get(), Some(false));
}

#[test]
fn a_show_command_shows_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Show]));
    let widget = Widget::text("x").id();

    app.dispatch_events(
        &surface,
        vec![RawUiEvent::widget(widget, RawEvent::press(0, 0, KeyModifiers::default())).into()],
    );
    assert_eq!(surface.visible.get(), Some(true));
}

#[test]
fn an_always_on_top_command_raises_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::AlwaysOnTop(true)]));
    let widget = Widget::text("x").id();

    app.dispatch_events(
        &surface,
        vec![RawUiEvent::widget(widget, RawEvent::press(0, 0, KeyModifiers::default())).into()],
    );
    assert_eq!(surface.on_top.get(), Some(true));
}

#[test]
fn a_focus_command_takes_the_focus() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Show, WindowCommand::Focus]));
    let widget = Widget::text("x").id();

    app.dispatch_events(
        &surface,
        vec![RawUiEvent::widget(widget, RawEvent::press(0, 0, KeyModifiers::default())).into()],
    );
    assert_eq!(surface.focus.get(), 1);
}

#[test]
fn a_timer_command_is_kept() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Timer(TimerCommand::Start {
        id: brul_ident::TimerId::new(1),
        owner: TimerOwner::new(0),
        function: "tick".into(),
        interval: Duration::from_millis(100),
        kind: brul_ident::TimerKind::Every,
    })]));
    let widget = Widget::text("x").id();

    app.dispatch_events(
        &surface,
        vec![RawUiEvent::widget(widget, RawEvent::press(0, 0, KeyModifiers::default())).into()],
    );
    assert_eq!(app.timers.len(), 1);
}

#[test]
fn a_timer_cancel_removes_the_started_timer() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![
        WindowCommand::Timer(TimerCommand::Start {
            id: brul_ident::TimerId::new(1),
            owner: TimerOwner::new(0),
            function: "tick".into(),
            interval: Duration::from_millis(100),
            kind: brul_ident::TimerKind::Every,
        }),
        WindowCommand::Timer(TimerCommand::Cancel {
            id: brul_ident::TimerId::new(1),
            owner: TimerOwner::new(0),
        }),
    ]));
    let widget = Widget::text("x").id();

    app.dispatch_events(
        &surface,
        vec![RawUiEvent::widget(widget, RawEvent::press(0, 0, KeyModifiers::default())).into()],
    );
    assert!(app.timers.is_empty());
}
