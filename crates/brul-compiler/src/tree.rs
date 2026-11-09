use brul_gui::{Direction, Widget, WidgetId};
use brul_script::{Component, Value, eval};
use brul_syntax::ast::{Expr, Position};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, BlockItem, Element};
use brul_syntax::style::Stylesheet;

use crate::styles::{self, Resolved};

#[derive(Debug, Clone)]
pub struct Binding {
    pub widget: WidgetId,
    pub target: BindingTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerEvent {
    Click,
    Press,
    Release,
    Enter,
    Leave,
}

impl HandlerEvent {
    pub fn name(self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::Press => "press",
            Self::Release => "release",
            Self::Enter => "enter",
            Self::Leave => "leave",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        match name {
            "click" => Some(Self::Click),
            "press" => Some(Self::Press),
            "release" => Some(Self::Release),
            "enter" => Some(Self::Enter),
            "leave" => Some(Self::Leave),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Handler {
    pub widget: WidgetId,
    pub event: HandlerEvent,
    pub function: String,
}

#[derive(Debug, Clone)]
pub enum BindingTarget {
    Text(Expr),
    Checked(Expr),
    Selected(Expr),
    Disabled(Expr),
    Visible(Expr),
    Offset { x: Option<Expr>, y: Option<Expr> },
}

pub struct Built {
    pub widget: Widget,
    pub bindings: Vec<Binding>,
    pub handlers: Vec<Handler>,
}

pub fn build_tree(
    element: &Element,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
    component: &mut Component,
) -> Result<Widget, Error> {
    Ok(build_bound_tree(element, stylesheet, theme, component)?.widget)
}

pub fn build_bound_tree(
    element: &Element,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
    component: &mut Component,
) -> Result<Built, Error> {
    let resolved = resolve(element, stylesheet, theme)?;

    let (widget, bindings, handlers) = match element.name.as_str() {
        "View" | "Row" | "Column" => {
            let direction = if element.name == "Row" {
                Direction::Row
            } else {
                Direction::Column
            };
            container(direction, element, stylesheet, theme, component, resolved)?
        }
        "Text" => {
            let (widget, expression) = text(element, component, resolved)?;
            let binding = Binding {
                widget: widget.id(),
                target: BindingTarget::Text(expression),
            };
            (widget, vec![binding], Vec::new())
        }
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

    let mut built = parameters(element, component, widget)?;
    built.bindings.extend(bindings);
    built.handlers.extend(handlers);
    Ok(built)
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

fn parameters(
    element: &Element,
    component: &mut Component,
    widget: Widget,
) -> Result<Built, Error> {
    let mut widget = widget;
    let id = widget.id();
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut x_value = None;
    let mut y_value = None;
    let mut x = None;
    let mut y = None;
    for arg in &element.args {
        match arg {
            Arg::Class(_) => {}
            Arg::Named { name, value } => match name.as_str() {
                "checked" => {
                    widget = widget.checked(boolean(component, value, name)?);
                    bindings.push(Binding {
                        widget: id,
                        target: BindingTarget::Checked(value.clone()),
                    });
                }
                "selected" => {
                    widget = widget.selected(boolean(component, value, name)?);
                    bindings.push(Binding {
                        widget: id,
                        target: BindingTarget::Selected(value.clone()),
                    });
                }
                "disabled" => {
                    widget = widget.disabled(boolean(component, value, name)?);
                    bindings.push(Binding {
                        widget: id,
                        target: BindingTarget::Disabled(value.clone()),
                    });
                }
                "visible" => {
                    widget = widget.visible(boolean(component, value, name)?);
                    bindings.push(Binding {
                        widget: id,
                        target: BindingTarget::Visible(value.clone()),
                    });
                }
                "x" => {
                    x_value = Some(integer(component, value, name)?);
                    x = Some(value.clone());
                }
                "y" => {
                    y_value = Some(integer(component, value, name)?);
                    y = Some(value.clone());
                }
                _ => {
                    return Err(error_at(
                        value.position,
                        format!("parameter '{name}' is not supported yet"),
                    ));
                }
            },
            Arg::Handler { event, handler } => {
                let Some(event) = HandlerEvent::parse(event) else {
                    return Err(error_at(
                        element.position,
                        format!("event '{event}' is not supported yet"),
                    ));
                };
                if handlers.iter().any(|item: &Handler| item.event == event) {
                    return Err(error_at(
                        element.position,
                        format!("event '{}' is handled twice", event.name()),
                    ));
                }
                if !component.has_function(handler) {
                    return Err(error_at(
                        element.position,
                        format!("function '{handler}' is not defined"),
                    ));
                }
                handlers.push(Handler {
                    widget: id,
                    event,
                    function: handler.clone(),
                });
            }
        }
    }
    if x.is_some() || y.is_some() {
        widget = widget.with_offset(x_value.unwrap_or(0), y_value.unwrap_or(0));
        bindings.push(Binding {
            widget: id,
            target: BindingTarget::Offset { x, y },
        });
    }
    Ok(Built {
        widget,
        bindings,
        handlers,
    })
}

fn container(
    direction: Direction,
    element: &Element,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
    component: &mut Component,
    resolved: Resolved,
) -> Result<(Widget, Vec<Binding>, Vec<Handler>), Error> {
    let mut children = Vec::with_capacity(element.block.items.len());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    for item in &element.block.items {
        match item {
            BlockItem::Element(child) => {
                let child = build_bound_tree(child, stylesheet, theme, component)?;
                children.push(child.widget);
                bindings.extend(child.bindings);
                handlers.extend(child.handlers);
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
    let widget = Widget::view(direction, children)
        .with_layout(resolved.layout)
        .with_styles(resolved.styles);
    Ok((widget, bindings, handlers))
}

fn text(
    element: &Element,
    component: &mut Component,
    resolved: Resolved,
) -> Result<(Widget, Expr), Error> {
    match element.block.items.as_slice() {
        [BlockItem::Expr(expression)] => match evaluate(component, expression)? {
            Value::String(content) => Ok((
                Widget::styled_text(content, resolved.text)
                    .with_layout(resolved.layout)
                    .with_styles(resolved.styles),
                expression.clone(),
            )),
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
