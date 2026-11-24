use std::cell::RefCell;
use std::rc::Rc;

use crate::policy::{
    __brul_attach_keyboard_apply, __brul_attach_keyboard_cleanup, __brul_attach_keyboard_raw_event,
};
use crate::ui::AssemblyParts;
use crate::{
    Build, Built, Component, ComponentHandle, Dispatch, Dyn, Event, EventKind, EventSet,
    EventTarget, Hooks, RawEvent, RawEventSet, RawKind, RawUiEvent, StateId, TestTheme, Ui,
    UiEvent, ViewError,
    gui::{KeyModifiers, KeyPress, Widget},
};

thread_local! {
    static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn seen() -> Vec<String> {
    SEEN.with(|seen| seen.borrow().clone())
}

fn record_event(_build: &mut Build, _state: StateId, event: &UiEvent) -> Dispatch {
    match &event.event {
        Event::Key { key, .. } => {
            SEEN.with(|seen| seen.borrow_mut().push(format!("key:{}", key.code)))
        }
        Event::Click { .. } => SEEN.with(|seen| seen.borrow_mut().push("click".into())),
        Event::Focus { .. } => SEEN.with(|seen| seen.borrow_mut().push("focus".into())),
        Event::Blur { .. } => SEEN.with(|seen| seen.borrow_mut().push("blur".into())),
        Event::Dismiss { .. } => SEEN.with(|seen| seen.borrow_mut().push("dismiss".into())),
        _ => {}
    }
    Dispatch::CONTINUE
}

fn ignore_event(_build: &mut Build, _state: StateId, _event: &UiEvent) -> Dispatch {
    Dispatch::CONTINUE
}

fn ignore_raw(_build: &mut Build, _state: StateId, _event: &RawUiEvent) -> Dispatch {
    Dispatch::CONTINUE
}

fn noop_apply(_build: &mut Build, _state: StateId, _params: &[Dyn]) -> Result<(), ViewError> {
    Ok(())
}

fn noop_cleanup(_build: &mut Build, _state: StateId) {}

fn mark_handled(_build: &mut Build, _state: StateId, _event: &RawUiEvent) -> Dispatch {
    Dispatch::handled()
}

fn observer(component: &ComponentHandle) -> Hooks {
    Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "observer"),
        component: Rc::clone(component),
        params: Vec::new(),
        apply: noop_apply,
        event: record_event,
        raw_event: ignore_raw,
        cleanup: noop_cleanup,
        events: EventSet::from([
            EventKind::Key,
            EventKind::Click,
            EventKind::Focus,
            EventKind::Blur,
            EventKind::Dismiss,
        ]),
        raw_events: RawEventSet::empty(),
    }
}

fn keyboard(component: &ComponentHandle) -> Hooks {
    Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "keyboard"),
        component: Rc::clone(component),
        params: Vec::new(),
        apply: __brul_attach_keyboard_apply,
        event: ignore_event,
        raw_event: __brul_attach_keyboard_raw_event,
        cleanup: __brul_attach_keyboard_cleanup,
        events: EventSet::empty(),
        raw_events: RawEventSet::from([RawKind::Key]),
    }
}

fn first_handled(component: &ComponentHandle) -> Hooks {
    Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "first"),
        component: Rc::clone(component),
        params: Vec::new(),
        apply: noop_apply,
        event: ignore_event,
        raw_event: mark_handled,
        cleanup: noop_cleanup,
        events: EventSet::empty(),
        raw_events: RawEventSet::from([RawKind::Key]),
    }
}

fn view(
    _theme: Option<TestTheme>,
    _build: &mut Build,
    _component: &ComponentHandle,
) -> Result<Built, ViewError> {
    Ok(Built {
        widget: Widget::text("view"),
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

fn assemble(widget: Widget, capabilities: Vec<Hooks>) -> Ui<TestTheme> {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let built = Built {
        widget,
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    };
    let mut ui = Ui::assemble_with(
        Rc::clone(&component),
        built,
        view,
        None,
        AssemblyParts {
            structures: Vec::new(),
            capabilities,
            nested: Vec::new(),
            states: Rc::new(RefCell::new(crate::States::default())),
            event_sink: None,
        },
        Rc::new(RefCell::new(Vec::new())),
        false,
    );
    ui.settle();
    SEEN.with(|seen| seen.borrow_mut().clear());
    ui
}

fn key(code: &str) -> RawUiEvent {
    RawUiEvent::window(RawEvent::key(KeyPress::from(code), KeyModifiers::default()))
}

#[test]
fn tab_moves_focus_and_emits_blur_before_focus() {
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
fn enter_clicks_the_focused_widget_and_its_ancestors() {
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
fn escape_in_a_modal_layer_dismisses_it() {
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
fn other_keys_become_semantic_key_events() {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(
        Widget::text("view"),
        vec![keyboard(&component), observer(&component)],
    );

    ui.dispatch_raw(&[key("a")]);

    assert_eq!(seen(), ["key:a"]);
}

#[test]
fn a_handled_key_is_left_to_the_other_policies() {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(
        Widget::text("view"),
        vec![
            first_handled(&component),
            keyboard(&component),
            observer(&component),
        ],
    );

    ui.dispatch_raw(&[key("a")]);

    assert!(seen().is_empty());
}
