use super::*;
use crate::Object;

#[test]
pub(super) fn a_capability_gets_the_clipboard_from_the_ui() {
    struct MemoryClipboard;

    impl Clipboard for MemoryClipboard {
        fn read(&self) -> Option<crate::gui::ClipboardData> {
            Some(crate::gui::ClipboardData::from(String::from("copied")))
        }

        fn write(&self, _data: &crate::gui::ClipboardData) {}
    }

    fn see_clipboard(ctx: &CapabilityContext, _: &[Dyn]) -> Result<(), ViewError> {
        ctx.component()
            .borrow_mut()
            .assign("seen", Value::Bool(ctx.clipboard().is_some()))
            .expect("seen");
        Ok(())
    }

    let component = component_with(&["seen"]);
    let hooks = Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "clipboard-capability"),
        component: Rc::clone(&component),
        params: Vec::new(),
        apply: see_clipboard,
        event: |_, _| Dispatch::CONTINUE,
        raw_event: |_, _| Dispatch::CONTINUE,
        cleanup: |_| {},
        events: EventSet::empty(),
        raw_events: RawEventSet::empty(),
    };
    let mut ui = Ui::assemble_with(
        Rc::clone(&component),
        Built {
            widget: Widget::text(""),
            bindings: Vec::new(),
            handlers: Vec::new(),
            props: Vec::new(),
        },
        themed_view,
        None,
        AssemblyParts {
            structures: Vec::new(),
            capabilities: vec![hooks],
            nested: Vec::new(),
            states: Rc::new(RefCell::new(crate::States::default())),
            event_sink: None,
            clipboard: Some(Rc::new(MemoryClipboard)),
        },
        Rc::new(RefCell::new(Vec::new())),
        false,
    );
    ui.settle();
    assert_eq!(component.borrow().read("seen"), Ok(Value::Bool(true)));
}

#[test]
pub(super) fn dispatch_calls_the_handler_of_the_widget() {
    let component = component_with(&["ran"]);
    let (id, mut ui) = dispatching(&component, Widget::text(""), |id| {
        vec![Handler::new(
            &component,
            Some(id),
            EventKind::Click,
            set_ran,
        )]
    });
    ui.dispatch(&[DispatchEvent::semantic(id, Event::click())]);
    assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
}

pub(super) struct ProbeSource {
    value: Signal<String>,
}

#[derive(Default)]
pub(super) struct ProbeState {
    applied: Vec<String>,
}

pub(super) fn probe_apply(ctx: &CapabilityContext, params: &[Dyn]) -> Result<(), ViewError> {
    if ctx.get_state::<ProbeState>().is_none() {
        ctx.set_state(ProbeState::default());
    }
    let value = params
        .first()
        .and_then(|param| String::try_from(param.clone()).ok())
        .unwrap_or_default();
    if let Some(probe) = ctx.get_state::<ProbeState>() {
        probe.borrow_mut().applied.push(value);
    }
    Ok(())
}

pub(super) fn probe_event(ctx: &CapabilityContext, event: &UiEvent) -> Dispatch {
    if matches!(event.event, Event::Click { .. }) {
        ctx.component()
            .borrow_mut()
            .assign("ran", Value::Bool(true))
            .expect("ran");
        Dispatch::stop()
    } else {
        Dispatch::CONTINUE
    }
}

pub(super) fn ignore_raw_event(_ctx: &CapabilityContext, _event: &RawUiEvent) -> Dispatch {
    Dispatch::CONTINUE
}

pub(super) fn probe_cleanup(ctx: &CapabilityContext) {
    ctx.component()
        .borrow_mut()
        .assign("cleaned", Value::Bool(true))
        .expect("cleaned");
}

pub(super) fn set_script(component: &mut Component, _: &[Dyn]) -> Result<Dyn, Error> {
    component.assign("script", Value::Bool(true))?;
    Ok(Dyn::null())
}

