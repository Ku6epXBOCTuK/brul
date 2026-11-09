use brul_gui::{Direction, Widget};
use brul_script::{Component, Value, eval};
use brul_syntax::ast::{Expr, Position};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, BlockItem, Element};
use brul_syntax::style::Stylesheet;

use crate::styles::{self, Resolved};

pub fn build_tree(
    element: &Element,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
    component: &mut Component,
) -> Result<Widget, Error> {
    let resolved = resolve(element, stylesheet, theme)?;
    let parameters = parameters(element, component)?;

    let widget = match element.name.as_str() {
        "View" => container(
            Direction::Column,
            element,
            stylesheet,
            theme,
            component,
            resolved,
        )?,
        "Row" => container(
            Direction::Row,
            element,
            stylesheet,
            theme,
            component,
            resolved,
        )?,
        "Column" => container(
            Direction::Column,
            element,
            stylesheet,
            theme,
            component,
            resolved,
        )?,
        "Text" => text(element, component, resolved)?,
        "Image" => {
            return Err(error_at(
                element.position,
                "widget 'Image' is not supported yet",
            ));
        }
        name => {
            return Err(error_at(
                element.position,
                format!("unknown widget '{name}'"),
            ));
        }
    };
    Ok(parameters.apply(widget))
}

fn resolve(
    element: &Element,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
) -> Result<Resolved, Error> {
    let classes: Vec<&str> = element
        .args
        .iter()
        .filter_map(|arg| match arg {
            Arg::Class(class) => Some(class.as_str()),
            _ => None,
        })
        .collect();
    styles::resolve(stylesheet, &classes, theme)
        .map_err(|message| error_at(element.position, message))
}

#[derive(Default)]
struct Parameters {
    checked: Option<bool>,
    selected: Option<bool>,
    disabled: Option<bool>,
    visible: Option<bool>,
    x: Option<i32>,
    y: Option<i32>,
}

impl Parameters {
    fn apply(self, widget: Widget) -> Widget {
        let mut widget = widget;
        if let Some(value) = self.checked {
            widget = widget.checked(value);
        }
        if let Some(value) = self.selected {
            widget = widget.selected(value);
        }
        if let Some(value) = self.disabled {
            widget = widget.disabled(value);
        }
        if let Some(value) = self.visible {
            widget = widget.visible(value);
        }
        if self.x.is_some() || self.y.is_some() {
            widget = widget.with_offset(self.x.unwrap_or(0), self.y.unwrap_or(0));
        }
        widget
    }
}

fn parameters(element: &Element, component: &mut Component) -> Result<Parameters, Error> {
    let mut parameters = Parameters::default();
    for arg in &element.args {
        match arg {
            Arg::Class(_) => {}
            Arg::Named { name, value } => match name.as_str() {
                "checked" => parameters.checked = Some(boolean(component, value, name)?),
                "selected" => parameters.selected = Some(boolean(component, value, name)?),
                "disabled" => parameters.disabled = Some(boolean(component, value, name)?),
                "visible" => parameters.visible = Some(boolean(component, value, name)?),
                "x" => parameters.x = Some(integer(component, value, name)?),
                "y" => parameters.y = Some(integer(component, value, name)?),
                _ => {
                    return Err(error_at(
                        value.position,
                        format!("parameter '{name}' is not supported yet"),
                    ));
                }
            },
            Arg::Handler { .. } => {
                return Err(error_at(element.position, "handlers are not supported yet"));
            }
        }
    }
    Ok(parameters)
}

fn container(
    direction: Direction,
    element: &Element,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
    component: &mut Component,
    resolved: Resolved,
) -> Result<Widget, Error> {
    let mut children = Vec::with_capacity(element.block.items.len());
    for item in &element.block.items {
        match item {
            BlockItem::Element(child) => {
                children.push(build_tree(child, stylesheet, theme, component)?);
            }
            BlockItem::If(_) => {
                return Err(error_at(element.position, "'if' is not supported yet"));
            }
            BlockItem::Each(_) => {
                return Err(error_at(element.position, "'each' is not supported yet"));
            }
            BlockItem::Expr(expression) => {
                return Err(error_at(
                    expression.position,
                    "unexpected expression in a container",
                ));
            }
        }
    }
    Ok(Widget::view(direction, children)
        .with_layout(resolved.layout)
        .with_styles(resolved.styles))
}

fn text(element: &Element, component: &mut Component, resolved: Resolved) -> Result<Widget, Error> {
    match element.block.items.as_slice() {
        [BlockItem::Expr(expression)] => match evaluate(component, expression)? {
            Value::String(content) => Ok(Widget::styled_text(content, resolved.text)
                .with_layout(resolved.layout)
                .with_styles(resolved.styles)),
            other => Err(error_at(
                expression.position,
                format!("Text content must be a string, got {}", other.type_name()),
            )),
        },
        _ => Err(error_at(
            element.position,
            "Text requires a single content expression",
        )),
    }
}

fn boolean(component: &mut Component, expression: &Expr, name: &str) -> Result<bool, Error> {
    match evaluate(component, expression)? {
        Value::Bool(value) => Ok(value),
        other => Err(error_at(
            expression.position,
            format!("parameter '{name}' expects Bool, got {}", other.type_name()),
        )),
    }
}

fn integer(component: &mut Component, expression: &Expr, name: &str) -> Result<i32, Error> {
    match evaluate(component, expression)? {
        Value::Number(value)
            if value.is_finite()
                && value.fract() == 0.0
                && value >= f64::from(i32::MIN)
                && value <= f64::from(i32::MAX) =>
        {
            Ok(value as i32)
        }
        _ => Err(error_at(
            expression.position,
            format!("parameter '{name}' expects an integer Number"),
        )),
    }
}

fn evaluate(component: &mut Component, expression: &Expr) -> Result<Value, Error> {
    eval(component, expression).map_err(script_error)
}

pub(crate) fn script_error(error: brul_script::Error) -> Error {
    let position = error.position().unwrap_or_default();
    Error {
        message: error.reason(),
        line: position.line,
        column: position.column,
    }
}

fn error_at(position: Position, message: impl Into<String>) -> Error {
    Error {
        message: message.into(),
        line: position.line,
        column: position.column,
    }
}
