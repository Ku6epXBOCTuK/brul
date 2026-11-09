use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Element};
use brul_syntax::script::Program;

use crate::error_at;
use crate::runtime::HandlerEvent;
use crate::window::HandlerArg;

pub(super) struct Events {
    pub drag: bool,
    pub handlers: Vec<(HandlerEvent, String)>,
}

pub(super) fn widget(element: &Element, program: Option<&Program>) -> Result<Events, Error> {
    let mut drag = false;
    let mut handlers = Vec::new();
    for arg in &element.args {
        let Arg::Handler { event, handler } = arg else {
            continue;
        };
        let Some(event) = HandlerEvent::parse(event) else {
            return Err(error_at(
                element.position,
                format!("event '{event}' is not supported yet"),
            ));
        };
        if event == HandlerEvent::Key {
            return Err(error_at(
                element.position,
                "key events are only supported on the window",
            ));
        }
        if event == HandlerEvent::Drag {
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
            .any(|item: &(HandlerEvent, String)| item.0 == event)
        {
            return Err(error_at(
                element.position,
                format!("event '{}' is handled twice", event.name()),
            ));
        }
        define(handler, program, element.position)?;
        handlers.push((event, handler.clone()));
    }
    Ok(Events { drag, handlers })
}

pub(super) fn window(
    args: &[HandlerArg],
    program: Option<&Program>,
) -> Result<Vec<(HandlerEvent, String)>, Error> {
    let mut handlers = Vec::new();
    for item in args {
        let Some(event) = HandlerEvent::parse(&item.event) else {
            return Err(error_at(
                item.position,
                format!("event '{}' is not supported yet", item.event),
            ));
        };
        if event != HandlerEvent::Key {
            return Err(error_at(
                item.position,
                format!("window event '{}' is not supported yet", event.name()),
            ));
        }
        if handlers
            .iter()
            .any(|other: &(HandlerEvent, String)| other.0 == event)
        {
            return Err(error_at(
                item.position,
                format!("event '{}' is handled twice", event.name()),
            ));
        }
        define(&item.function, program, item.position)?;
        handlers.push((event, item.function.clone()));
    }
    Ok(handlers)
}

pub(super) fn define(
    name: &str,
    program: Option<&Program>,
    position: Position,
) -> Result<(), Error> {
    let declared = program.is_some_and(|program| {
        program
            .functions
            .iter()
            .any(|function| function.name == name)
    });
    if declared {
        Ok(())
    } else {
        Err(error_at(
            position,
            format!("function '{name}' is not defined"),
        ))
    }
}