#[test]
pub(super) fn attachment_reruns_vetoes_events_and_cleans_up() {
    let component = component_with(&["ran", "script", "cleaned"]);
    component.borrow_mut().set_state(ProbeSource {
        value: Signal::new("first".to_owned()),
    });
    let widget = Widget::text("");
    let id = widget.id();
    let hooks = Hooks {
        target: EventTarget::Widget(id),
        state: StateId::source("test", "seven"),
        component: Rc::clone(&component),
        params: vec![|component: &mut Component| -> Result<Dyn, Error> {
            Ok(Dyn::text(component.state::<ProbeSource>().value.get()))
        }],
        apply: probe_apply,
        event: probe_event,
        raw_event: ignore_raw_event,
        cleanup: probe_cleanup,
        events: EventSet::from([EventKind::Click]),
        raw_events: RawEventSet::empty(),
    };
    let built = Built {
        widget,
        bindings: Vec::new(),
        handlers: vec![Handler::new(
            &component,
            Some(id),
            EventKind::Click,
            set_script,
        )],
        props: Vec::new(),
    };
    let mut ui = Ui::assemble_with(
        Rc::clone(&component),
        built,
        themed_view,
        None,
        AssemblyParts {
            structures: Vec::new(),
            capabilities: vec![hooks],
            nested: Vec::new(),
            states: Rc::new(RefCell::new(crate::States::default())),
            event_sink: None,
            clipboard: None,
        },
        Rc::new(RefCell::new(Vec::new())),
        false,
    );
    ui.settle();
    let state = StateId::source("test", "seven").for_widget(id);
    let probe = ui
        .states
        .borrow()
        .get_cloned::<RefCell<ProbeState>>(state)
        .expect("probe state");
    assert_eq!(probe.borrow().applied, ["first".to_owned()]);

    component
        .borrow_mut()
        .state::<ProbeSource>()
        .value
        .set("second".to_owned());
    brul_reactive::flush();
    ui.settle();
    assert_eq!(
        probe.borrow().applied,
        ["first".to_owned(), "second".to_owned()]
    );

    ui.dispatch(&[DispatchEvent::semantic(id, Event::click())]);
    assert_eq!(component.borrow().read("ran"), Ok(Value::Bool(true)));
    assert_ne!(component.borrow().read("script"), Ok(Value::Bool(true)));

    drop(ui);
    assert_eq!(component.borrow().read("cleaned"), Ok(Value::Bool(true)));
}

#[test]
pub(super) fn dispatch_runs_the_first_of_two_handlers_of_a_widget() {
    let component = component_with(&["ran", "off"]);
    let (id, mut ui) = dispatching(&component, Widget::text(""), |id| {
        vec![
            Handler::new(&component, Some(id), EventKind::Click, set_ran),
            Handler::new(&component, Some(id), EventKind::Click, set_off),
        ]
    });

    ui.dispatch(&[DispatchEvent::semantic(id, Event::click())]);
    assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
    assert_ne!(ui.component.borrow().read("off"), Ok(Value::Bool(true)));
}

#[test]
pub(super) fn dispatch_passes_the_key_code() {
    let component = component_with(&["seen"]);
    let (_, mut ui) = dispatching(&component, Widget::text(""), |_| {
        vec![Handler::new(&component, None, EventKind::Key, key_code)]
    });
    ui.dispatch(&[UiEvent::window(Event::key(
        brul_gui::KeyPress::from("Enter"),
        brul_gui::KeyModifiers::default(),
    ))
    .into()]);
    let seen = ui.component.borrow().read("seen").expect("seen");
    assert_eq!(
        seen,
        Value::Object(
            Object::builder()
                .field("code", Value::String("Enter".into()))
                .field("physical", Value::Null)
                .field("ctrl", Value::Bool(false))
                .field("shift", Value::Bool(false))
                .field("alt", Value::Bool(false))
                .field("meta", Value::Bool(false))
                .field("target", Value::Null)
                .into()
        )
    );
}

#[test]
pub(super) fn dispatch_passes_a_key_to_the_widget_then_the_window() {
    let component = component_with(&["seen", "ran"]);
    let (id, mut ui) = dispatching(&component, Widget::text(""), |id| {
        vec![
            Handler::new(&component, Some(id), EventKind::Key, key_code),
            Handler::new(&component, None, EventKind::Key, set_ran),
        ]
    });
    ui.dispatch(&[DispatchEvent::semantic(
        id,
        Event::key(
            brul_gui::KeyPress::from("Escape"),
            brul_gui::KeyModifiers::default(),
        ),
    )]);
    let seen = ui.component.borrow().read("seen").expect("seen");
    let Value::Object(object) = seen else {
        panic!("key event is not an object");
    };
    assert_eq!(
        object.get("target"),
        Some(&Value::Number(id.get() as f64)),
        "widget target is preserved"
    );
    assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
}

#[test]
pub(super) fn dispatch_reaches_the_window_when_the_widget_is_gone() {
    let component = component_with(&["ran"]);
    let (_, mut ui) = dispatching(&component, Widget::text(""), |_| {
        vec![Handler::new(&component, None, EventKind::Key, set_ran)]
    });
    let gone = Widget::text("").id();

    ui.dispatch(&[DispatchEvent::semantic(
        gone,
        Event::key(
            brul_gui::KeyPress::from("Escape"),
            brul_gui::KeyModifiers::default(),
        ),
    )]);

    assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
}

