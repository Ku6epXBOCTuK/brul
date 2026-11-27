use super::*;
#[test]
pub(super) fn a_handle_message_reaches_the_component() {
    let seeds = [("name", Value::String("".into()))];
    let component = generated_component(NAMES, None, &seeds).expect("component");
    let defs = vec![WindowDef {
        name: "Only",
        config: WindowConfig::default(),
        view: seeded_view,
    }];
    let mut runtime = Runtime::new(
        defs,
        component,
        Rc::new(RefCell::new(Vec::new())),
        Rc::new(RefCell::new(None)),
        true,
        Vec::new(),
        None,
    );
    runtime
        .create(WindowKey(0), ColorScheme::Dark)
        .expect("window");
    let id = LAST_ID
        .with(|slot| slot.borrow_mut().take())
        .expect("widget");

    runtime.apply(vec![scoped(Message::Set("name".to_owned(), "live".into()))]);

    let tree = &mut runtime.uis[&WindowKey(0)].tree.borrow_mut();
    assert!(!tree.set_text(id, "live".to_owned()));
}

#[test]
pub(super) fn a_handle_call_runs_the_script_function() {
    let seeds = [("name", Value::String("".into()))];
    let script = Script::Generated {
        init: noop_init,
        dispatch: mark,
    };
    let component = generated_component(NAMES, Some(script), &seeds).expect("component");
    let defs = vec![WindowDef {
        name: "Only",
        config: WindowConfig::default(),
        view: seeded_view,
    }];
    let mut runtime = Runtime::new(
        defs,
        component,
        Rc::new(RefCell::new(Vec::new())),
        Rc::new(RefCell::new(None)),
        true,
        Vec::new(),
        None,
    );
    runtime
        .create(WindowKey(0), ColorScheme::Dark)
        .expect("window");
    let id = LAST_ID
        .with(|slot| slot.borrow_mut().take())
        .expect("widget");

    runtime.apply(vec![scoped(Message::Call("mark".to_owned(), Vec::new()))]);

    let tree = &mut runtime.uis[&WindowKey(0)].tree.borrow_mut();
    assert!(!tree.set_text(id, "called".to_owned()));
}

pub(super) fn record_widget_event(
    _component: &mut Component,
    _arguments: &[Dyn],
) -> Result<Dyn, crate::Error> {
    WIDGET_CALLS.with(|calls| calls.borrow_mut().push("changed".to_owned()));
    Ok(Dyn::null())
}

