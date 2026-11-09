use brul_gui::window::WindowConfig;
use brul_syntax::ast::{Expr, ExprKind, Position};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Element, Member, Window};

use crate::error_at;

pub struct HandlerArg {
    pub event: String,
    pub function: String,
    pub position: Position,
}

pub fn arguments(window: &Window) -> Result<(WindowConfig, Vec<HandlerArg>), Error> {
    let mut config = WindowConfig::default();
    let mut handlers = Vec::new();
    for arg in &window.args {
        match arg {
            Arg::Named { name, value } => match name.as_str() {
                "title" => config.title = string(value, "title")?,
                "width" => config.width = size(value, "width")?,
                "height" => config.height = size(value, "height")?,
                "frameless" => flag(value, "frameless")?,
                "transparent" => flag(value, "transparent")?,
                _ => {
                    return Err(error_at(
                        value.position,
                        format!("unknown window argument '{name}'"),
                    ));
                }
            },
            Arg::Handler { event, handler } => handlers.push(HandlerArg {
                event: event.clone(),
                function: handler.clone(),
                position: window.position,
            }),
            Arg::Class(_) => {
                return Err(error_at(window.position, "window arguments must be named"));
            }
        }
    }
    Ok((config, handlers))
}

pub fn root(window: &Window) -> Result<&Element, Error> {
    let mut roots = window.members.iter().filter_map(|member| match member {
        Member::Element(element) => Some(element),
        Member::Prop(_) | Member::Event(_) => None,
    });
    let root = roots
        .next()
        .ok_or_else(|| error_at(window.position, "window has no root element"))?;
    if roots.next().is_some() {
        return Err(error_at(
            window.position,
            "window has more than one root element",
        ));
    }
    Ok(root)
}

fn string(value: &Expr, name: &str) -> Result<String, Error> {
    match &value.kind {
        ExprKind::String(text) => Ok(text.clone()),
        _ => Err(error_at(
            value.position,
            format!("window argument '{name}' must be a string"),
        )),
    }
}

fn size(value: &Expr, name: &str) -> Result<u32, Error> {
    match &value.kind {
        ExprKind::Number(number)
            if number.is_finite()
                && *number > 0.0
                && number.fract() == 0.0
                && *number <= f64::from(u32::MAX) =>
        {
            Ok(*number as u32)
        }
        _ => Err(error_at(
            value.position,
            format!("window argument '{name}' must be a positive integer"),
        )),
    }
}

fn flag(value: &Expr, name: &str) -> Result<(), Error> {
    match &value.kind {
        ExprKind::Bool(true) => Ok(()),
        ExprKind::Bool(false) => Err(error_at(value.position, format!("window is always {name}"))),
        _ => Err(error_at(
            value.position,
            format!("window argument '{name}' must be a bool"),
        )),
    }
}
