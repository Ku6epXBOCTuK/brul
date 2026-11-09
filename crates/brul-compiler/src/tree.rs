use brul_gui::{Direction, LayoutStyle, Widget, WidgetId};
use brul_script::{Component, Value, Variable, eval};
use brul_syntax::ast::{Expr, Position};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, BlockItem, EachBlock, Element, IfBlock};
use brul_syntax::style::Stylesheet;

use crate::styles::{self, Resolved};

#[derive(Debug, Clone)]
pub struct Binding {
    pub widget: WidgetId,
    pub target: BindingTarget,
    pub scopes: Vec<Vec<Variable>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerEvent {
    Click,
    Press,
    Release,
    Enter,
    Leave,
    Drag,
    Key,
}

impl HandlerEvent {
    pub fn name(self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::Press => "press",
            Self::Release => "release",
            Self::Enter => "enter",
            Self::Leave => "leave",
            Self::Drag => "drag",
            Self::Key => "key",
        }
    }

    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "click" => Some(Self::Click),
            "press" => Some(Self::Press),
            "release" => Some(Self::Release),
            "enter" => Some(Self::Enter),
            "leave" => Some(Self::Leave),
            "drag" => Some(Self::Drag),
            "key" => Some(Self::Key),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Handler {
    pub widget: Option<WidgetId>,
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

#[derive(Debug, Clone)]
pub struct Structure {
    pub parent: WidgetId,
    pub direction: Direction,
    pub spacing: u32,
    pub kind: StructureKind,
}

#[derive(Debug, Clone)]
pub struct RowInfo {
    pub key: Option<Value>,
    pub item: Value,
    pub start: usize,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub enum StructureKind {
    If {
        block: IfBlock,
        last: bool,
    },
    Each {
        block: EachBlock,
        last: Vec<Value>,
        rows: Vec<RowInfo>,
    },
}

pub struct Built {
    pub widget: Widget,
    pub bindings: Vec<Binding>,
    pub handlers: Vec<Handler>,
    pub structures: Vec<Structure>,
}

type Parts = (Widget, Vec<Binding>, Vec<Handler>, Vec<Structure>);
type Children = (Vec<Widget>, Vec<Binding>, Vec<Handler>, Vec<Structure>);

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

    let (widget, bindings, handlers, structures) = match element.name.as_str() {
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
                scopes: component.build_scopes(),
            };
            (widget, vec![binding], Vec::new(), Vec::new())
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
    built.structures.extend(structures);
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
                        scopes: component.build_scopes(),
                    });
                }
                "selected" => {
                    widget = widget.selected(boolean(component, value, name)?);
                    bindings.push(Binding {
                        widget: id,
                        target: BindingTarget::Selected(value.clone()),
                        scopes: component.build_scopes(),
                    });
                }
                "disabled" => {
                    widget = widget.disabled(boolean(component, value, name)?);
                    bindings.push(Binding {
                        widget: id,
                        target: BindingTarget::Disabled(value.clone()),
                        scopes: component.build_scopes(),
                    });
                }
                "visible" => {
                    widget = widget.visible(boolean(component, value, name)?);
                    bindings.push(Binding {
                        widget: id,
                        target: BindingTarget::Visible(value.clone()),
                        scopes: component.build_scopes(),
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
                    widget = widget.draggable(true);
                    continue;
                }
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
                    widget: Some(id),
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
            scopes: component.build_scopes(),
        });
    }
    Ok(Built {
        widget,
        bindings,
        handlers,
        structures: Vec::new(),
    })
}

fn container(
    direction: Direction,
    element: &Element,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
    component: &mut Component,
    resolved: Resolved,
) -> Result<Parts, Error> {
    let built = build_items(
        &element.block.items,
        stylesheet,
        theme,
        component,
        element.position,
        direction,
        resolved.layout.spacing,
    )?;
    let mut children = Vec::with_capacity(built.len());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut structures = Vec::new();
    for child in built {
        children.push(child.widget);
        bindings.extend(child.bindings);
        handlers.extend(child.handlers);
        structures.extend(child.structures);
    }
    let widget = Widget::view(direction, children)
        .with_layout(resolved.layout)
        .with_styles(resolved.styles);
    Ok((widget, bindings, handlers, structures))
}

