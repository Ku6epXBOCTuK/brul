use super::*;
#[test]
pub(super) fn stop_capabilities_keeps_the_script() {
    let (mut ui, component) = flags_ui(first_stop_capabilities, raw_first_continue);
    ui.dispatch(&[key_event()]);
    let component = component.borrow();
    assert_eq!(component.read("first"), Ok(Value::Bool(true)));
    assert_ne!(component.read("second"), Ok(Value::Bool(true)));
    assert_eq!(component.read("ran"), Ok(Value::Bool(true)));
}

#[test]
pub(super) fn stop_scripts_keeps_other_capabilities_and_raw_policies() {
    let (mut ui, component) = flags_ui(first_stop_scripts, raw_stop_scripts);
    ui.dispatch(&[key_event()]);
    ui.dispatch(&[RawUiEvent::window(brul_gui::RawEvent::key(
        brul_gui::KeyPress::from("Enter"),
        brul_gui::KeyModifiers::default(),
    ))
    .into()]);
    let component = component.borrow();
    assert_eq!(component.read("first"), Ok(Value::Bool(true)));
    assert_eq!(component.read("second"), Ok(Value::Bool(true)));
    assert_ne!(component.read("ran"), Ok(Value::Bool(true)));
}

#[test]
pub(super) fn handled_is_visible_to_later_capabilities() {
    let (mut ui, component) = flags_ui(first_handled, raw_first_continue);
    ui.dispatch(&[key_event()]);
    let component = component.borrow();
    assert_eq!(component.read("handled"), Ok(Value::Bool(true)));
    assert_eq!(component.read("second"), Ok(Value::Bool(true)));
    assert_eq!(component.read("ran"), Ok(Value::Bool(true)));
}

pub(super) fn ignore_ui_event(_ctx: &CapabilityContext, _event: &UiEvent) -> Dispatch {
    Dispatch::CONTINUE
}

pub(super) fn emit_raw_on_key(ctx: &CapabilityContext, event: &RawUiEvent) -> Dispatch {
    if let Some(id) = event.target.widget() {
        ctx.emit_raw_event(RawUiEvent::widget(
            id,
            brul_gui::RawEvent::paste(String::new()),
        ));
    }
    Dispatch::CONTINUE
}

pub(super) fn record_raw_paste(ctx: &CapabilityContext, event: &RawUiEvent) -> Dispatch {
    if matches!(event.event, brul_gui::RawEvent::Paste { .. }) {
        ctx.component()
            .borrow_mut()
            .assign("seen", Value::Bool(true))
            .expect("seen");
    }
    Dispatch::CONTINUE
}

pub(super) fn record_attachment_target(ctx: &CapabilityContext, event: &UiEvent) -> Dispatch {
    if let Some(id) = event.target.widget() {
        ctx.component()
            .borrow_mut()
            .assign("attach", Value::Number(id.get() as f64))
            .expect("attach");
    }
    Dispatch::CONTINUE
}

pub(super) fn record_raw_target(ctx: &CapabilityContext, event: &RawUiEvent) -> Dispatch {
    if let Some(id) = event.target.widget() {
        ctx.component()
            .borrow_mut()
            .assign("attach", Value::Number(id.get() as f64))
            .expect("attach");
    }
    Dispatch::CONTINUE
}

pub(super) fn record_script_target(
    component: &mut Component,
    name: &str,
    arguments: &[Dyn],
) -> Result<Dyn, Error> {
    let target = arguments[0].field("target")?.as_number()?;
    component.assign(name, Value::Number(target))?;
    Ok(Dyn::null())
}

pub(super) fn record_widget_script_target(
    component: &mut Component,
    arguments: &[Dyn],
) -> Result<Dyn, Error> {
    record_script_target(component, "script", arguments)
}

pub(super) fn record_window_script_target(
    component: &mut Component,
    arguments: &[Dyn],
) -> Result<Dyn, Error> {
    record_script_target(component, "window", arguments)
}

