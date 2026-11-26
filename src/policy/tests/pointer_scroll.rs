use super::*;
#[test]
pub(super) fn pointer_click_bubbles_from_leaf_to_root() {
    let text = Widget::text("ok").with_layout(fixed(40, 20));
    let text_id = text.id();
    let child = Widget::column(vec![text]).with_layout(fixed(80, 80));
    let child_id = child.id();
    let root = Widget::column(vec![child]).with_layout(fixed(100, 100));
    let root_id = root.id();
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble_laid_out(
        root,
        vec![pointer(&component), observer(&component)],
        Size {
            width: 100,
            height: 100,
        },
    );

    let modifiers = KeyModifiers::default();
    ui.dispatch_raw(&[
        raw(text_id, RawEvent::press(5, 5, modifiers)),
        raw(child_id, RawEvent::press(5, 5, modifiers)),
        raw(root_id, RawEvent::press(5, 5, modifiers)),
    ]);
    assert!(seen().is_empty());

    ui.dispatch_raw(&[
        raw(text_id, RawEvent::release(5, 5, modifiers)),
        raw(child_id, RawEvent::release(5, 5, modifiers)),
        raw(root_id, RawEvent::release(5, 5, modifiers)),
    ]);
    assert_eq!(seen(), ["click", "click", "click"]);
    let tree = ui.tree.borrow();
    for id in [text_id, child_id, root_id] {
        assert!(!tree.pressed(id));
    }
}

#[test]
pub(super) fn pointer_press_moves_focus_and_blurs_the_previous_widget() {
    let first = Widget::text("first")
        .focusable(true)
        .with_layout(fixed(40, 20));
    let first_id = first.id();
    let second = Widget::text("second")
        .focusable(true)
        .with_layout(fixed(40, 20));
    let second_id = second.id();
    let root = Widget::column(vec![first, second]).with_layout(fixed(100, 100));
    let root_id = root.id();
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble_laid_out(
        root,
        vec![pointer(&component), observer(&component)],
        Size {
            width: 100,
            height: 100,
        },
    );
    assert!(ui.tree.borrow_mut().set_focused(Some(first_id)));
    SEEN.with(|seen| seen.borrow_mut().clear());

    ui.dispatch_raw(&[raw(
        second_id,
        RawEvent::press(5, 25, KeyModifiers::default()),
    )]);
    assert_eq!(ui.tree.borrow().focused_id(), Some(second_id));
    assert_eq!(seen(), ["blur", "focus"]);

    SEEN.with(|seen| seen.borrow_mut().clear());
    ui.dispatch_raw(&[raw(
        root_id,
        RawEvent::press(95, 95, KeyModifiers::default()),
    )]);
    assert_eq!(ui.tree.borrow().focused_id(), None);
    assert_eq!(seen(), ["blur"]);
}

#[test]
pub(super) fn pointer_press_and_drag_place_and_extend_the_caret() {
    let input = Widget::input("ab").with_layout(fixed(80, 20));
    let id = input.id();
    let root = Widget::column(vec![input]).with_layout(fixed(100, 40));
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble_laid_out(
        root,
        vec![pointer(&component)],
        Size {
            width: 100,
            height: 40,
        },
    );

    let modifiers = KeyModifiers::default();
    ui.dispatch_raw(&[raw(id, RawEvent::press(60, 5, modifiers))]);
    let buffer = ui.tree.borrow().edit_buffer(id).expect("input");
    assert_eq!(buffer.caret().offset, 2);
    assert_eq!(buffer.selection().start.offset, 2);

    ui.dispatch_raw(&[raw(id, RawEvent::pointer_move(0, 5, modifiers))]);
    let buffer = ui.tree.borrow().edit_buffer(id).expect("input");
    let selection = buffer.selection();
    assert_eq!((selection.start.offset, selection.end.offset), (0, 2));
}

