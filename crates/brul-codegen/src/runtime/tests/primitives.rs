use super::*;

#[test]
fn an_image_binding_loads_the_file_once() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../brul-gui/tests/fixtures/tint.png"
    );
    let (binding, textures) = image_binding(Value::Image(path.into()));
    let first = evaluate(&binding, &textures);
    let second = evaluate(&binding, &textures);
    let (Write::Image(Some(first)), Write::Image(Some(second))) = (first, second) else {
        panic!("the fixture must load");
    };

    assert!(first.ptr_eq(&second));
    assert!(first.width() > 1);
    assert_eq!(textures.borrow().loaded.len(), 1);
}

#[test]
fn an_image_binding_decodes_an_embedded_resource() {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../brul-gui/tests/fixtures/tint.png"
    ))
    .unwrap();
    let (binding, textures) = image_binding(Value::image_bytes(&bytes));

    let Write::Image(Some(image)) = evaluate(&binding, &textures) else {
        panic!("the fixture must decode");
    };
    assert!(image.width() > 1);
    assert!(textures.borrow().loaded.is_empty());
}

#[test]
fn an_image_binding_ignores_a_missing_file() {
    let (binding, textures) = image_binding(Value::Image("no-such-image.png".into()));

    assert!(matches!(evaluate(&binding, &textures), Write::Image(None)));
    assert!(textures.borrow().loaded.is_empty());
}

#[test]
fn an_image_binding_remembers_a_missing_file() {
    let (binding, textures) = image_binding(Value::Image("no-such-image.png".into()));

    evaluate(&binding, &textures);
    assert!(matches!(evaluate(&binding, &textures), Write::Image(None)));
    assert_eq!(textures.borrow().failed.len(), 1);
}

#[test]
fn an_image_binding_shows_an_empty_texture_for_null() {
    let (binding, textures) = image_binding(Value::Null);
    let Write::Image(Some(texture)) = evaluate(&binding, &textures) else {
        panic!("null must be an empty texture");
    };

    assert_eq!((texture.width(), texture.height()), (1, 1));
}

#[test]
fn a_window_target_has_no_element() {
    let mut build = Build::new();
    build.set_target(
        EventTarget::Window,
        StateId::source("test", "one"),
        Rc::new(RefCell::new(Component::new())),
    );
    assert!(build.try_element().is_none());
}

#[test]
fn attach_primitives_edit_the_field_and_query_focus_and_flags() {
    while take_pending().is_some() {}

    let input = Widget::input("abc");
    let id = input.id();
    let root = Widget::row(vec![input, Widget::text("label")]);
    let root_id = root.id();
    let tree = Rc::new(RefCell::new(WidgetTree::new(root)));
    let mut build = Build::new();
    build.set_tree(Rc::clone(&tree));
    build.set_target(
        EventTarget::Widget(id),
        StateId::source("test", "nine"),
        Rc::new(RefCell::new(Component::new())),
    );

    let element = build.element();
    assert_eq!(element.edit_buffer().expect("input").text(), "abc");
    let edited = element.with_edit(|buffer| {
        buffer.insert_text(Position::new(0, 0), "x");
        buffer.text().to_owned()
    });
    assert_eq!(edited.as_deref(), Some("xabc"));
    assert_eq!(tree.borrow().edit_buffer(id).expect("input").text(), "xabc");

    let rejected = element.with_edit(|buffer| {
        buffer.insert_fragment(
            Position::new(0, 1),
            Fragment::Image(Image::from(Texture::empty())),
        );
    });
    assert!(rejected.is_none());
    assert_eq!(tree.borrow().edit_buffer(id).expect("input").text(), "xabc");
    assert_eq!(element.caret(), Some(Position::new(0, 4)));
    assert_eq!(
        element.selection(),
        Position::new(0, 4)..Position::new(0, 4)
    );

    assert_eq!(build.focused(), None);
    assert!(build.focus(Some(id)));
    assert_eq!(build.focused(), Some(id));
    assert!(element.focused());
    assert!(build.focusables().contains(&id));
    assert_eq!(build.path_to(id), vec![root_id, id]);
    assert_eq!(build.ancestors(id), vec![root_id]);
    assert_eq!(build.modal_scope(), None);
    assert!(build.within_scope(root_id, id));

    assert!(element.set_hovered(true));
    assert!(element.hovered());
    assert!(element.set_pressed(true));
    assert!(element.pressed());
    assert!(element.scroll_by(5));

    build.emit_event(UiEvent::widget(id, Event::click()));
    let Pending::Ui(event) = take_pending().expect("ui event") else {
        panic!("a semantic event was expected");
    };
    assert_eq!(event.target, EventTarget::Widget(id));
    assert!(take_pending().is_none());
}

#[test]
fn build_addresses_any_widget_and_hits_the_active_path() {
    let disabled = Widget::text("no")
        .with_layout(LayoutStyle {
            width: Dimension::Px(40),
            height: Dimension::Px(20),
            ..LayoutStyle::default()
        })
        .disabled(true);
    let input = Widget::input("yes").with_layout(LayoutStyle {
        width: Dimension::Px(40),
        height: Dimension::Px(20),
        ..LayoutStyle::default()
    });
    let input_id = input.id();
    let root = Widget::row(vec![disabled, input]).with_layout(LayoutStyle {
        width: Dimension::Px(100),
        height: Dimension::Px(100),
        ..LayoutStyle::default()
    });
    let root_id = root.id();
    let tree = Rc::new(RefCell::new(WidgetTree::new(root)));
    tree.borrow_mut()
        .layout(
            &mut TextEngine::new(),
            Size {
                width: 100,
                height: 100,
            },
            1.0,
        )
        .expect("layout");

    let mut build = Build::new();
    build.set_tree(Rc::clone(&tree));
    build.set_target(
        EventTarget::Window,
        StateId::source("test", "hits"),
        Rc::new(RefCell::new(Component::new())),
    );

    assert!(build.try_element().is_none());
    assert_eq!(build.element_for(input_id).map(|e| e.id()), Some(input_id));
    assert_eq!(build.hit(5, 5), Some(root_id));
    assert_eq!(build.hit(45, 5), Some(input_id));
}

