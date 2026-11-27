use super::*;
#[test]
pub(super) fn a_theme_change_rereads_the_theme_from_the_script() {
    let component = generated_component(NAMES, None, &[]).expect("component");
    let commands = Rc::new(RefCell::new(Vec::new()));
    let shown = Rc::new(RefCell::new(None));
    component.borrow_mut().set_host(CommandHost {
        commands: Rc::clone(&commands),
        app: None,
        windows: Rc::new(HashMap::new()),
        theme: Rc::clone(&shown),
    });
    let defs = vec![WindowDef {
        name: "Only",
        config: WindowConfig::default(),
        view: theme_view,
    }];
    let caller = Rc::clone(&component);
    let mut runtime = Runtime::new(defs, component, commands, shown, true, Vec::new(), None);

    runtime
        .create(WindowKey(0), ColorScheme::Dark)
        .expect("window");
    assert_eq!(
        SEEN.with(|seen| seen.borrow().last().cloned()),
        Some("dark".to_owned())
    );

    caller
        .borrow_mut()
        .call("setTheme", &[Value::String("light".into())])
        .expect("theme command");
    runtime.dispatch(WindowKey(0), &[]);
    assert_eq!(
        SEEN.with(|seen| seen.borrow().last().cloned()),
        Some("light".to_owned())
    );
    assert_eq!(
        VIEW_THEMES.with(|seen| seen.borrow().last().copied()),
        Some(Some("light"))
    );
}

thread_local! {
    pub(super) static LAST_ID: RefCell<Option<WidgetId>> = const { RefCell::new(None) };
}

pub(super) fn seeded_view(
    _theme: Option<TestTheme>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    let widget = Widget::text("");
    let id = widget.id();
    LAST_ID.with(|slot| *slot.borrow_mut() = Some(id));
    Ok(Built {
        widget,
        bindings: vec![Binding::new(component, id, BindingTarget::Text(name))],
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

#[test]
fn a_theme_parses_from_its_name() {
    assert_eq!(TestTheme::parse("dark"), Some(TestTheme::Dark));
    assert_eq!(TestTheme::parse("sepia"), None);
}

#[test]
fn system_theme_follows_the_color_scheme() {
    assert_eq!(scheme_theme(None, ColorScheme::Light), "light");
    assert_eq!(scheme_theme(None, ColorScheme::Dark), "dark");
}
