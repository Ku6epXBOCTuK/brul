use super::*;
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
    let mut app = super::super::tree_app(
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