#[test]
pub(super) fn one_targeted_event_reaches_the_attachment_and_both_scripts() {
    let component = component_with(&["attach", "script", "window"]);
    let widget = Widget::text("");
    let id = widget.id();
    let built = Built {
        widget,
        bindings: Vec::new(),
        handlers: vec![
            Handler::new(
                &component,
                Some(id),
                EventKind::Key,
                record_widget_script_target,
            ),
            Handler::new(
                &component,
                None,
                EventKind::Key,
                record_window_script_target,
            ),
        ],
        props: Vec::new(),
    };
    let hooks = Hooks {
        target: EventTarget::Widget(id),
        state: StateId::source("test", "seven"),
        component: Rc::clone(&component),
        params: Vec::new(),
        apply: probe_apply,
        event: record_attachment_target,
        raw_event: ignore_raw_event,
        cleanup: |_| {},
        events: EventSet::from([EventKind::Key]),
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
    ui.dispatch(&[DispatchEvent::semantic(
        id,
        Event::key(
            brul_gui::KeyPress::from("a"),
            brul_gui::KeyModifiers::default(),
        ),
    )]);
    let expected = Value::Number(id.get() as f64);
    let component = ui.component.borrow();
    assert_eq!(component.read("attach"), Ok(expected.clone()));
    assert_eq!(component.read("script"), Ok(expected));
    assert_eq!(component.read("window"), Ok(Value::Number(id.get() as f64)));
}

#[test]
pub(super) fn a_window_capability_sees_a_raw_key_and_emits_a_semantic_key() {
    let component = component_with(&["attach", "seen", "ran", "off"]);
    let built = Built {
        widget: Widget::text(""),
        bindings: Vec::new(),
        handlers: vec![Handler::new(&component, None, EventKind::Key, set_ran)],
        props: Vec::new(),
    };
    let raw = Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "seven"),
        component: Rc::clone(&component),
        params: Vec::new(),
        apply: probe_apply,
        event: ignore_ui_event,
        raw_event: window_raw_key,
        cleanup: |_| {},
        events: EventSet::empty(),
        raw_events: RawEventSet::from([brul_gui::RawKind::Key]),
    };
    let semantic = Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "eight"),
        component: Rc::clone(&component),
        params: Vec::new(),
        apply: probe_apply,
        event: window_semantic_key,
        raw_event: ignore_raw_event,
        cleanup: |_| {},
        events: EventSet::from([EventKind::Key]),
        raw_events: RawEventSet::empty(),
    };
    let mut ui = Ui::assemble_with(
        Rc::clone(&component),
        built,
        themed_view,
        None,
        AssemblyParts {
            structures: Vec::new(),
            capabilities: vec![raw, semantic],
            nested: Vec::new(),
            states: Rc::new(RefCell::new(crate::States::default())),
            event_sink: None,
            clipboard: None,
        },
        Rc::new(RefCell::new(Vec::new())),
        false,
    );

    ui.dispatch(&[RawUiEvent::window(brul_gui::RawEvent::key(
        brul_gui::KeyPress::from("Enter"),
        brul_gui::KeyModifiers::default(),
    ))
    .into()]);

    let component = ui.component.borrow();
    assert_eq!(component.read("attach"), Ok(Value::Bool(true)));
    assert_eq!(component.read("seen"), Ok(Value::Bool(true)));
    assert_eq!(component.read("ran"), Ok(Value::Bool(true)));
    assert_ne!(component.read("off"), Ok(Value::Bool(true)));
}

#[test]
pub(super) fn a_raw_window_key_stays_with_capabilities() {
    let component = component_with(&["attach", "ran"]);
    let built = Built {
        widget: Widget::text(""),
        bindings: Vec::new(),
        handlers: vec![Handler::new(&component, None, EventKind::Key, set_ran)],
        props: Vec::new(),
    };
    let hooks = Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "seven"),
        component: Rc::clone(&component),
        params: Vec::new(),
        apply: probe_apply,
        event: ignore_ui_event,
        raw_event: |ctx: &CapabilityContext, _: &RawUiEvent| {
            ctx.component()
                .borrow_mut()
                .assign("attach", Value::Bool(true))
                .expect("attach");
            Dispatch::CONTINUE
        },
        cleanup: |_| {},
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

    ui.dispatch(&[RawUiEvent::window(brul_gui::RawEvent::key(
        brul_gui::KeyPress::from("Enter"),
        brul_gui::KeyModifiers::default(),
    ))
    .into()]);

    let component = ui.component.borrow();
    assert_eq!(component.read("attach"), Ok(Value::Bool(true)));
    assert_ne!(component.read("ran"), Ok(Value::Bool(true)));
}

#[test]
pub(super) fn a_window_capability_can_stop_the_dispatch_chain() {
    let component = component_with(&["attach", "off", "ran"]);
    let built = Built {
        widget: Widget::text(""),
        bindings: Vec::new(),
        handlers: vec![Handler::new(&component, None, EventKind::Key, set_ran)],
        props: Vec::new(),
    };
    let stop = Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "seven"),
        component: Rc::clone(&component),
        params: Vec::new(),
        apply: probe_apply,
        event: |ctx: &CapabilityContext, _: &UiEvent| {
            ctx.component()
                .borrow_mut()
                .assign("attach", Value::Bool(true))
                .expect("attach");
            Dispatch::stop()
        },
        raw_event: |ctx: &CapabilityContext, _: &RawUiEvent| {
            ctx.component()
                .borrow_mut()
                .assign("attach", Value::Bool(true))
                .expect("attach");
            Dispatch::stop()
        },
        cleanup: |_| {},
        events: EventSet::from([EventKind::Key]),
        raw_events: RawEventSet::from([brul_gui::RawKind::Key]),
    };
    let later = Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "eight"),
        component: Rc::clone(&component),
        params: Vec::new(),
        apply: probe_apply,
        event: |ctx: &CapabilityContext, _: &UiEvent| {
            ctx.component()
                .borrow_mut()
                .assign("off", Value::Bool(true))
                .expect("off");
            Dispatch::CONTINUE
        },
        raw_event: |ctx: &CapabilityContext, _: &RawUiEvent| {
            ctx.component()
                .borrow_mut()
                .assign("off", Value::Bool(true))
                .expect("off");
            Dispatch::CONTINUE
        },
        cleanup: |_| {},
        events: EventSet::from([EventKind::Key]),
        raw_events: RawEventSet::from([brul_gui::RawKind::Key]),
    };
    let mut ui = Ui::assemble_with(
        Rc::clone(&component),
        built,
        themed_view,
        None,
        AssemblyParts {
            structures: Vec::new(),
            capabilities: vec![stop, later],
            nested: Vec::new(),
            states: Rc::new(RefCell::new(crate::States::default())),
            event_sink: None,
            clipboard: None,
        },
        Rc::new(RefCell::new(Vec::new())),
        false,
    );

    ui.dispatch(&[UiEvent::window(Event::key(
        brul_gui::KeyPress::from("Enter"),
        brul_gui::KeyModifiers::default(),
    ))
    .into()]);
    ui.dispatch(&[RawUiEvent::window(brul_gui::RawEvent::key(
        brul_gui::KeyPress::from("Enter"),
        brul_gui::KeyModifiers::default(),
    ))
    .into()]);

    let component = ui.component.borrow();
    assert_eq!(component.read("attach"), Ok(Value::Bool(true)));
    assert_ne!(component.read("off"), Ok(Value::Bool(true)));
    assert_ne!(component.read("ran"), Ok(Value::Bool(true)));
}

