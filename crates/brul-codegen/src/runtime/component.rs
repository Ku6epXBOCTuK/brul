use std::cell::RefCell;
use std::rc::Rc;

use brul_script::{Component, Error as ScriptError, Script, Value};
use brul_syntax::TypeName;
use brul_syntax::markup::PropType;
use brul_syntax::style::Stylesheet;

use crate::{Import, Registry};

use super::binding::{PropBinding, PropExpression};
use super::build::Build;
use super::error::ViewError;
use super::event::{ComponentEvent, EventHost};
use super::slot::{SlotArgument, Slots};
use super::structure::{Built, View};

pub type ComponentHandle = Rc<RefCell<Component>>;

#[derive(Clone)]
pub struct Context {
    pub registry: Rc<Registry>,
    pub imports: Vec<Import>,
    pub stylesheet: Rc<Stylesheet>,
}

pub struct PropArgument {
    pub name: &'static str,
    pub ty: PropType,
    pub expression: PropExpression,
}

pub struct Instance<'a> {
    pub theme: Option<&'a str>,
    pub view: View,
    pub script: Option<Script>,
    pub name: &'static str,
    pub names: &'static [&'static str],
    pub props: &'a [PropArgument],
    pub defaults: &'a [PropArgument],
    pub events: &'a [ComponentEvent],
    pub slots: &'a [SlotArgument],
}

pub fn instance(
    build: &mut Build,
    parent: &ComponentHandle,
    instance: Instance<'_>,
) -> Result<Built, ViewError> {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    build.register_component(Rc::clone(&component));
    component.borrow_mut().set_names(instance.names);
    start(&component, &instance)?;
    let given = declare_props(&component, parent, instance.props, instance.name)?;
    declare_defaults(&component, instance.defaults, &given, instance.name)?;
    let slots = (!instance.slots.is_empty()).then(|| {
        Rc::new(Slots {
            component: Rc::clone(parent),
            scope: parent.borrow().scope().clone(),
            parent: build.slots(),
            entries: instance.slots.to_vec(),
        })
    });
    let previous = build.replace_slots(slots);
    let rendered = (instance.view)(instance.theme, build, &component);
    build.replace_slots(previous);
    let mut built = rendered?;
    bind(&mut built, parent, &component, instance.props);
    Ok(built)
}

fn start(component: &ComponentHandle, instance: &Instance<'_>) -> Result<(), ViewError> {
    if !instance.events.is_empty() {
        component.borrow_mut().set_host(EventHost {
            events: instance
                .events
                .iter()
                .map(|event| ComponentEvent {
                    name: event.name,
                    call: event.call,
                    params: event.params,
                    component: Rc::clone(&event.component),
                })
                .collect(),
        });
    }
    match instance.script {
        Some(Script::Generated { init, dispatch }) => {
            component.borrow_mut().set_dispatch(dispatch);
            init(&mut component.borrow_mut()).map_err(ViewError::from)
        }
        Some(Script::Typed {
            init,
            call,
            dispatch,
        }) => {
            let mut component = component.borrow_mut();
            component.set_dispatch(dispatch);
            component.set_typed_call(call);
            init(&mut component).map_err(ViewError::from)
        }
        None => Ok(()),
    }
}

fn declare_props(
    component: &ComponentHandle,
    parent: &ComponentHandle,
    props: &[PropArgument],
    name: &str,
) -> Result<Vec<&'static str>, ViewError> {
    let mut given = Vec::new();
    for argument in props {
        let value = evaluate_argument(parent, argument)?;
        assign(component, argument, value, name)?;
        given.push(argument.name);
    }
    Ok(given)
}

fn declare_defaults(
    component: &ComponentHandle,
    defaults: &[PropArgument],
    given: &[&str],
    name: &str,
) -> Result<(), ViewError> {
    for argument in defaults {
        if given.contains(&argument.name) {
            continue;
        }
        let value = evaluate_argument(component, argument)?;
        assign(component, argument, value, name)?;
    }
    Ok(())
}

fn bind(
    built: &mut Built,
    parent: &ComponentHandle,
    component: &ComponentHandle,
    props: &[PropArgument],
) {
    let widget = built.widget.id();
    for argument in props {
        built.props.push(PropBinding {
            widget,
            target: Rc::clone(component),
            name: argument.name,
            ty: argument.ty.clone(),
            expression: argument.expression,
            scope: parent.borrow().scope().clone(),
            component: Rc::clone(parent),
        });
    }
}

fn evaluate_argument(
    component: &ComponentHandle,
    argument: &PropArgument,
) -> Result<Value, ViewError> {
    argument
        .expression
        .evaluate(&mut component.borrow_mut())
        .map_err(ViewError::from)
}

fn assign(
    component: &ComponentHandle,
    argument: &PropArgument,
    value: Value,
    name: &str,
) -> Result<(), ViewError> {
    if !accepts_prop(&value, &argument.ty) {
        return Err(ViewError::expected(
            format!("property '{}' of component '{name}'", argument.name),
            &argument.ty,
            value.type_name(),
        ));
    }
    let id = component
        .borrow()
        .id(argument.name)
        .ok_or_else(|| ScriptError::Undefined(argument.name.to_owned()))?;
    component
        .borrow_mut()
        .declare_property(id, value)
        .map_err(ViewError::from)
}

pub fn accepts_prop(value: &Value, ty: &PropType) -> bool {
    match ty {
        PropType::Bool => matches!(value, Value::Bool(_)),
        PropType::Number => matches!(value, Value::Number(_)),
        PropType::String => matches!(value, Value::String(_)),
        PropType::Image => matches!(value, Value::Null | Value::Image(_) | Value::ImageBytes(_)),
        PropType::Null => matches!(value, Value::Null),
        PropType::Object => matches!(value, Value::Object(_)),
        PropType::Array(_) => matches!(value, Value::Array(_)),
        PropType::Timer => matches!(value, Value::Timer(_)),
        PropType::Void => false,
    }
}
