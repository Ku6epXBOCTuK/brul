use super::*;
#[test]
fn closing_a_window_drops_its_ui() {
    let mut runtime = runtime(None);
    runtime
        .create(WindowKey(0), ColorScheme::Dark)
        .expect("first window");
    runtime.close(WindowKey(0));
    assert!(runtime.uis.is_empty());
}

fn key_view(component: &ComponentHandle, call: crate::EventCall) -> Result<Built, ViewError> {
    let widget = Widget::text("");
    Ok(Built {
        widget,
        bindings: Vec::new(),
        handlers: vec![Handler::new(component, None, EventKind::Key, call)],
        props: Vec::new(),
    })
}

fn picker_view(
    _theme: Option<TestTheme>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    key_view(component, key_exit)
}

fn settings_view(
    _theme: Option<TestTheme>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    key_view(component, key_exit)
}

fn key_exit(component: &mut Component, arguments: &[Dyn]) -> Result<Dyn, crate::Error> {
    if arguments[0].field("code")?.as_text()? == "Escape" {
        <crate::builtins::Exit as crate::HostCommand>::invoke(component, ())?;
    }
    Ok(Dyn::null())
}

fn key_exit_dispatch(
    component: &mut Component,
    name: &str,
    arguments: Vec<Dyn>,
) -> Result<Dyn, crate::Error> {
    if name != "handleKey" && name != "handleSettingsKey" {
        return Err(crate::Error::UnknownFunction(name.to_owned()));
    }
    let code = brul_script::eval::read_field(&Value::from(arguments[0].clone()), "code")?;
    if matches!(code, Value::String(text) if text.as_ref() == "Escape") {
        brul_script::eval::call(component, "exit", Vec::new())?;
    }
    Ok(Dyn::null())
}

#[test]
fn a_key_closes_only_the_window_that_received_it() {
    let names: &'static [&'static str] = &["handleKey", "handleSettingsKey", "exit", "ev", "code"];
    let script = Script::Generated {
        init: noop_init,
        dispatch: key_exit_dispatch,
    };
    let component = generated_component(names, Some(script), &[]).expect("component");
    let commands = Rc::new(RefCell::new(Vec::new()));
    let keys = HashMap::from([
        ("Picker".to_owned(), WindowKey(0)),
        ("Settings".to_owned(), WindowKey(1)),
    ]);
    component.borrow_mut().set_host(CommandHost {
        commands: Rc::clone(&commands),
        app: None,
        windows: Rc::new(keys),
        theme: Rc::new(RefCell::new(None)),
    });
    let defs = vec![
        WindowDef {
            name: "Picker",
            config: WindowConfig::default(),
            view: picker_view,
        },
        WindowDef {
            name: "Settings",
            config: WindowConfig::default(),
            view: settings_view,
        },
    ];
    let mut runtime = Runtime::new(
        defs,
        component,
        commands,
        Rc::new(RefCell::new(None)),
        true,
        Vec::new(),
        None,
    );
    runtime
        .create(WindowKey(0), ColorScheme::Dark)
        .expect("picker");
    runtime
        .create(WindowKey(1), ColorScheme::Dark)
        .expect("settings");

    let commands = runtime.dispatch(
        WindowKey(1),
        &[DispatchEvent::Semantic(brul_gui::UiEvent::window(
            Event::key(
                brul_gui::KeyPress::from("Escape"),
                brul_gui::KeyModifiers::default(),
            ),
        ))],
    );

    assert_eq!(commands, vec![WindowCommand::Close]);
    assert!(runtime.uis.contains_key(&WindowKey(0)));
}
