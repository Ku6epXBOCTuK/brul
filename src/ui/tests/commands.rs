use super::*;

pub(super) fn command_host(
    app: Option<Box<dyn Host>>,
) -> (CommandHost, Rc<RefCell<Vec<WindowCommand>>>) {
    let commands = Rc::new(RefCell::new(Vec::new()));
    (
        CommandHost {
            commands: Rc::clone(&commands),
            app,
            windows: Rc::new(HashMap::new()),
            theme: Rc::new(RefCell::new(None)),
        },
        commands,
    )
}

#[test]
pub(super) fn theme_reports_the_shown_theme() {
    let (mut host, _) = command_host(None);
    assert_eq!(host.call("theme", &[]), Ok(Value::Null));

    *host.theme.borrow_mut() = Some("dark".to_owned());
    assert_eq!(
        host.call("theme", &[]),
        Ok(Value::String("dark".to_owned().into()))
    );
    assert_eq!(
        host.call("theme", &[Value::String("dark".into())]),
        Err(Error::Arity {
            name: "theme".to_owned(),
            expected: "no",
        })
    );
}

pub(super) struct Recorder {
    calls: Rc<RefCell<Vec<String>>>,
}

impl Host for Recorder {
    fn call(&mut self, name: &str, _arguments: &[Value]) -> Result<Value, Error> {
        self.calls.borrow_mut().push(name.to_owned());
        Ok(Value::Null)
    }
}

#[test]
pub(super) fn set_theme_takes_exactly_one_string() {
    let (mut host, commands) = command_host(None);
    assert_eq!(
        host.call("setTheme", &[]),
        Err(Error::Arity {
            name: "setTheme".to_owned(),
            expected: "one",
        })
    );
    assert_eq!(
        host.call("setTheme", &[Value::Number(1.0)]),
        Err(Error::Argument {
            name: "setTheme",
            expected: "String",
            found: "Number",
        })
    );
    assert_eq!(
        host.call("setTheme", &[Value::String("dark".into())]),
        Ok(Value::Null)
    );
    assert_eq!(
        *commands.borrow(),
        vec![WindowCommand::SetTheme("dark".to_owned())]
    );
}

#[test]
pub(super) fn set_always_on_top_takes_exactly_one_flag() {
    let (mut host, commands) = command_host(None);
    assert_eq!(
        host.call("setAlwaysOnTop", &[]),
        Err(Error::Arity {
            name: "setAlwaysOnTop".to_owned(),
            expected: "one",
        })
    );
    assert_eq!(
        host.call("setAlwaysOnTop", &[Value::Number(1.0)]),
        Err(Error::Argument {
            name: "setAlwaysOnTop",
            expected: "Bool",
            found: "Number",
        })
    );
    assert_eq!(
        host.call("setAlwaysOnTop", &[Value::Bool(true)]),
        Ok(Value::Null)
    );
    assert_eq!(*commands.borrow(), vec![WindowCommand::AlwaysOnTop(true)]);
}

#[test]
pub(super) fn every_host_builtin_is_dispatched() {
    let (mut host, _) = command_host(None);
    let names: Vec<_> = BUILTINS
        .iter()
        .filter(|builtin| matches!(builtin.emit, BuiltinEmit::Host { .. }))
        .map(|builtin| builtin.name)
        .collect();
    assert!(!names.is_empty());
    for name in names {
        assert!(
            !matches!(host.call(name, &[]), Err(Error::UnknownFunction(_))),
            "{name} is not dispatched"
        );
    }
}

#[test]
pub(super) fn every_host_builtin_has_a_marker() {
    let names: Vec<_> = BUILTINS
        .iter()
        .filter_map(|builtin| match builtin.emit {
            BuiltinEmit::Host { marker } => Some((builtin.name, marker)),
            _ => None,
        })
        .collect();
    assert!(!names.is_empty());
    for (name, marker) in names {
        let entry = crate::builtins::HOST_MARKERS
            .iter()
            .find(|(marker_name, _, _)| *marker_name == name)
            .unwrap_or_else(|| panic!("{name} has no generated marker"));
        assert_eq!(entry.1, marker);
        assert_eq!((entry.2)(), name);
    }
}

#[test]
pub(super) fn the_builtin_commands_take_no_arguments() {
    let (mut host, commands) = command_host(None);
    assert_eq!(host.call("exit", &[]), Ok(Value::Null));
    assert_eq!(host.call("closeWindow", &[]), Ok(Value::Null));
    assert_eq!(host.call("dragWindow", &[]), Ok(Value::Null));
    assert_eq!(host.call("showWindow", &[]), Ok(Value::Null));
    assert_eq!(host.call("hideWindow", &[]), Ok(Value::Null));
    assert_eq!(host.call("focusWindow", &[]), Ok(Value::Null));
    assert_eq!(
        host.call("exit", &[Value::Number(1.0)]),
        Err(Error::Arity {
            name: "exit".to_owned(),
            expected: "no",
        })
    );
    assert_eq!(
        *commands.borrow(),
        vec![
            WindowCommand::Close,
            WindowCommand::Close,
            WindowCommand::Drag,
            WindowCommand::Show,
            WindowCommand::Hide,
            WindowCommand::Focus
        ]
    );
}

