use std::cell::RefCell;
use std::rc::Rc;

use brul_codegen::{POLICY_NAMES, policy_events};

use crate::policy::{
    __brul_attach_editor_apply, __brul_attach_editor_cleanup, __brul_attach_editor_raw_event,
    __brul_attach_keyboard_apply, __brul_attach_keyboard_cleanup, __brul_attach_keyboard_raw_event,
    __brul_attach_pointer_apply, __brul_attach_pointer_cleanup, __brul_attach_pointer_raw_event,
    __brul_attach_scroll_apply, __brul_attach_scroll_cleanup, __brul_attach_scroll_raw_event,
};
use crate::ui::AssemblyParts;
use crate::{
    Build, Built, Component, ComponentHandle, Dispatch, Dyn, Event, EventKind, EventSet,
    EventTarget, Hooks, RawEvent, RawEventSet, RawKind, RawUiEvent, StateId, TestTheme, Ui,
    UiEvent, ViewError,
    gui::{
        Dimension, ImeKind, KeyModifiers, KeyPress, LayoutStyle, Size, StyleSet, TextEngine,
        Widget, WidgetId, WidgetStyle,
    },
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
    let label = match &event.event {
        RawEvent::KeyNoop { key, .. } => Some(format!("noop:{}", key.code)),
        RawEvent::Paste { text } if text.is_empty() => Some("paste_request".to_owned()),
        RawEvent::Paste { text } => Some(format!("paste:{text}")),
        RawEvent::Ime { kind } => Some(format!("ime:{}", ime_label(kind))),
        _ => None,
    };
    if let Some(label) = label {
        SEEN.with(|seen| seen.borrow_mut().push(label));
    }
    Dispatch::CONTINUE
}

fn ime_label(kind: &ImeKind) -> &'static str {
    match kind {
        ImeKind::Enabled => "enabled",
        ImeKind::Preedit(_) => "preedit",
        ImeKind::Commit(_) => "commit",
        ImeKind::DeleteSurrounding { .. } => "delete_surrounding",
        ImeKind::Disabled => "disabled",
    }
}

#[derive(Default)]
struct ClipboardBridge {
    text: Option<String>,
}

fn clipboard_state(build: &mut Build) -> Rc<RefCell<ClipboardBridge>> {
    if build.get_state::<ClipboardBridge>().is_none() {
        build.set_state(ClipboardBridge::default());
    }
    build.get_state().expect("clipboard state")
}

fn record_copy(build: &mut Build, _state: StateId, event: &UiEvent) -> Dispatch {
    if let Event::Copy { text } = &event.event {
        clipboard_state(build).borrow_mut().text = Some(text.clone());
    }
    Dispatch::CONTINUE
}

fn answer_paste(build: &mut Build, _state: StateId, event: &RawUiEvent) -> Dispatch {
    let RawEvent::Paste { text } = &event.event else {
        return Dispatch::CONTINUE;
    };
    if !text.is_empty() {
        return Dispatch::CONTINUE;
    }
    let Some(text) = clipboard_state(build).borrow().text.clone() else {
        return Dispatch::CONTINUE;
    };
    build.emit_raw_event(RawUiEvent::at(event.target, RawEvent::paste(text)));
    Dispatch::handled()
}

fn clipboard(component: &ComponentHandle) -> Hooks {
    Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "clipboard"),
        component: Rc::clone(component),
        params: Vec::new(),
        apply: noop_apply,
        event: record_copy,
        raw_event: answer_paste,
        cleanup: noop_cleanup,
        events: EventSet::from([EventKind::Copy]),
        raw_events: RawEventSet::from([RawKind::Paste]),
    }
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
    let events = policy_events("keyboard");
    Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "keyboard"),
        component: Rc::clone(component),
        params: Vec::new(),
        apply: __brul_attach_keyboard_apply,
        event: ignore_event,
        raw_event: __brul_attach_keyboard_raw_event,
        cleanup: __brul_attach_keyboard_cleanup,
        events: EventSet::from_iter(events.semantic),
        raw_events: RawEventSet::from_iter(events.raw),
    }
}