#[test]
pub(super) fn pointer_cancel_clears_pressed_without_clicking() {
    let root = Widget::column(vec![]).with_layout(fixed(100, 100));
    let id = root.id();
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble_laid_out(
        root,
        vec![pointer(&component), observer(&component)],
        Size {
            width: 100,
            height: 100,
        },
    );

    ui.dispatch_raw(&[raw(id, RawEvent::press(5, 5, KeyModifiers::default()))]);
    assert!(ui.tree.borrow().pressed(id));

    ui.dispatch_raw(&[raw(id, RawEvent::pointer_cancel())]);
    assert!(!ui.tree.borrow().pressed(id));
    assert!(seen().is_empty());
}

#[test]
pub(super) fn pointer_enter_and_leave_update_hover() {
    let root = Widget::column(vec![]).with_layout(fixed(100, 100));
    let id = root.id();
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble_laid_out(
        root,
        vec![pointer(&component)],
        Size {
            width: 100,
            height: 100,
        },
    );

    ui.dispatch_raw(&[raw(id, RawEvent::pointer_enter())]);
    assert!(ui.tree.borrow().hovered(id));
    ui.dispatch_raw(&[raw(id, RawEvent::pointer_leave())]);
    assert!(!ui.tree.borrow().hovered(id));
}

pub(super) fn scrollable_list() -> (Widget, WidgetId) {
    let first = Widget::text("one").with_layout(fixed(80, 60));
    let first_id = first.id();
    let list = Widget::column(vec![first, Widget::text("two").with_layout(fixed(80, 60))])
        .with_layout(fixed(80, 80))
        .with_styles(StyleSet::new(WidgetStyle {
            clip: true,
            scroll: true,
            ..WidgetStyle::default()
        }));
    (list, first_id)
}

#[test]
pub(super) fn wheel_scrolls_the_nearest_scrollable_ancestor() {
    let (list, first_id) = scrollable_list();
    let root = Widget::column(vec![list]).with_layout(fixed(80, 100));
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble_laid_out(
        root,
        vec![scroll(&component)],
        Size {
            width: 80,
            height: 100,
        },
    );

    ui.dispatch_raw(&[raw(first_id, RawEvent::wheel(40))]);

    {
        let mut tree = ui.tree.borrow_mut();
        tree.layout(
            &mut TextEngine::new(),
            Size {
                width: 80,
                height: 100,
            },
            1.0,
        )
        .expect("relayout");
    }
    let mut tree = ui.tree.borrow_mut();
    let snapshot = tree.snapshot().expect("snapshot");
    assert!(snapshot.node(first_id).expect("first").y < 0);
}

#[test]
pub(super) fn wheel_inside_a_modal_does_not_scroll_the_content_below() {
    let (list, first_id) = scrollable_list();
    let card = Widget::text("dialog").with_layout(fixed(20, 20));
    let card_id = card.id();
    let dialog = Widget::layer(vec![card]).modal(true);
    let root = Widget::column(vec![list, dialog]).with_layout(fixed(80, 100));
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble_laid_out(
        root,
        vec![scroll(&component)],
        Size {
            width: 80,
            height: 100,
        },
    );

    ui.dispatch_raw(&[raw(card_id, RawEvent::wheel(40))]);

    let mut tree = ui.tree.borrow_mut();
    let snapshot = tree.snapshot().expect("snapshot");
    assert_eq!(snapshot.node(first_id).expect("first").y, 0);
}

#[test]
pub(super) fn wheel_scrolls_the_root_when_root_scrolling_is_enabled() {
    let child = Widget::text("one").with_layout(fixed(80, 120));
    let child_id = child.id();
    let root = Widget::column(vec![child]).with_layout(fixed(80, 100));
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble_laid_out(
        root,
        vec![scroll(&component)],
        Size {
            width: 80,
            height: 100,
        },
    );
    ui.tree.borrow_mut().set_root_scroll(true);

    ui.dispatch_raw(&[raw(child_id, RawEvent::wheel(30))]);

    {
        let mut tree = ui.tree.borrow_mut();
        tree.layout(
            &mut TextEngine::new(),
            Size {
                width: 80,
                height: 100,
            },
            1.0,
        )
        .expect("relayout");
    }
    let mut tree = ui.tree.borrow_mut();
    let snapshot = tree.snapshot().expect("snapshot");
    assert!(snapshot.node(child_id).expect("child").y < 0);
}

