use brul_event::EventKind;
use brul_syntax::ast::ExprKind;
use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Element, PropType};
use brul_syntax::script::Program;

use crate::error_at;
use crate::window::HandlerArg;

pub(super) struct Events {
    pub drag: bool,
    pub focusable: bool,
    pub multiline: bool,
    pub modal: bool,
    pub handlers: Vec<(EventKind, String)>,
}

pub(super) fn widget(element: &Element, program: Option<&Program>) -> Result<Events, Error> {
    let mut drag = false;
    let mut focusable = false;
    let mut multiline = false;
    let mut modal = false;
    let mut handlers = Vec::new();
    for arg in &element.args {
        if let Some(enabled) = literal_flag(arg, "focusable")? {
            focusable = enabled;
            continue;
        }
        if let Some(enabled) = literal_flag(arg, "multiline")? {
            multiline = enabled;
            continue;
        }
        if let Some(enabled) = literal_flag(arg, "modal")? {
            modal = enabled;
            continue;
        }
        let Arg::Handler { event, handler } = arg else {
            continue;
        };
        let Ok(event) = event.parse::<EventKind>() else {
            return Err(error_at(
                element.position,
                format!("event '{event}' is not supported yet"),
            ));
        };
        if event == EventKind::Key {
            return Err(error_at(
                element.position,
                "key events are only supported on the window",
            ));
        }
        if event == EventKind::Close {
            return Err(error_at(
                element.position,
                "close events are only supported on the window",
            ));
        }
        if event == EventKind::Drag {
            if handler != "dragWindow" {
                return Err(error_at(
                    element.position,
                    "drag handler must be dragWindow",
                ));
            }
            drag = true;
            continue;
        }
        if handlers
            .iter()
            .any(|item: &(EventKind, String)| item.0 == event)
        {
            return Err(error_at(
                element.position,
                format!("event '{}' is handled twice", event.name()),
            ));
        }
        define(handler, program, params(event), element.position)?;
        handlers.push((event, handler.clone()));
    }
    Ok(Events {
        drag,
        focusable,
        multiline,
        modal,
        handlers,
    })
}

fn params(event: EventKind) -> &'static [PropType] {
    if event.arguments_count() == 0 {
        &[]
    } else {
        &[PropType::Object]
    }
}

fn literal_flag(arg: &Arg, name: &str) -> Result<Option<bool>, Error> {
    let Arg::Named {
        name: parameter,
        value,
    } = arg
    else {
        return Ok(None);
    };
    if parameter != name {
        return Ok(None);
    }
    let ExprKind::Bool(enabled) = value.kind else {
        return Err(error_at(
            value.position,
            format!("parameter '{name}' must be a boolean literal"),
        ));
    };
    Ok(Some(enabled))
}

pub(super) fn window(
    args: &[HandlerArg],
    program: Option<&Program>,
) -> Result<Vec<(EventKind, String)>, Error> {
    let mut handlers = Vec::new();
    for item in args {
        let Ok(event) = item.event.parse::<EventKind>() else {
            return Err(error_at(
                item.position,
                format!("event '{}' is not supported yet", item.event),
            ));
        };
        if !matches!(event, EventKind::Key | EventKind::Close) {
            return Err(error_at(
                item.position,
                format!("window event '{}' is not supported yet", event.name()),
            ));
        }
        if handlers
            .iter()
            .any(|other: &(EventKind, String)| other.0 == event)
        {
            return Err(error_at(
                item.position,
                format!("event '{}' is handled twice", event.name()),
            ));
        }
        define(&item.function, program, params(event), item.position)?;
        handlers.push((event, item.function.clone()));
    }
    Ok(handlers)
}

pub(super) fn define(
    name: &str,
    program: Option<&Program>,
    params: &[PropType],
    position: Position,
) -> Result<(), Error> {
    let function = program.and_then(|program| {
        program
            .functions
            .iter()
            .find(|function| function.name == name)
    });
    let Some(function) = function else {
        return Err(error_at(
            position,
            format!("function '{name}' is not defined"),
        ));
    };
    if function.params.len() != params.len() {
        return Err(error_at(
            position,
            format!(
                "function '{name}' expects {} arguments, got {}",
                params.len(),
                function.params.len()
            ),
        ));
    }
    for (parameter, expected) in function.params.iter().zip(params) {
        if &parameter.ty != expected {
            return Err(crate::arg_mismatch(position, expected, &parameter.ty));
        }
    }
    Ok(())
}