#[test]
pub(super) fn a_raw_noop_key_reaches_the_capability_only() {
    let component = component_with(&["attach", "seen", "ran", "cleaned"]);
    let widget = Widget::text("");
    let id = widget.id();
    let built = Built {
        widget,
        bindings: Vec::new(),
        handlers: vec![
            Handler::new(&component, Some(id), EventKind::Key, key_code),
            Handler::new(&component, None, EventKind::Key, set_ran),
        ],
        props: Vec::new(),
    };
    let hooks = Hooks {
        target: EventTarget::Widget(id),
        state: StateId::source("test", "seven"),
        component: Rc::clone(&component),
        params: Vec::new(),
        apply: probe_apply,
        event: ignore_ui_event,
        raw_event: record_raw_target,
        cleanup: probe_cleanup,
        events: EventSet::empty(),
        raw_events: RawEventSet::from([brul_gui::RawKind::Key]),
    };
    let mut ui = Ui::assemble_with(
        Rc::clone(&component),
        built,
        themed_view,
        None,
        AssemblyParts {
            structures: Vec::new(),
            capabilities: vec![hooks],
            nested: Vec::new(),
            states: Rc::new(RefCell::new(crate::States::default())),
            event_sink: None,
            clipboard: None,
        },
        Rc::new(RefCell::new(Vec::new())),
        false,
    );
    ui.dispatch(&[RawUiEvent::widget(
        id,
        brul_gui::RawEvent::key_noop(
            brul_gui::KeyPress::from("Backspace"),
            brul_gui::KeyModifiers::default(),
        ),
    )
    .into()]);

    let component = ui.component.borrow();
    assert_eq!(component.read("attach"), Ok(Value::Number(id.get() as f64)));
    assert!(!matches!(component.read("seen"), Ok(Value::Object(_))));
    assert_ne!(component.read("ran"), Ok(Value::Bool(true)));
}

#[test]
pub(super) fn a_capability_can_emit_a_raw_event_to_other_capabilities() {
    let component = component_with(&["seen", "ran", "cleaned"]);
    let widget = Widget::text("");
    let id = widget.id();
    let built = Built {
        widget,
        bindings: Vec::new(),
        handlers: vec![Handler::new(&component, Some(id), EventKind::Key, set_ran)],
        props: Vec::new(),
    };
    let emitter = Hooks {
        target: EventTarget::Widget(id),
        state: StateId::source("test", "seven"),
        component: Rc::clone(&component),
        params: Vec::new(),
        apply: probe_apply,
        event: ignore_ui_event,
        raw_event: emit_raw_on_key,
        cleanup: probe_cleanup,
        events: EventSet::empty(),
        raw_events: RawEventSet::from([brul_gui::RawKind::Key]),
    };
    let observer = Hooks {
        target: EventTarget::Widget(id),
        state: StateId::source("test", "eight"),
        component: Rc::clone(&component),
        params: Vec::new(),
        apply: probe_apply,
        event: ignore_ui_event,
        raw_event: record_raw_paste,
        cleanup: probe_cleanup,
        events: EventSet::empty(),
        raw_events: RawEventSet::from([brul_gui::RawKind::Paste]),
    };
    let mut ui = Ui::assemble_with(
        Rc::clone(&component),
        built,
        themed_view,
        None,
        AssemblyParts {
            structures: Vec::new(),
            capabilities: vec![emitter, observer],
            nested: Vec::new(),
            states: Rc::new(RefCell::new(crate::States::default())),
            event_sink: None,
            clipboard: None,
        },
        Rc::new(RefCell::new(Vec::new())),
        false,
    );
    ui.dispatch(&[RawUiEvent::widget(
        id,
        brul_gui::RawEvent::key_noop(
            brul_gui::KeyPress::from("Backspace"),
            brul_gui::KeyModifiers::default(),
        ),
    )
    .into()]);

    let component = ui.component.borrow();
    assert_eq!(component.read("seen"), Ok(Value::Bool(true)));
    assert_ne!(component.read("ran"), Ok(Value::Bool(true)));
}

pub(super) fn window_raw_key(ctx: &CapabilityContext, _event: &RawUiEvent) -> Dispatch {
    ctx.component()
        .borrow_mut()
        .assign("attach", Value::Bool(true))
        .expect("attach");
    ctx.emit_event(UiEvent::window(Event::key(
        brul_gui::KeyPress::from("Enter"),
        brul_gui::KeyModifiers::default(),
    )));
    Dispatch::CONTINUE
}

pub(super) fn window_semantic_key(ctx: &CapabilityContext, _event: &UiEvent) -> Dispatch {
    ctx.component()
        .borrow_mut()
        .assign("seen", Value::Bool(true))
        .expect("seen");
    Dispatch::CONTINUE
}
