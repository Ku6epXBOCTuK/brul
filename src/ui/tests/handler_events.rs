use super::*;
pub(super) fn handler_dispatch(
    component: &mut Component,
    name: &str,
    arguments: Vec<Dyn>,
) -> Result<Dyn, Error> {
    match name {
        "clicked" | "closing" | "tick" | "first" => {
            component.assign("ran", Value::Bool(true))?;
        }
        "second" => component.assign("off", Value::Bool(true))?,
        "pressed" => component.assign("seen", Value::from(arguments[0].clone()))?,
        _ => return Err(Error::UnknownFunction(name.to_owned())),
    }
    Ok(Dyn::null())
}

pub(super) fn component_with(declared: &[&str]) -> ComponentHandle {
    let mut component = Component::new();
    component.set_names(NAMES);
    for name in declared {
        component
            .declare(name, Value::Null, false)
            .expect("declared");
    }
    component.set_dispatch(handler_dispatch);
    component.register_event_call("tick", set_ran);
    component.register_event_call("applied", applied_call);
    component.register_event_call("schedule", schedule_tick);
    Rc::new(RefCell::new(component))
}

pub(super) fn noop_init(_: &mut Component) -> Result<(), Error> {
    Ok(())
}

pub(super) fn noop_dispatch(_: &mut Component, _: &str, _: Vec<Dyn>) -> Result<Dyn, Error> {
    Ok(Dyn::null())
}

pub(super) fn raise_dispatch(
    component: &mut Component,
    name: &str,
    arguments: Vec<Dyn>,
) -> Result<Dyn, Error> {
    if name == "raise" {
        let arguments = arguments.into_iter().map(Value::from).collect();
        Ok(Dyn::from(brul_script::eval::call(
            component, "clicked", arguments,
        )?))
    } else {
        Err(Error::UnknownFunction(name.to_owned()))
    }
}

pub(super) fn applied_dispatch(
    component: &mut Component,
    name: &str,
    _: Vec<Dyn>,
) -> Result<Dyn, Error> {
    if name == "applied" {
        component.assign("label", Value::String("after".into()))?;
        Ok(Dyn::null())
    } else {
        Err(Error::UnknownFunction(name.to_owned()))
    }
}

pub(super) fn set_ran(component: &mut Component, _: &[Dyn]) -> Result<Dyn, Error> {
    component.assign("ran", Value::Bool(true))?;
    Ok(Dyn::null())
}

pub(super) fn schedule_tick(component: &mut Component, _: &[Dyn]) -> Result<Dyn, Error> {
    component.every(1000.0, "tick")?;
    Ok(Dyn::null())
}

pub(super) fn set_off(component: &mut Component, _: &[Dyn]) -> Result<Dyn, Error> {
    component.assign("off", Value::Bool(true))?;
    Ok(Dyn::null())
}

pub(super) fn key_code(component: &mut Component, arguments: &[Dyn]) -> Result<Dyn, Error> {
    component.assign("seen", Value::from(arguments[0].clone()))?;
    Ok(Dyn::null())
}

pub(super) fn raise_call(component: &mut Component, arguments: &[Dyn]) -> Result<Dyn, Error> {
    raise_dispatch(component, "raise", arguments.to_vec())
}

pub(super) fn applied_call(component: &mut Component, _: &[Dyn]) -> Result<Dyn, Error> {
    component.assign("label", Value::String("after".into()))?;
    Ok(Dyn::null())
}

pub(super) fn script_with(init: fn(&mut Component) -> Result<(), Error>) -> Script {
    Script::Generated {
        init,
        dispatch: noop_dispatch,
    }
}

pub(super) fn shown_false(component: &mut Component) -> Result<(), Error> {
    component.declare("shown", Value::Bool(false), false)
}

pub(super) fn shown_and_label(component: &mut Component) -> Result<(), Error> {
    component.declare("shown", Value::Bool(false), false)?;
    component.declare("label", Value::String("bound".into()), false)
}

pub(super) fn dispatching(
    component: &ComponentHandle,
    widget: Widget,
    handlers: impl FnOnce(WidgetId) -> Vec<Handler>,
) -> (WidgetId, Ui<TestTheme>) {
    let id = widget.id();
    let built = Built {
        widget,
        bindings: Vec::new(),
        handlers: handlers(id),
        props: Vec::new(),
    };
    (
        id,
        Ui::assemble(
            Rc::clone(component),
            built,
            themed_view,
            None,
            Vec::new(),
            None,
        ),
    )
}
#[test]
pub(super) fn a_close_handler_keeps_the_window_open() {
    let component = component_with(&["ran"]);
    let (_, mut ui) = dispatching(&component, Widget::text(""), |_| {
        vec![Handler::new(&component, None, EventKind::Close, set_ran)]
    });

    assert!(
        ui.dispatch(&[UiEvent::window(Event::close()).into()])
            .is_empty()
    );
    assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
}

#[test]
pub(super) fn a_close_without_a_handler_closes_the_window() {
    let component = component_with(&[]);
    let (_, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());

    assert_eq!(
        ui.dispatch(&[UiEvent::window(Event::close()).into()]),
        vec![WindowCommand::Close]
    );
}
