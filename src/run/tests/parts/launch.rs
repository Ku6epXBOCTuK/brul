use super::*;
#[test]
pub(super) fn generated_launch_declares_seeds() {
    let seeds = [("name", Value::String("from seed".into()))];
    let (id, ui) = assembled(None, &seeds, None);
    brul_reactive::flush();
    assert!(!ui.tree.borrow_mut().set_text(id, "from seed".to_owned()));
}

#[test]
pub(super) fn generated_launch_runs_the_script() {
    let script = Script::Generated {
        init: declares_name,
        dispatch: noop,
    };
    let (id, ui) = assembled(None, &[], Some(script));
    brul_reactive::flush();
    assert!(!ui.tree.borrow_mut().set_text(id, "from script".to_owned()));
}

#[test]
pub(super) fn a_window_is_built_once_and_reused() {
    let mut runtime = runtime(None);
    let first = runtime
        .create(WindowKey(1), ColorScheme::Dark)
        .expect("second window");
    let again = runtime
        .create(WindowKey(1), ColorScheme::Light)
        .expect("second window again");

    assert!(Rc::ptr_eq(&first.tree, &again.tree));
    assert_eq!(first.config.title, again.config.title);
}

#[test]
pub(super) fn an_unknown_key_is_an_error() {
    let mut runtime = runtime(None);
    let error = match runtime.create(WindowKey(7), ColorScheme::Dark) {
        Ok(_) => panic!("unknown window was created"),
        Err(error) => error,
    };
    assert!(
        matches!(error, WindowError::Undeclared(WindowKey(7))),
        "{error}"
    );
}

#[test]
pub(super) fn a_theme_command_reaches_every_open_window() {
    let mut runtime = runtime(None);
    runtime
        .create(WindowKey(0), ColorScheme::Dark)
        .expect("first window");
    runtime
        .create(WindowKey(1), ColorScheme::Dark)
        .expect("second window");
    runtime
        .commands
        .borrow_mut()
        .push(WindowCommand::SetTheme("light".to_owned()));

    assert!(runtime.dispatch(WindowKey(0), &[]).is_empty());
    for ui in runtime.uis.values() {
        assert_eq!(ui.theme, Some(TestTheme::Light));
    }
}

pub(super) const WIDGET_STATE: StateId = StateId::source("test", "widget");

thread_local! {
    pub(super) static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    pub(super) static WIDGET_CALLS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub(super) fn theme_text(component: &mut crate::Component) -> Result<String, crate::Error> {
    let theme = component.call("theme", &[])?;
    let name = match &theme {
        Value::String(name) => name.to_string(),
        _ => "none".to_owned(),
    };
    SEEN.with(|seen| seen.borrow_mut().push(name.clone()));
    Ok(name)
}

pub(super) fn theme_view(
    _theme: Option<TestTheme>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    let widget = Widget::text("");
    let id = widget.id();
    Ok(Built {
        widget,
        bindings: vec![Binding::new(component, id, BindingTarget::Text(theme_text))],
        handlers: Vec::new(),
        props: Vec::new(),
    })
}
