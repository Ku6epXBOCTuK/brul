use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use brul_gui::{
    Atlas, Cursor, Dimension, Dirty, Event, KeyPress, LayoutStyle, PlatformViewId, Size, StyleSet,
    TextEngine, TimerOwner, UiEvent, Widget, WidgetError, WidgetSnapshot, WidgetStyle, WidgetTree,
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

    let step = app.dispatch_events(&surface, vec![UiEvent::widget(widget, Event::drag())]);
    assert_eq!(step, Step::Continue);
    assert_eq!(surface.drags.get(), 1);
}

#[test]
fn a_close_request_waits_for_the_handler() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(Vec::new()));
    assert_eq!(
        app.dispatch_events(&surface, vec![UiEvent::window(Event::close())]),
        Step::Continue
    );

    let mut app = tree_app(commands(vec![WindowCommand::Close]));
    assert_eq!(
        app.dispatch_events(&surface, vec![UiEvent::window(Event::close())]),
        Step::Close
    );
}

#[test]
fn the_exit_command_finishes_the_loop() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Exit]));
    let widget = Widget::text("x").id();

    let step = app.dispatch_events(&surface, vec![UiEvent::widget(widget, Event::press())]);
    assert_eq!(step, Step::Exit);
}

#[test]
fn the_close_command_closes_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Close]));
    let widget = Widget::text("x").id();

    let step = app.dispatch_events(&surface, vec![UiEvent::widget(widget, Event::press())]);
    assert_eq!(step, Step::Close);
}

#[test]
fn the_open_command_names_the_window() {
    let surface = FakeSurface::new(1.0);
    let key = WindowKey(7);
    let mut app = tree_app(commands(vec![WindowCommand::Open(key)]));
    let widget = Widget::text("x").id();

    let step = app.dispatch_events(&surface, vec![UiEvent::widget(widget, Event::press())]);
    assert_eq!(step, Step::Open(key));
}

#[test]
fn a_drag_command_drags_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Drag]));
    let widget = Widget::text("x").id();

    app.dispatch_events(&surface, vec![UiEvent::widget(widget, Event::press())]);
    assert_eq!(surface.drags.get(), 1);
}

#[test]
fn a_hide_command_hides_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Hide]));
    let widget = Widget::text("x").id();

    let step = app.dispatch_events(&surface, vec![UiEvent::widget(widget, Event::press())]);
    assert_eq!(step, Step::Continue);
    assert_eq!(surface.visible.get(), Some(false));
}

#[test]
fn a_show_command_shows_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Show]));
    let widget = Widget::text("x").id();

    app.dispatch_events(&surface, vec![UiEvent::widget(widget, Event::press())]);
    assert_eq!(surface.visible.get(), Some(true));
}

#[test]
fn an_always_on_top_command_raises_the_window() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::AlwaysOnTop(true)]));
    let widget = Widget::text("x").id();

    app.dispatch_events(&surface, vec![UiEvent::widget(widget, Event::press())]);
    assert_eq!(surface.on_top.get(), Some(true));
}

#[test]
fn a_focus_command_takes_the_focus() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(vec![WindowCommand::Show, WindowCommand::Focus]));
    let widget = Widget::text("x").id();

    app.dispatch_events(&surface, vec![UiEvent::widget(widget, Event::press())]);
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

    app.dispatch_events(&surface, vec![UiEvent::widget(widget, Event::press())]);
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

    app.dispatch_events(&surface, vec![UiEvent::widget(widget, Event::press())]);
    assert!(app.timers.is_empty());
}

#[test]
fn a_size_of_zero_is_ignored() {
    let surface = FakeSurface::new(2.0);
    let mut app = tree_app(commands(Vec::new()));

    assert_eq!(app.resize(&surface, 0, 10), Step::Continue);
    assert_eq!(surface.redraws.get(), 0);
}

#[test]
fn a_resize_asks_for_a_redraw() {
    let surface = FakeSurface::new(2.0);
    let mut app = tree_app(commands(Vec::new()));

    assert_eq!(app.resize(&surface, 4, 5), Step::Continue);
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
    assert_eq!(
        app.handle(&surface, Input::Key(KeyPress::from(code))),
        Step::Continue
    );
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
    app.renderer = Some(Renderer::Test(Box::new(renderer)));

    assert_eq!(app.resize(&surface, 4, 5), Step::Continue);
    assert_eq!(calls.borrow().as_slice(), ["resize 4x5"]);
}

#[test]
fn a_replaced_tree_is_laid_out_again_before_the_frame() {
    let surface = FakeSurface::new(1.0);
    let mut app = tree_app(commands(Vec::new()));
    app.engine = Some(TextEngine::new());
    let (renderer, _) = FakeRenderer::new();
    app.renderer = Some(Renderer::Test(Box::new(renderer)));

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
    app.renderer = Some(Renderer::Test(Box::new(renderer)));

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
        None,
    );
    app.engine = Some(TextEngine::new());
    let (renderer, _) = FakeRenderer::new();
    app.renderer = Some(Renderer::Test(Box::new(renderer)));

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
fn a_frame_is_rendered() {
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

    assert!(app.render_frame(&surface).is_ok());
    assert_eq!(calls.borrow().as_slice(), ["render"]);
}