#[test]
pub(super) fn open_window_finds_a_declared_name() {
    let commands = Rc::new(RefCell::new(Vec::new()));
    let mut windows = HashMap::new();
    windows.insert("Settings".to_owned(), WindowKey(1));
    let mut host = CommandHost {
        commands: Rc::clone(&commands),
        app: None,
        windows: Rc::new(windows),
        theme: Rc::new(RefCell::new(None)),
    };

    assert_eq!(
        host.call("openWindow", &[Value::String("Settings".into())]),
        Ok(Value::Null)
    );
    assert_eq!(
        host.call("openWindow", &[Value::String("Missing".into())]),
        Err(Error::UnknownWindow("Missing".to_owned()))
    );
    assert_eq!(
        host.call("openWindow", &[]),
        Err(Error::Arity {
            name: "openWindow".to_owned(),
            expected: "one",
        })
    );
    assert_eq!(*commands.borrow(), vec![WindowCommand::Open(WindowKey(1))]);
}

#[test]
pub(super) fn an_unknown_command_without_a_host_is_an_error() {
    let (mut host, _commands) = command_host(None);
    assert_eq!(
        host.call("appCommand", &[]),
        Err(Error::UnknownFunction("appCommand".to_owned()))
    );
}

#[test]
pub(super) fn commands_reach_the_application_host() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let recorder = Recorder {
        calls: Rc::clone(&calls),
    };
    let (mut host, _commands) = command_host(Some(Box::new(recorder)));
    assert_eq!(host.call("appCommand", &[]), Ok(Value::Null));
    assert_eq!(*calls.borrow(), vec!["appCommand".to_owned()]);
}
#[test]
pub(super) fn every_before_assembly_becomes_a_window_command() {
    let component = component_with(&[]);
    component.borrow_mut().every(500.0, "tick").expect("every");
    let (_, ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
    assert_eq!(
        *ui.commands.borrow(),
        vec![WindowCommand::Timer(TimerCommand::Start {
            id: brul_ident::TimerId::new(0),
            owner: TimerOwner::new(0),
            function: "tick".into(),
            interval: Duration::from_secs_f64(0.5),
            kind: brul_ident::TimerKind::Every,
        })]
    );
}

#[test]
pub(super) fn every_from_a_handler_is_scheduled_without_an_app_host() {
    let component = component_with(&[]);
    let (_, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
    assert_eq!(
        ui.dispatch(&[UiEvent::window(Event::timer(TimerOwner::new(0), "schedule".into())).into()]),
        vec![WindowCommand::Timer(TimerCommand::Start {
            id: brul_ident::TimerId::new(0),
            owner: TimerOwner::new(0),
            function: "tick".into(),
            interval: Duration::from_secs(1),
            kind: brul_ident::TimerKind::Every,
        })]
    );
}

#[test]
pub(super) fn dispatch_ignores_events_without_a_handler() {
    let component = component_with(&[]);
    let (id, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
    let other = Widget::text("other").id();
    assert!(
        ui.dispatch(&[
            RawUiEvent::widget(id, brul_gui::RawEvent::drag(0, 0)).into(),
            DispatchEvent::semantic(other, Event::click()),
        ])
        .is_empty()
    );
    assert!(ui.component.borrow().read("ran").is_err());
}

#[test]
pub(super) fn dispatch_returns_commands_it_cannot_handle() {
    let component = component_with(&[]);
    let (_, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
    ui.commands.borrow_mut().push(WindowCommand::Exit);
    assert_eq!(ui.dispatch(&[]), vec![WindowCommand::Exit]);
}

#[test]
pub(super) fn dispatch_returns_the_theme_command() {
    let (component, built, structures) =
        generated_built(themed_view, NAMES, None, &[], None).expect("launch");
    let mut ui = Ui::assemble(component, built, themed_view, None, structures, None);
    ui.commands
        .borrow_mut()
        .push(WindowCommand::SetTheme("dark".to_owned()));
    assert_eq!(
        ui.dispatch(&[]),
        vec![WindowCommand::SetTheme("dark".to_owned())]
    );
    assert!(ui.theme.is_none());
}

pub(super) fn child_view(
    theme: Option<&str>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    if theme.is_some() {
        return Err(unknown_theme());
    }
    let text = Widget::text("child");
    let id = text.id();
    Ok(Built {
        widget: text,
        bindings: Vec::new(),
        handlers: vec![Handler::new(
            component,
            Some(id),
            EventKind::Click,
            raise_call,
        )],
        props: Vec::new(),
    })
}
