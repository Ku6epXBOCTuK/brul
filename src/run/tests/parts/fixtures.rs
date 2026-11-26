use super::*;
pub(super) const NAMES: &[&str] = &["name"];

pub(super) fn name(component: &mut crate::Component) -> Result<String, crate::Error> {
    (&component.read("name")?).try_into()
}

pub(super) fn noop_init(_: &mut Component) -> Result<(), crate::Error> {
    Ok(())
}

pub(super) fn noop(_: &mut Component, _: &str, _: Vec<Dyn>) -> Result<Dyn, crate::Error> {
    Ok(Dyn::null())
}

pub(super) fn declares_name(component: &mut Component) -> Result<(), crate::Error> {
    component.declare("name", Value::String("from script".into()), false)
}

pub(super) fn mark(
    component: &mut Component,
    name: &str,
    _: Vec<Dyn>,
) -> Result<Dyn, crate::Error> {
    if name != "mark" {
        return Err(crate::Error::UnknownFunction(name.to_owned()));
    }
    component.assign("name", Value::String("called".into()))?;
    Ok(Dyn::null())
}

pub(super) fn mark_with_arg(
    component: &mut Component,
    name: &str,
    arguments: Vec<Dyn>,
) -> Result<Dyn, crate::Error> {
    if name != "mark" {
        return Err(crate::Error::UnknownFunction(name.to_owned()));
    }
    let label = Value::from(arguments[0].clone());
    component.assign("name", label)?;
    Ok(Dyn::null())
}

pub(super) fn generated_view(
    _theme: Option<TestTheme>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    let widget = Widget::text("");
    let id = widget.id();
    Ok(Built {
        widget,
        bindings: vec![Binding::new(component, id, BindingTarget::Text(name))],
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

pub(super) fn defs() -> Vec<WindowDef<TestTheme>> {
    ["First", "Second"]
        .into_iter()
        .map(|name| WindowDef {
            name,
            config: WindowConfig::default(),
            view: generated_view,
        })
        .collect()
}

pub(super) fn runtime(theme: Option<TestTheme>) -> Runtime<TestTheme> {
    let component = generated_component(NAMES, None, &[]).expect("component");
    Runtime::new(
        defs(),
        component,
        Rc::new(RefCell::new(Vec::new())),
        theme,
        Rc::new(RefCell::new(
            theme.map(crate::Theme::name).map(str::to_owned),
        )),
        Vec::new(),
        None,
    )
}

pub(super) fn scoped(message: Message) -> ScopedMessage {
    ScopedMessage {
        window: WindowKey(0),
        message,
    }
}

pub(super) fn assembled(
    theme: Option<TestTheme>,
    seeds: &[(&str, Value)],
    script: Option<Script>,
) -> (WidgetId, Ui<TestTheme>) {
    let (component, built, structures) =
        generated_built(generated_view, NAMES, theme, seeds, script).expect("launch succeeds");
    let id = built.widget.id();
    (
        id,
        Ui::assemble(component, built, generated_view, None, structures, None),
    )
}
