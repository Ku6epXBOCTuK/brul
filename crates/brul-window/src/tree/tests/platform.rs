use super::*;
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
            id.state_id().path(),
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
            id.state_id().path(),
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            scale as u32
        ));
    }
    fn set_source(&mut self, id: PlatformViewId, source: &str) {
        self.push(format!("source {} {}", id.state_id().path(), source));
    }
    fn set_visible(&mut self, id: PlatformViewId, visible: bool) {
        self.push(format!("visible {} {visible}", id.state_id().path()));
    }
    fn focus(&mut self, id: PlatformViewId) {
        self.push(format!("focus {}", id.state_id().path()));
    }
    fn blur(&mut self) {
        self.push("blur".to_owned());
    }
    fn remove(&mut self, id: PlatformViewId) {
        self.push(format!("remove {}", id.state_id().path()));
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
            Widget::platform_view(
                PlatformViewId::new(StateId::source("test", "view")),
                "about:blank",
            )
            .focusable(true)
            .with_layout(LayoutStyle {
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
            .contains(&"create view about:blank 0,0,4x2 @1".to_owned()),
        "{calls:?}"
    );
}

#[test]
fn a_focus_event_focuses_and_blurs_a_platform_view() {
    let (mut app, surface, calls) = platform_app();
    let view = app
        .tree
        .borrow()
        .focusables()
        .into_iter()
        .find(|id| app.tree.borrow().platform_view(*id).is_some())
        .expect("platform view");
    assert!(app.tree.borrow_mut().set_focused(Some(view)));
    app.dispatch_events(
        &surface,
        vec![DispatchEvent::semantic(view, Event::focus())],
    );
    assert!(calls.borrow().contains(&"focus view".to_owned()));

    assert!(app.tree.borrow_mut().set_focused(None));
    app.dispatch_events(&surface, vec![DispatchEvent::semantic(view, Event::blur())]);
    assert!(calls.borrow().contains(&"blur".to_owned()));
}

#[test]
fn a_focused_platform_view_swallows_keyboard_input() {
    let (mut app, surface, calls) = platform_app();
    let view = app
        .tree
        .borrow()
        .focusables()
        .into_iter()
        .find(|id| app.tree.borrow().platform_view(*id).is_some())
        .expect("platform view");
    assert!(app.tree.borrow_mut().set_focused(Some(view)));
    app.dispatch_events(
        &surface,
        vec![DispatchEvent::semantic(view, Event::focus())],
    );

    let seen = Rc::new(RefCell::new(Vec::new()));
    let record = Rc::clone(&seen);
    app.handler = Box::new(move |events| {
        record
            .borrow_mut()
            .extend(events.iter().map(|event| format!("{event:?}")));
        Vec::new()
    });
    seen.borrow_mut().clear();

    app.handle(
        &surface,
        Input::Key(KeyPress {
            code: KeyCode::Character("A".to_owned()),
            physical: None,
        }),
    );
    assert!(seen.borrow().is_empty());
    assert!(!calls.borrow().contains(&"blur".to_owned()));
}