fn editor(component: &ComponentHandle) -> Hooks {
    let events = policy_events("editor");
    Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "editor"),
        component: Rc::clone(component),
        params: Vec::new(),
        apply: __brul_attach_editor_apply,
        event: ignore_event,
        raw_event: __brul_attach_editor_raw_event,
        cleanup: __brul_attach_editor_cleanup,
        events: EventSet::from_iter(events.semantic),
        raw_events: RawEventSet::from_iter(events.raw),
    }
}

fn pointer(component: &ComponentHandle) -> Hooks {
    let events = policy_events("pointer");
    Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "pointer"),
        component: Rc::clone(component),
        params: Vec::new(),
        apply: __brul_attach_pointer_apply,
        event: ignore_event,
        raw_event: __brul_attach_pointer_raw_event,
        cleanup: __brul_attach_pointer_cleanup,
        events: EventSet::from_iter(events.semantic),
        raw_events: RawEventSet::from_iter(events.raw),
    }
}

fn scroll(component: &ComponentHandle) -> Hooks {
    let events = policy_events("scroll");
    Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "scroll"),
        component: Rc::clone(component),
        params: Vec::new(),
        apply: __brul_attach_scroll_apply,
        event: ignore_event,
        raw_event: __brul_attach_scroll_raw_event,
        cleanup: __brul_attach_scroll_cleanup,
        events: EventSet::from_iter(events.semantic),
        raw_events: RawEventSet::from_iter(events.raw),
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
        raw_events: RawEventSet::from([RawKind::Key, RawKind::Ime, RawKind::Paste]),
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

fn fixed(width: u32, height: u32) -> LayoutStyle {
    LayoutStyle {
        width: Dimension::Px(width),
        height: Dimension::Px(height),
        ..LayoutStyle::default()
    }
}

fn assemble_laid_out(widget: Widget, capabilities: Vec<Hooks>, size: Size) -> Ui<TestTheme> {
    let ui = assemble(widget, capabilities);
    ui.tree
        .borrow_mut()
        .layout(&mut TextEngine::new(), size, 1.0)
        .expect("layout");
    SEEN.with(|seen| seen.borrow_mut().clear());
    ui
}

fn raw(id: WidgetId, event: RawEvent) -> RawUiEvent {
    RawUiEvent::widget(id, event)
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

fn paste(id: WidgetId, text: &str) -> RawUiEvent {
    raw(id, RawEvent::paste(text.to_owned()))
}

fn ime(id: WidgetId, kind: ImeKind) -> RawUiEvent {
    raw(id, RawEvent::ime(kind))
}

fn seed_clipboard(ui: &Ui<TestTheme>, text: &str) {
    let state = ui.states.borrow_mut().get::<RefCell<ClipboardBridge>>(
        StateId::source("test", "clipboard"),
        &ui.component,
        || RefCell::new(ClipboardBridge::default()),
    );
    state.borrow_mut().text = Some(text.to_owned());
}

fn editor_ui(text: &str, wrap: bool) -> (Ui<TestTheme>, WidgetId) {
    let input = Widget::input(text).focusable(true).wrap(wrap);
    let id = input.id();
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let ui = assemble(
        input,
        vec![
            editor(&component),
            clipboard(&component),
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

fn preedit_of(ui: &Ui<TestTheme>, id: WidgetId) -> String {
    ui.tree
        .borrow()
        .edit_buffer(id)
        .expect("input")
        .preedit()
        .to_owned()
}

#[path = "tests/editor.rs"]
mod editor;
#[path = "tests/keyboard.rs"]
mod keyboard;
#[path = "tests/pointer_scroll.rs"]
mod pointer_scroll;
