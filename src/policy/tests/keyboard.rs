use super::*;
#[test]
pub(super) fn tab_moves_focus_and_emits_blur_before_focus() {
    let first = Widget::text("first").focusable(true);
    let first_id = first.id();
    let second = Widget::text("second").focusable(true);
    let second_id = second.id();
    let root = Widget::column(vec![first, second]);
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(root, vec![keyboard(&component), observer(&component)]);
    ui.tree.borrow_mut().set_focused(Some(first_id));

    ui.dispatch_raw(&[key("Tab")]);

    assert_eq!(ui.tree.borrow().focused_id(), Some(second_id));
    assert_eq!(seen(), ["blur", "focus"]);
}

#[test]
pub(super) fn enter_clicks_the_focused_widget_and_its_ancestors() {
    let focused = Widget::text("focused").focusable(true);
    let focused_id = focused.id();
    let root = Widget::column(vec![focused]);
    let root_id = root.id();
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(root, vec![keyboard(&component), observer(&component)]);
    ui.tree.borrow_mut().set_focused(Some(focused_id));

    ui.dispatch_raw(&[key("Enter")]);

    assert_eq!(seen(), ["click", "click"]);
    assert!(!seen().iter().any(|event| event == "key:Enter"));
    assert_eq!(ui.tree.borrow().focused_id(), Some(focused_id));
    assert_ne!(focused_id, root_id);
}

#[test]
pub(super) fn escape_in_a_modal_layer_dismisses_it() {
    let child = Widget::text("child").focusable(true);
    let modal = Widget::layer(vec![child]).modal(true);
    let modal_id = modal.id();
    let root = Widget::column(vec![modal]);
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(root, vec![keyboard(&component), observer(&component)]);
    ui.tree.borrow_mut().set_focused(Some(modal_id));

    ui.dispatch_raw(&[key("Escape")]);

    assert_eq!(seen(), ["dismiss"]);
}

#[test]
pub(super) fn other_keys_become_semantic_key_events() {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(
        Widget::text("view"),
        vec![keyboard(&component), observer(&component)],
    );

    ui.dispatch_raw(&[key("a")]);

    assert_eq!(seen(), ["key:a"]);
}

#[test]
pub(super) fn a_focused_input_gets_a_semantic_key_when_no_policy_handled_it() {
    let input = Widget::input("").focusable(true);
    let input_id = input.id();
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(input, vec![keyboard(&component), observer(&component)]);
    ui.tree.borrow_mut().set_focused(Some(input_id));

    ui.dispatch_raw(&[key("a")]);
    assert_eq!(seen(), ["key:a"]);

    SEEN.with(|seen| seen.borrow_mut().clear());
    ui.dispatch_raw(&[key_noop("a")]);
    assert_eq!(seen(), ["key:a"]);
}

#[test]
pub(super) fn a_handled_key_is_left_to_the_other_policies() {
    let input = Widget::input("").focusable(true);
    let input_id = input.id();
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(
        input,
        vec![
            first_handled(&component),
            keyboard(&component),
            observer(&component),
        ],
    );
    ui.tree.borrow_mut().set_focused(Some(input_id));

    ui.dispatch_raw(&[key("a")]);
    assert!(seen().is_empty());

    SEEN.with(|seen| seen.borrow_mut().clear());
    ui.dispatch_raw(&[key_noop("a")]);
    assert!(seen().is_empty());
}

#[test]
pub(super) fn space_clicks_the_focused_widget_and_its_ancestors() {
    let focused = Widget::text("focused").focusable(true);
    let focused_id = focused.id();
    let root = Widget::column(vec![focused]);
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(root, vec![keyboard(&component), observer(&component)]);
    ui.tree.borrow_mut().set_focused(Some(focused_id));

    ui.dispatch_raw(&[key(" ")]);

    assert_eq!(seen(), ["click", "click"]);
    assert_eq!(ui.tree.borrow().focused_id(), Some(focused_id));
}

#[test]
pub(super) fn tab_skips_a_disabled_widget() {
    let first = Widget::text("first").focusable(true);
    let first_id = first.id();
    let disabled = Widget::text("disabled").focusable(true).disabled(true);
    let third = Widget::text("third").focusable(true);
    let third_id = third.id();
    let root = Widget::column(vec![first, disabled, third]);
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(root, vec![keyboard(&component), observer(&component)]);
    ui.tree.borrow_mut().set_focused(Some(first_id));

    ui.dispatch_raw(&[key("Tab")]);

    assert_eq!(ui.tree.borrow().focused_id(), Some(third_id));
    assert_eq!(seen(), ["blur", "focus"]);
}

#[test]
pub(super) fn tab_stays_inside_a_visible_modal_layer() {
    let child = Widget::text("child").focusable(true);
    let child_id = child.id();
    let root = Widget::column(vec![Widget::layer(vec![child]).modal(true)]);
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(root, vec![keyboard(&component), observer(&component)]);
    ui.tree.borrow_mut().set_focused(Some(child_id));

    ui.dispatch_raw(&[key("Tab")]);
    ui.dispatch_raw(&[key_at(child_id, "Tab", shift())]);

    assert_eq!(ui.tree.borrow().focused_id(), Some(child_id));
    assert!(seen().is_empty());
}

#[test]
pub(super) fn tab_returns_focus_when_the_modal_layer_is_hidden() {
    let trigger = Widget::text("trigger").focusable(true);
    let trigger_id = trigger.id();
    let child = Widget::text("child").focusable(true);
    let child_id = child.id();
    let dialog = Widget::layer(vec![child]).modal(true).visible(false);
    let dialog_id = dialog.id();
    let root = Widget::column(vec![trigger, dialog]);
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(root, vec![keyboard(&component), observer(&component)]);
    ui.tree.borrow_mut().set_focused(Some(trigger_id));
    assert!(ui.tree.borrow_mut().set_visible(dialog_id, true));

    ui.dispatch_raw(&[key("Tab")]);
    assert_eq!(ui.tree.borrow().focused_id(), Some(child_id));
    SEEN.with(|seen| seen.borrow_mut().clear());

    assert!(ui.tree.borrow_mut().set_visible(dialog_id, false));
    assert_eq!(ui.tree.borrow().focused_id(), Some(trigger_id));
}

#[test]
pub(super) fn tab_without_focusables_changes_nothing() {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(
        Widget::text("view"),
        vec![keyboard(&component), observer(&component)],
    );

    ui.dispatch_raw(&[key("Tab")]);

    assert_eq!(ui.tree.borrow().focused_id(), None);
    assert!(seen().is_empty());
}
#[test]
pub(super) fn shift_tab_moves_focus_back() {
    let first = Widget::text("first").focusable(true);
    let first_id = first.id();
    let second = Widget::text("second").focusable(true);
    let second_id = second.id();
    let root = Widget::column(vec![first, second]);
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(root, vec![keyboard(&component), observer(&component)]);
    ui.tree.borrow_mut().set_focused(Some(second_id));

    ui.dispatch_raw(&[key_at(second_id, "Tab", shift())]);

    assert_eq!(ui.tree.borrow().focused_id(), Some(first_id));
    assert_eq!(seen(), ["blur", "focus"]);
}