fn build_items(
    items: &[BlockItem],
    stylesheet: &Stylesheet,
    theme: Option<&str>,
    component: &mut Component,
    position: Position,
    direction: Direction,
    spacing: u32,
) -> Result<Vec<Built>, Error> {
    let mut built = Vec::new();
    for item in items {
        match item {
            BlockItem::Element(child) => {
                built.push(build_bound_tree(child, stylesheet, theme, component)?);
            }
            BlockItem::If(block) => {
                let (mut node, last) = build_if(
                    block, stylesheet, theme, component, position, direction, spacing,
                )?;
                node.structures.push(Structure {
                    parent: node.widget.id(),
                    direction,
                    spacing,
                    kind: StructureKind::If {
                        block: block.clone(),
                        last,
                    },
                });
                built.push(node);
            }
            BlockItem::Each(block) => {
                let (mut node, last, rows) = build_each(
                    block, stylesheet, theme, component, position, direction, spacing,
                )?;
                node.structures.push(Structure {
                    parent: node.widget.id(),
                    direction,
                    spacing,
                    kind: StructureKind::Each {
                        block: block.clone(),
                        last,
                        rows,
                    },
                });
                built.push(node);
            }
            BlockItem::Expr(expression) => {
                return Err(error_at(
                    expression.position,
                    "unexpected expression in a container",
                ));
            }
        }
    }
    Ok(built)
}

fn build_if(
    block: &IfBlock,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
    component: &mut Component,
    position: Position,
    direction: Direction,
    spacing: u32,
) -> Result<(Built, bool), Error> {
    let condition = match evaluate(component, &block.condition)? {
        Value::Bool(condition) => condition,
        other => {
            return Err(error_at(
                block.condition.position,
                format!("if condition expects Bool, got {}", other.type_name()),
            ));
        }
    };
    let branch = if condition {
        Some(&block.then_block)
    } else {
        block.else_block.as_ref()
    };
    let (children, bindings, handlers, structures) = match branch {
        Some(branch) => flatten(build_items(
            &branch.items,
            stylesheet,
            theme,
            component,
            position,
            direction,
            spacing,
        )?),
        None => (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
    };
    let mut wrapper = Widget::view(direction, Vec::new()).with_layout(LayoutStyle {
        spacing,
        ..Default::default()
    });
    wrapper.set_children(children);
    Ok((
        Built {
            widget: wrapper,
            bindings,
            handlers,
            structures,
        },
        condition,
    ))
}

fn flatten(built: Vec<Built>) -> Children {
    let mut children = Vec::with_capacity(built.len());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut structures = Vec::new();
    for child in built {
        children.push(child.widget);
        bindings.extend(child.bindings);
        handlers.extend(child.handlers);
        structures.extend(child.structures);
    }
    (children, bindings, handlers, structures)
}

fn build_each(
    block: &EachBlock,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
    component: &mut Component,
    position: Position,
    direction: Direction,
    spacing: u32,
) -> Result<(Built, Vec<Value>, Vec<RowInfo>), Error> {
    let items = match evaluate(component, &block.collection)? {
        Value::Array(items) => items,
        other => {
            return Err(error_at(
                block.collection.position,
                format!("each expects Array, got {}", other.type_name()),
            ));
        }
    };
    let last = items.clone();
    let mut all = Vec::new();
    let mut rows = Vec::new();
    for (index, item) in items.into_iter().enumerate() {
        let mut variables = vec![Variable {
            name: block.item.clone(),
            value: item.clone(),
        }];
        if let Some(name) = &block.index {
            variables.push(Variable {
                name: name.clone(),
                value: Value::Number(index as f64),
            });
        }
        component.push_build_scope(&variables);
        let outcome = (|| -> Result<(Option<Value>, Vec<Built>), Error> {
            let key = match &block.key {
                Some(expression) => Some(evaluate(component, expression)?),
                None => None,
            };
            let built = build_items(
                &block.block.items,
                stylesheet,
                theme,
                component,
                position,
                direction,
                spacing,
            )?;
            Ok((key, built))
        })();
        component.pop_build_scope();
        let (key, row) = outcome?;
        let start = all.len();
        all.extend(row);
        rows.push(RowInfo {
            key,
            item,
            start,
            count: all.len() - start,
        });
    }
    let (children, bindings, handlers, structures) = flatten(all);
    let mut wrapper = Widget::view(direction, Vec::new()).with_layout(LayoutStyle {
        spacing,
        ..Default::default()
    });
    wrapper.set_children(children);
    Ok((
        Built {
            widget: wrapper,
            bindings,
            handlers,
            structures,
        },
        last,
        rows,
    ))
}

pub fn rebuild_structure(
    structure: &Structure,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
    component: &mut Component,
) -> Result<(Built, Value, Vec<RowInfo>), Error> {
    match &structure.kind {
        StructureKind::If { block, .. } => {
            let (built, condition) = build_if(
                block,
                stylesheet,
                theme,
                component,
                block.condition.position,
                structure.direction,
                structure.spacing,
            )?;
            Ok((built, Value::Bool(condition), Vec::new()))
        }
        StructureKind::Each { block, .. } => {
            let (built, items, rows) = build_each(
                block,
                stylesheet,
                theme,
                component,
                block.collection.position,
                structure.direction,
                structure.spacing,
            )?;
            Ok((built, Value::Array(items), rows))
        }
    }
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