pub(super) fn mark_first(ctx: &CapabilityContext) {
    ctx.component()
        .borrow_mut()
        .assign("first", Value::Bool(true))
        .expect("first");
}

pub(super) fn first_handled(ctx: &CapabilityContext, _: &UiEvent) -> Dispatch {
    mark_first(ctx);
    Dispatch::handled()
}

pub(super) fn first_stop_capabilities(ctx: &CapabilityContext, _: &UiEvent) -> Dispatch {
    mark_first(ctx);
    Dispatch::stop_capabilities()
}

pub(super) fn first_stop_scripts(ctx: &CapabilityContext, _: &UiEvent) -> Dispatch {
    mark_first(ctx);
    Dispatch::stop_scripts()
}

pub(super) fn raw_stop_scripts(ctx: &CapabilityContext, _: &RawUiEvent) -> Dispatch {
    mark_first(ctx);
    Dispatch::stop_scripts()
}

pub(super) fn raw_first_continue(ctx: &CapabilityContext, _: &RawUiEvent) -> Dispatch {
    mark_first(ctx);
    Dispatch::CONTINUE
}

pub(super) fn second_flags(ctx: &CapabilityContext, _: &UiEvent) -> Dispatch {
    let mut component = ctx.component().borrow_mut();
    component
        .assign("second", Value::Bool(true))
        .expect("second");
    if ctx.handled() {
        component
            .assign("handled", Value::Bool(true))
            .expect("handled");
    }
    Dispatch::CONTINUE
}

pub(super) fn second_raw(ctx: &CapabilityContext, _: &RawUiEvent) -> Dispatch {
    ctx.component()
        .borrow_mut()
        .assign("second", Value::Bool(true))
        .expect("second");
    Dispatch::CONTINUE
}

pub(super) fn flags_ui(
    first_event: fn(&CapabilityContext, &UiEvent) -> Dispatch,
    first_raw: fn(&CapabilityContext, &RawUiEvent) -> Dispatch,
) -> (Ui<TestTheme>, ComponentHandle) {
    let component = component_with(&["first", "second", "handled", "ran"]);
    let built = Built {
        widget: Widget::text(""),
        bindings: Vec::new(),
        handlers: vec![Handler::new(&component, None, EventKind::Key, set_ran)],
        props: Vec::new(),
    };
    let first = Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "flags-first"),
        component: Rc::clone(&component),
        params: Vec::new(),
        apply: probe_apply,
        event: first_event,
        raw_event: first_raw,
        cleanup: |_| {},
        events: EventSet::from([EventKind::Key]),
        raw_events: RawEventSet::from([brul_gui::RawKind::Key]),
    };
    let second = Hooks {
        target: EventTarget::Window,
        state: StateId::source("test", "flags-second"),
        component: Rc::clone(&component),
        params: Vec::new(),
        apply: probe_apply,
        event: second_flags,
        raw_event: second_raw,
        cleanup: |_| {},
        events: EventSet::from([EventKind::Key]),
        raw_events: RawEventSet::from([brul_gui::RawKind::Key]),
    };
    let ui = Ui::assemble_with(
        Rc::clone(&component),
        built,
        themed_view,
        None,
        AssemblyParts {
            structures: Vec::new(),
            capabilities: vec![first, second],
            nested: Vec::new(),
            states: Rc::new(RefCell::new(crate::States::default())),
            event_sink: None,
            clipboard: None,
        },
        Rc::new(RefCell::new(Vec::new())),
        false,
    );
    (ui, component)
}

pub(super) fn key_event() -> DispatchEvent {
    UiEvent::window(Event::key(
        brul_gui::KeyPress::from("Enter"),
        brul_gui::KeyModifiers::default(),
    ))
    .into()
}