pub(super) fn stateful_view(
    _theme: Option<TestTheme>,
    build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    let count = build.state(WIDGET_STATE, component, || Rc::new(RefCell::new(0)));
    *count.borrow_mut() += 1;
    component
        .borrow_mut()
        .register_event_call("changed", record_widget_event);
    Ok(Built {
        widget: Widget::text(""),
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}
#[test]
fn a_widget_state_survives_a_rebuild() {
    let component = generated_component(NAMES, None, &[]).expect("component");
    let mut runtime = Runtime::new(
        vec![WindowDef {
            name: "Only",
            config: WindowConfig::default(),
            view: stateful_view,
        }],
        component,
        Rc::new(RefCell::new(Vec::new())),
        Rc::new(RefCell::new(Some("dark".to_owned()))),
        false,
        Vec::new(),
        None,
    );
    runtime
        .create(WindowKey(0), ColorScheme::Dark)
        .expect("window");
    let states = Rc::clone(&runtime.uis[&WindowKey(0)].states);
    let count = states
        .borrow()
        .get_cloned::<Rc<RefCell<i32>>>(WIDGET_STATE)
        .expect("state");
    assert_eq!(*count.borrow(), 1);

    runtime
        .uis
        .get_mut(&WindowKey(0))
        .expect("ui")
        .set_theme(TestTheme::Light)
        .expect("theme");
    assert_eq!(*count.borrow(), 2);
}

#[test]
fn a_widget_event_reaches_the_state_owner() {
    let component = generated_component(NAMES, None, &[]).expect("component");
    let mut runtime = Runtime::new(
        vec![WindowDef {
            name: "Only",
            config: WindowConfig::default(),
            view: stateful_view,
        }],
        component,
        Rc::new(RefCell::new(Vec::new())),
        Rc::new(RefCell::new(None)),
        true,
        Vec::new(),
        None,
    );
    runtime
        .create(WindowKey(0), ColorScheme::Dark)
        .expect("window");

    runtime.apply(vec![scoped(Message::Widget {
        state: WIDGET_STATE,
        event: "changed".to_owned(),
        arguments: Vec::new(),
    })]);

    assert_eq!(
        WIDGET_CALLS.with(|calls| calls.borrow().clone()),
        vec!["changed".to_owned()]
    );
}

#[test]
fn a_handle_call_passes_arguments() {
    const CALL_NAMES: &[&str] = &["name", "label"];
    let seeds = [("name", Value::from(""))];
    let script = Script::Generated {
        init: noop_init,
        dispatch: mark_with_arg,
    };
    let component = generated_component(CALL_NAMES, Some(script), &seeds).expect("component");
    let mut runtime = Runtime::new(
        defs(),
        component,
        Rc::new(RefCell::new(Vec::new())),
        Rc::new(RefCell::new(None)),
        true,
        Vec::new(),
        None,
    );

    runtime.apply(vec![scoped(Message::Call(
        "mark".to_owned(),
        vec![Value::from("argument")],
    ))]);

    assert_eq!(
        runtime.component.borrow().read("name"),
        Ok(Value::from("argument"))
    );
}

struct Recorder(Rc<RefCell<Vec<ScopedCommand>>>);

impl CommandSink for Recorder {
    fn send(&self, command: ScopedCommand) {
        self.0.borrow_mut().push(command);
    }
}

fn empty_view(
    _theme: Option<TestTheme>,
    _build: &mut Build,
    _component: &ComponentHandle,
) -> Result<Built, ViewError> {
    Ok(Built {
        widget: Widget::text(""),
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

fn hide_dispatch(
    component: &mut Component,
    name: &str,
    _arguments: Vec<Dyn>,
) -> Result<Dyn, crate::Error> {
    if name != "hide" {
        return Err(crate::Error::UnknownFunction(name.to_owned()));
    }
    brul_script::eval::call(component, "hideWindow", Vec::new())?;
    Ok(Dyn::null())
}

#[test]
fn a_handle_call_targets_its_window() {
    const HIDE_NAMES: &[&str] = &["hide"];
    let script = Script::Generated {
        init: noop_init,
        dispatch: hide_dispatch,
    };
    let component = generated_component(HIDE_NAMES, Some(script), &[]).expect("component");
    let commands = Rc::new(RefCell::new(Vec::new()));
    component.borrow_mut().set_host(CommandHost {
        commands: Rc::clone(&commands),
        app: None,
        windows: Rc::new(HashMap::new()),
        theme: Rc::new(RefCell::new(None)),
    });
    let defs = vec![
        WindowDef {
            name: "Main",
            config: WindowConfig::default(),
            view: empty_view,
        },
        WindowDef {
            name: "Other",
            config: WindowConfig::default(),
            view: empty_view,
        },
    ];
    let recorded = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = Runtime::new(
        defs,
        component,
        commands,
        Rc::new(RefCell::new(None)),
        true,
        Vec::new(),
        Some(Box::new(Recorder(Rc::clone(&recorded)))),
    );
    runtime
        .create(WindowKey(0), ColorScheme::Dark)
        .expect("main window");
    runtime
        .create(WindowKey(1), ColorScheme::Dark)
        .expect("other window");

    runtime.apply(vec![ScopedMessage {
        window: WindowKey(1),
        message: Message::Call("hide".to_owned(), Vec::new()),
    }]);

    assert_eq!(
        *recorded.borrow(),
        vec![ScopedCommand {
            window: WindowKey(1),
            command: WindowCommand::Hide,
        }]
    );
}

#[test]
fn a_handle_set_carries_a_list() {
    let component = generated_component(NAMES, None, &[("name", Value::Null)]).expect("component");
    let mut runtime = Runtime::new(
        defs(),
        component,
        Rc::new(RefCell::new(Vec::new())),
        Rc::new(RefCell::new(None)),
        true,
        Vec::new(),
        None,
    );
    let rows = Value::Array(vec![Value::from(1.0), Value::from(2.0)].into());

    runtime.apply(vec![scoped(Message::Set("name".to_owned(), rows.clone()))]);

    assert_eq!(runtime.component.borrow().read("name"), Ok(rows));
}
