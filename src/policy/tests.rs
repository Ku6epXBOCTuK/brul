use std::cell::RefCell;
use std::rc::Rc;

use crate::policy::{
    __brul_attach_editor_apply, __brul_attach_editor_cleanup, __brul_attach_editor_raw_event,
    __brul_attach_keyboard_apply, __brul_attach_keyboard_cleanup, __brul_attach_keyboard_raw_event,
};
use crate::ui::AssemblyParts;
use crate::{
    Build, Built, Component, ComponentHandle, Dispatch, Dyn, Event, EventKind, EventSet,
    EventTarget, Hooks, RawEvent, RawEventSet, RawKind, RawUiEvent, StateId, TestTheme, Ui,
    UiEvent, ViewError,
    gui::{KeyModifiers, KeyPress, Size, TextEngine, Widget, WidgetId},
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
        Event::Input { value, caret } => {
            SEEN.with(|seen| seen.borrow_mut().push(format!("input:{value}@{caret}")))
        }
        Event::Copy { text } => SEEN.with(|seen| seen.borrow_mut().push(format!("copy:{text}"))),
        Event::Submit { .. } => SEEN.with(|seen| seen.borrow_mut().push("submit".into())),
        _ => {}
    }
    Dispatch::CONTINUE
}

fn record_raw(_build: &mut Build, _state: StateId, event: &RawUiEvent) -> Dispatch {
    match &event.event {
        RawEvent::KeyNoop { key, .. } => {
            SEEN.with(|seen| seen.borrow_mut().push(format!("noop:{}", key.code)))
        }
        RawEvent::Paste { .. } => SEEN.with(|seen| seen.borrow_mut().push("paste_request".into())),
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
            EventKind::Input,
            EventKind::Copy,
            EventKind::Submit,
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

fn editor(component: &ComponentHandle) -> Hooks {
    Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "editor"),
        component: Rc::clone(component),
        params: Vec::new(),
        apply: __brul_attach_editor_apply,
        event: ignore_event,
        raw_event: __brul_attach_editor_raw_event,
        cleanup: __brul_attach_editor_cleanup,
        events: EventSet::empty(),
        raw_events: RawEventSet::from([RawKind::Key]),
    }
}

fn raw_observer(component: &ComponentHandle) -> Hooks {
    Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "raw_observer"),
        component: Rc::clone(component),
        params: Vec::new(),
        apply: noop_apply,
        event: ignore_event,
        raw_event: record_raw,
        cleanup: noop_cleanup,
        events: EventSet::empty(),
        raw_events: RawEventSet::from([RawKind::Key, RawKind::Paste]),
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

fn key_noop(code: &str) -> RawUiEvent {
    RawUiEvent::window(RawEvent::key_noop(
        KeyPress::from(code),
        KeyModifiers::default(),
    ))
}

fn editor_ui(text: &str, wrap: bool) -> (Ui<TestTheme>, WidgetId) {
    let input = Widget::input(text).focusable(true).wrap(wrap);
    let id = input.id();
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let ui = assemble(
        input,
        vec![
            editor(&component),
            keyboard(&component),
            observer(&component),
            raw_observer(&component),
        ],
    );
    {
        let mut tree = ui.tree.borrow_mut();
        let mut engine = TextEngine::new();
        tree.layout(
            &mut engine,
            Size {
                width: 200,
                height: 100,
            },
            1.0,
        )
        .expect("layout");
        tree.set_focused(Some(id));
    }
    SEEN.with(|seen| seen.borrow_mut().clear());
    (ui, id)
}

fn key_at(id: WidgetId, code: &str, modifiers: KeyModifiers) -> RawUiEvent {
    RawUiEvent::widget(id, RawEvent::key(KeyPress::from(code), modifiers))
}

fn control() -> KeyModifiers {
    KeyModifiers {
        control: true,
        ..KeyModifiers::default()
    }
}

fn shift() -> KeyModifiers {
    KeyModifiers {
        shift: true,
        ..KeyModifiers::default()
    }
}

fn text_of(ui: &Ui<TestTheme>, id: WidgetId) -> String {
    ui.tree
        .borrow()
        .edit_buffer(id)
        .expect("input")
        .text()
        .to_owned()
}