#[test]
pub(super) fn builtin_hook_masks_match_the_codegen_catalog() {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    for hooks in [
        editor(&component),
        keyboard(&component),
        pointer(&component),
        scroll(&component),
    ] {
        let events = policy_events(hooks.state.path());
        assert_eq!(hooks.events, EventSet::from_iter(events.semantic));
        assert_eq!(hooks.raw_events, RawEventSet::from_iter(events.raw));
    }
    assert_eq!(POLICY_NAMES.len(), 4);
}

#[test]
pub(super) fn a_click_in_a_disabled_subtree_does_not_click() {
    let child = Widget::text("no").disabled(true).with_layout(fixed(40, 20));
    let root = Widget::column(vec![child]).with_layout(fixed(80, 40));
    let root_id = root.id();
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble_laid_out(
        root,
        vec![pointer(&component), observer(&component)],
        Size {
            width: 80,
            height: 40,
        },
    );
    let modifiers = KeyModifiers::default();

    ui.dispatch_raw(&[
        raw(root_id, RawEvent::press(5, 5, modifiers)),
        raw(root_id, RawEvent::release(5, 5, modifiers)),
    ]);

    assert_eq!(seen(), ["click"]);
}

#[test]
pub(super) fn an_inner_scroll_container_wins_over_the_root() {
    let (list, first_id) = scrollable_list();
    let root = Widget::column(vec![list]).with_layout(fixed(80, 100));
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble_laid_out(
        root,
        vec![scroll(&component)],
        Size {
            width: 80,
            height: 100,
        },
    );
    ui.tree.borrow_mut().set_root_scroll(true);

    ui.dispatch_raw(&[raw(first_id, RawEvent::wheel(40))]);

    ui.tree
        .borrow_mut()
        .layout(
            &mut TextEngine::new(),
            Size {
                width: 80,
                height: 100,
            },
            1.0,
        )
        .expect("relayout");
    let mut tree = ui.tree.borrow_mut();
    let snapshot = tree.snapshot().expect("snapshot");
    assert!(snapshot.node(first_id).expect("first").y < 0);
}

#[test]
pub(super) fn a_list_inside_a_modal_layer_still_scrolls() {
    let (list, first_id) = scrollable_list();
    let dialog = Widget::layer(vec![list]).modal(true);
    let root = Widget::column(vec![dialog]).with_layout(fixed(80, 100));
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble_laid_out(
        root,
        vec![scroll(&component)],
        Size {
            width: 80,
            height: 100,
        },
    );

    ui.dispatch_raw(&[raw(first_id, RawEvent::wheel(40))]);

    ui.tree
        .borrow_mut()
        .layout(
            &mut TextEngine::new(),
            Size {
                width: 80,
                height: 100,
            },
            1.0,
        )
        .expect("relayout");
    let mut tree = ui.tree.borrow_mut();
    let snapshot = tree.snapshot().expect("snapshot");
    assert!(snapshot.node(first_id).expect("first").y < 0);
}

#[test]
pub(super) fn wheel_reaches_content_through_a_plain_layer() {
    let (list, first_id) = scrollable_list();
    let card = Widget::text("dialog").with_layout(fixed(20, 20));
    let root = Widget::column(vec![list, Widget::layer(vec![card])]).with_layout(fixed(80, 100));
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble_laid_out(
        root,
        vec![scroll(&component)],
        Size {
            width: 80,
            height: 100,
        },
    );

    let target = ui
        .tree
        .borrow()
        .hit(60, 60)
        .expect("hit")
        .map_or(EventTarget::Window, EventTarget::Widget);
    ui.dispatch_raw(&[RawUiEvent::at(target, RawEvent::wheel(40))]);

    ui.tree
        .borrow_mut()
        .layout(
            &mut TextEngine::new(),
            Size {
                width: 80,
                height: 100,
            },
            1.0,
        )
        .expect("relayout");
    let mut tree = ui.tree.borrow_mut();
    let snapshot = tree.snapshot().expect("snapshot");
    assert!(snapshot.node(first_id).expect("first").y < 0);
}