#[test]
fn a_frame_without_changes_is_not_presented_twice() {
    let (mut app, surface, calls) = renderable_app();

    assert!(app.render_frame(&surface).is_ok());
    assert!(app.render_frame(&surface).is_ok());
    assert_eq!(calls.borrow().as_slice(), ["render"]);
}

#[test]
fn a_surface_without_the_frame_presents_it_again() {
    let (mut app, surface, calls) = renderable_app();

    assert!(app.render_frame(&surface).is_ok());
    app.presented = false;
    assert!(app.render_frame(&surface).is_ok());
    assert_eq!(calls.borrow().as_slice(), ["render", "render"]);
}

#[test]
fn a_resize_forgets_the_presented_frame() {
    let (mut app, surface, _calls) = renderable_app();

    assert!(app.render_frame(&surface).is_ok());
    assert!(app.presented);
    app.resize(&surface, 10, 8);
    assert!(!app.presented);
}

struct RecordingPlatform {
    calls: Rc<RefCell<Vec<String>>>,
}

impl RecordingPlatform {
    fn push(&self, call: String) {
        self.calls.borrow_mut().push(call);
    }
}

impl PlatformViewBackend for RecordingPlatform {
    fn create(&mut self, id: PlatformViewId, source: &str, bounds: PlatformViewBounds, scale: f64) {
        self.push(format!(
            "create {} {} {},{},{}x{} @{}",
            id.get(),
            source,
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            scale as u32
        ));
    }
    fn set_bounds(&mut self, id: PlatformViewId, bounds: PlatformViewBounds, scale: f64) {
        self.push(format!(
            "bounds {} {},{},{}x{} @{}",
            id.get(),
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            scale as u32
        ));
    }
    fn set_source(&mut self, id: PlatformViewId, source: &str) {
        self.push(format!("source {} {}", id.get(), source));
    }
    fn set_visible(&mut self, id: PlatformViewId, visible: bool) {
        self.push(format!("visible {} {visible}", id.get()));
    }
    fn focus(&mut self, id: PlatformViewId) {
        self.push(format!("focus {}", id.get()));
    }
    fn blur(&mut self) {
        self.push("blur".to_owned());
    }
    fn remove(&mut self, id: PlatformViewId) {
        self.push(format!("remove {}", id.get()));
    }
}

fn platform_app() -> (TreeApp, FakeSurface, Rc<RefCell<Vec<String>>>) {
    let (mut app, surface, _) = renderable_app();
    let calls = Rc::new(RefCell::new(Vec::new()));
    app.set_platform_backend(Box::new(RecordingPlatform {
        calls: Rc::clone(&calls),
    }));
    let mut engine = TextEngine::new();
    let tree = Rc::new(RefCell::new(WidgetTree::new(
        Widget::column(vec![
            Widget::platform_view(PlatformViewId::new(4), "about:blank").with_layout(LayoutStyle {
                width: Dimension::Px(4),
                height: Dimension::Px(2),
                ..LayoutStyle::default()
            }),
            Widget::input(""),
        ])
        .with_layout(LayoutStyle {
            width: Dimension::Px(8),
            height: Dimension::Px(6),
            ..LayoutStyle::default()
        }),
    )));
    tree.borrow_mut()
        .layout(
            &mut engine,
            Size {
                width: 8,
                height: 6,
            },
            2.0,
        )
        .expect("layout");
    app.tree = tree;
    app.engine = Some(engine);
    (app, surface, calls)
}

#[test]
fn platform_views_follow_the_snapshot_geometry() {
    let (mut app, surface, calls) = platform_app();

    app.render_frame(&surface).expect("render");

    assert!(
        calls
            .borrow()
            .contains(&"create 4 about:blank 0,0,4x2 @1".to_owned()),
        "{calls:?}"
    );
}

#[test]
fn tab_moves_focus_between_a_platform_view_and_the_tree() {
    let (mut app, surface, calls) = platform_app();
    let tab = || KeyPress {
        code: "Tab".to_owned(),
        physical: None,
    };

    let events = app.tree.borrow_mut().press_key(tab());
    app.dispatch_events(&surface, events);
    assert!(calls.borrow().contains(&"focus 4".to_owned()));

    let events = app.tree.borrow_mut().press_key(tab());
    app.dispatch_events(&surface, events);
    assert!(calls.borrow().contains(&"blur".to_owned()));
}

#[test]
fn a_focused_platform_view_keeps_keyboard_input_to_itself() {
    let (mut app, surface, calls) = platform_app();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let record = Rc::clone(&seen);
    app.handler = Box::new(move |events| {
        record
            .borrow_mut()
            .extend(events.iter().map(|event| format!("{event:?}")));
        Vec::new()
    });
    let events = app.tree.borrow_mut().press_key(KeyPress {
        code: "Tab".to_owned(),
        physical: None,
    });
    app.dispatch_events(&surface, events);
    seen.borrow_mut().clear();

    app.handle(
        &surface,
        Input::Key(KeyPress {
            code: "A".to_owned(),
            physical: None,
        }),
    );
    assert!(seen.borrow().is_empty());
    assert!(!calls.borrow().contains(&"blur".to_owned()));

    app.handle(
        &surface,
        Input::Key(KeyPress {
            code: "Tab".to_owned(),
            physical: None,
        }),
    );
    assert!(calls.borrow().contains(&"blur".to_owned()));
}