fn caret_of(ui: &Ui<TestTheme>, id: WidgetId) -> usize {
    ui.tree
        .borrow()
        .edit_buffer(id)
        .expect("input")
        .caret()
        .offset
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
fn a_focused_input_gets_a_semantic_key_when_no_policy_handled_it() {
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
fn a_handled_key_is_left_to_the_other_policies() {
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
fn typing_inserts_at_the_caret_and_emits_input() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "X", KeyModifiers::default())]);

    assert_eq!(text_of(&ui, id), "abX");
    assert_eq!(caret_of(&ui, id), 3);
    assert_eq!(seen(), ["input:abX@3"]);
}

#[test]
fn backspace_reports_a_noop_when_the_field_is_empty() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "Backspace", KeyModifiers::default())]);
    assert_eq!(text_of(&ui, id), "a");
    assert_eq!(seen(), ["input:a@1"]);

    SEEN.with(|seen| seen.borrow_mut().clear());
    ui.dispatch_raw(&[key_at(id, "Backspace", KeyModifiers::default())]);
    assert_eq!(text_of(&ui, id), "");
    assert_eq!(seen(), ["input:@0"]);

    SEEN.with(|seen| seen.borrow_mut().clear());
    ui.dispatch_raw(&[key_at(id, "Backspace", KeyModifiers::default())]);
    assert_eq!(seen(), ["noop:Backspace", "key:Backspace"]);
}

#[test]
fn control_a_then_typing_replaces_the_selection() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "a", control())]);
    ui.dispatch_raw(&[key_at(id, "Z", KeyModifiers::default())]);

    assert_eq!(text_of(&ui, id), "Z");
    assert_eq!(seen(), ["input:Z@1"]);
}

#[test]
fn control_c_copies_only_a_selection() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "a", control())]);
    ui.dispatch_raw(&[key_at(id, "c", control())]);
    assert_eq!(seen(), ["copy:ab"]);

    ui.dispatch_raw(&[key_at(id, "ArrowRight", KeyModifiers::default())]);
    SEEN.with(|seen| seen.borrow_mut().clear());
    ui.dispatch_raw(&[key_at(id, "c", control())]);
    assert!(seen().is_empty());
}

#[test]
fn control_x_emits_input_before_copy() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "a", control())]);
    ui.dispatch_raw(&[key_at(id, "x", control())]);

    assert_eq!(text_of(&ui, id), "");
    assert_eq!(seen(), ["input:@0", "copy:ab"]);
}

#[test]
fn control_v_asks_for_the_clipboard() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "v", control())]);

    assert_eq!(seen(), ["paste_request"]);
}

#[test]
fn enter_submits_a_single_line_and_inserts_a_newline_when_wrapped() {
    let (mut ui, id) = editor_ui("a", false);
    ui.dispatch_raw(&[key_at(id, "Enter", KeyModifiers::default())]);
    assert_eq!(seen(), ["submit"]);

    let (mut ui, id) = editor_ui("a", true);
    ui.dispatch_raw(&[key_at(id, "Enter", KeyModifiers::default())]);
    assert_eq!(text_of(&ui, id), "a\n");
    assert_eq!(seen(), ["input:a\n@2".to_string()]);
}

#[test]
fn arrows_move_the_caret_and_extend_the_selection() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "ArrowLeft", KeyModifiers::default())]);
    assert_eq!(caret_of(&ui, id), 1);
    ui.dispatch_raw(&[key_at(id, "ArrowRight", shift())]);
    let selection = ui.tree.borrow().edit_buffer(id).expect("input").selection();
    assert_eq!((selection.start.offset, selection.end.offset), (1, 2));
    assert!(seen().is_empty());
}

#[test]
fn unconsumed_keys_reach_the_keyboard_policy() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "ArrowUp", KeyModifiers::default())]);

    assert_eq!(seen(), ["key:ArrowUp"]);
}

#[test]
fn a_handled_key_skips_the_editor() {
    let input = Widget::input("ab").focusable(true);
    let id = input.id();
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(
        input,
        vec![
            first_handled(&component),
            editor(&component),
            keyboard(&component),
            observer(&component),
        ],
    );
    {
        let mut tree = ui.tree.borrow_mut();
        let mut engine = TextEngine::new();
        tree.layout(
            &mut engine,
            Size {
                width: 200,
                height: 100,
            },
            1.0,
        )
        .expect("layout");
        tree.set_focused(Some(id));
    }
    SEEN.with(|seen| seen.borrow_mut().clear());

    ui.dispatch_raw(&[key_at(id, "X", KeyModifiers::default())]);

    assert_eq!(text_of(&ui, id), "ab");
    assert!(seen().is_empty());
}