#[test]
fn an_element_exposes_text_and_scroll_primitives() {
    let input = Widget::input("ab\ncd").wrap(true).with_layout(LayoutStyle {
        width: Dimension::Px(80),
        height: Dimension::Px(40),
        ..LayoutStyle::default()
    });
    let id = input.id();
    let root = Widget::column(vec![input]).with_layout(LayoutStyle {
        width: Dimension::Px(100),
        height: Dimension::Px(100),
        ..LayoutStyle::default()
    });
    let root_id = root.id();
    let tree = Rc::new(RefCell::new(WidgetTree::new(root)));
    tree.borrow_mut()
        .layout(
            &mut TextEngine::new(),
            Size {
                width: 100,
                height: 100,
            },
            1.0,
        )
        .expect("layout");
    tree.borrow_mut().set_root_scroll(true);

    let mut build = Build::new();
    build.set_tree(Rc::clone(&tree));
    build.set_target(
        EventTarget::Widget(id),
        StateId::source("test", "text"),
        Rc::new(RefCell::new(Component::new())),
    );

    let element = build.element();
    assert!(element.is_input());
    assert!(element.wrap());
    assert_eq!(element.line_start(3), Some(3));
    assert_eq!(element.line_end(3), Some(5));
    assert!(element.caret_above(3).is_some());
    assert!(element.caret_below(0).is_some());
    assert!(!element.scrollable());
    assert_eq!(build.nearest_scrollable(id), Some(root_id));
    build.enter_modal_scope();
    assert_eq!(build.focused(), None);
}

#[test]
fn clearing_focus_on_another_element_keeps_the_focused_one() {
    let first = Widget::input("a");
    let first_id = first.id();
    let second = Widget::input("b");
    let second_id = second.id();
    let root = Widget::row(vec![first, second]);
    let tree = Rc::new(RefCell::new(WidgetTree::new(root)));
    let mut build = Build::new();
    build.set_tree(Rc::clone(&tree));
    build.set_target(
        EventTarget::Window,
        StateId::source("test", "focus"),
        Rc::new(RefCell::new(Component::new())),
    );

    assert!(build.focus(Some(first_id)));
    assert!(
        !build
            .element_for(second_id)
            .expect("second")
            .set_focused(false)
    );
    assert_eq!(build.focused(), Some(first_id));
    assert!(
        build
            .element_for(first_id)
            .expect("first")
            .set_focused(false)
    );
    assert_eq!(build.focused(), None);
}

#[test]
fn a_capability_can_queue_a_raw_event() {
    let widget = Widget::text("");
    let id = widget.id();
    let build = Build::new();
    build.emit_raw_event(RawUiEvent::widget(
        id,
        RawEvent::key_noop(KeyPress::from("Backspace"), KeyModifiers::default()),
    ));

    let Pending::Raw(event) = take_pending().expect("raw event") else {
        panic!("a raw event was expected");
    };
    assert_eq!(event.target, EventTarget::Widget(id));
    assert!(matches!(event.event, RawEvent::KeyNoop { .. }));
    assert!(take_pending().is_none());
}

#[test]
fn an_element_maps_pixels_to_a_caret_position() {
    let input = Widget::input("ab").with_layout(LayoutStyle {
        width: Dimension::Px(80),
        height: Dimension::Px(20),
        ..LayoutStyle::default()
    });
    let id = input.id();
    let root = Widget::column(vec![input]).with_layout(LayoutStyle {
        width: Dimension::Px(100),
        height: Dimension::Px(40),
        ..LayoutStyle::default()
    });
    let tree = Rc::new(RefCell::new(WidgetTree::new(root)));
    tree.borrow_mut()
        .layout(
            &mut TextEngine::new(),
            Size {
                width: 100,
                height: 100,
            },
            1.0,
        )
        .expect("layout");

    let mut build = Build::new();
    build.set_tree(Rc::clone(&tree));
    build.set_target(
        EventTarget::Widget(id),
        StateId::source("test", "ten"),
        Rc::new(RefCell::new(Component::new())),
    );

    assert_eq!(
        build.element().caret_at_pixel(1000, 5),
        Some(Position::new(0, 2))
    );
}

#[test]
fn the_clipboard_port_is_typed_and_optional() {
    struct MemoryClipboard(RefCell<Option<ClipboardData>>);

    impl Clipboard for MemoryClipboard {
        fn read(&self) -> Option<ClipboardData> {
            self.0.borrow().clone()
        }

        fn write(&self, data: &ClipboardData) {
            *self.0.borrow_mut() = Some(data.clone());
        }
    }

    let mut build = Build::new();
    build.set_clipboard(&ClipboardData::from(String::from("copied")));
    assert!(build.clipboard().is_none());

    build.set_clipboard_bridge(Some(Rc::new(MemoryClipboard(RefCell::new(None)))));
    build.set_clipboard(&ClipboardData::from(String::from("copied")));
    assert!(matches!(build.clipboard(), Some(ClipboardData::Text(text)) if text == "copied"));
}
