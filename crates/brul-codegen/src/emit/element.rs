use std::fmt::Write;

use brul_gui::TextStyleSet;
use brul_script::Names;
use brul_syntax::ast::{Expr, ExprKind};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Block, BlockItem, Element};

use crate::error_at;
use crate::styles;

use super::Emit;
use super::Frame;
use super::style::{layout, style_set, text_style_set};
use super::{component, events, expression, structure};
use crate::runtime::HandlerEvent;

pub(super) fn widget(
    out: &mut String,
    element: &Element,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    if let Some(index) = emit
        .imports
        .iter()
        .position(|import| import.name == element.name)
    {
        return component::component(out, element, index, emit, frame, counter);
    }
    let resolved = styles::resolve(emit.stylesheet, &classes(element), frame.theme())
        .map_err(|message| error_at(element.position, message))?;
    let events = events::widget(element, emit.program)?;
    let name = format!("widget{}", *counter);
    *counter += 1;
    let pad = "    ".repeat(frame.depth());
    let inner = "    ".repeat(frame.depth() + 1);
    let _ = writeln!(out, "{pad}{{");
    let _ = write!(out, "{inner}let {name} = ");
    let text = kind(out, element, &resolved, emit, frame, counter)?;
    modifiers(out, &resolved, events.drag, &inner);
    out.push_str(";\n");
    text_binding(out, text, &name, &inner, emit.names);
    parameters(out, element, &name, &inner, emit.names)?;
    handler_bindings(out, &events.handlers, &name, &inner);
    let _ = writeln!(out, "{inner}{name}");
    let _ = write!(out, "{pad}}}");
    Ok(())
}

fn classes(element: &Element) -> Vec<&str> {
    element
        .args
        .iter()
        .filter_map(|arg| match arg {
            Arg::Class(class) => Some(class.as_str()),
            Arg::Named { .. } | Arg::Handler { .. } => None,
        })
        .collect()
}

fn kind(
    out: &mut String,
    element: &Element,
    resolved: &styles::Resolved,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<Option<Expr>, Error> {
    match element.name.as_str() {
        "View" | "Row" | "Column" => {
            container(out, element, resolved, emit, frame, counter)?;
            Ok(None)
        }
        "Text" => text_widget(out, element, &resolved.text),
        "Image" => image_widget(out, element, emit),
        name => Err(error_at(
            element.position,
            format!("widget '{name}' is not generated yet"),
        )),
    }
}

fn container(
    out: &mut String,
    element: &Element,
    resolved: &styles::Resolved,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    let row = element.name == "Row";
    let call = if row { "row" } else { "column" };
    if element.block.items.is_empty() {
        let _ = write!(out, "brul_gui::Widget::{call}(vec![])");
        return Ok(());
    }
    let direction = if row { "Row" } else { "Column" };
    let _ = writeln!(out, "brul_gui::Widget::{call}(vec![");
    let children = frame.deeper(2).inside(direction, resolved.layout.spacing);
    items(out, &element.block, emit, children, counter)?;
    let inner = "    ".repeat(frame.depth() + 1);
    let _ = write!(out, "{inner}])");
    Ok(())
}

fn modifiers(out: &mut String, resolved: &styles::Resolved, drag: bool, inner: &str) {
    if let Some(layout) = layout(resolved.layout) {
        let _ = write!(out, "\n{inner}.with_layout({layout})");
    }
    if let Some(styles) = style_set(resolved.styles.clone()) {
        let _ = write!(out, "\n{inner}.with_styles({styles})");
    }
    if drag {
        let _ = write!(out, "\n{inner}.draggable(true)");
    }
}

fn text_binding(out: &mut String, text: Option<Expr>, name: &str, inner: &str, names: &Names<'_>) {
    let Some(expression) = text else {
        return;
    };
    let _ = writeln!(
        out,
        "{inner}bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::Text({})));",
        expression::closure(&expression, names)
    );
}

fn handler_bindings(
    out: &mut String,
    handlers: &[(HandlerEvent, String)],
    name: &str,
    inner: &str,
) {
    for (event, handler) in handlers {
        let handler = handler.as_str();
        let _ = writeln!(
            out,
            "{inner}handlers.push(brul::Handler::new(component, Some({name}.id()), brul::HandlerEvent::{event:?}, {handler:?}));"
        );
    }
}

pub(crate) fn items(
    out: &mut String,
    block: &Block,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    for item in &block.items {
        match item {
            BlockItem::Element(child) => {
                widget(out, child, emit, frame, counter)?;
                out.push_str(",\n");
            }
            BlockItem::If(structure) => {
                structure::structure_if(out, structure, emit, frame, counter)?;
                out.push_str(",\n");
            }
            BlockItem::Each(structure) => {
                structure::structure_each(out, structure, emit, frame, counter)?;
                out.push_str(",\n");
            }
            BlockItem::Expr(expression) => {
                return Err(error_at(
                    expression.position,
                    "unexpected expression in a container",
                ));
            }
        }
    }
    Ok(())
}

fn text_widget(
    out: &mut String,
    element: &Element,
    styles: &TextStyleSet,
) -> Result<Option<Expr>, Error> {
    let styles = text_style_set(styles);
    match element.block.items.as_slice() {
        [BlockItem::Expr(expression)] => {
            let ExprKind::String(content) = &expression.kind else {
                let _ = match &styles {
                    Some(styles) => {
                        write!(out, "brul_gui::Widget::text_with_styles(\"\", {styles})")
                    }
                    None => write!(out, "brul_gui::Widget::text(\"\")"),
                };
                return Ok(Some(expression.clone()));
            };
            let _ = match &styles {
                Some(styles) => write!(
                    out,
                    "brul_gui::Widget::text_with_styles({content:?}, {styles})"
                ),
                None => write!(out, "brul_gui::Widget::text({content:?})"),
            };
            Ok(None)
        }
        _ => Err(error_at(
            element.position,
            "Text requires a single content expression",
        )),
    }
}

fn image_widget(
    out: &mut String,
    element: &Element,
    emit: &Emit<'_>,
) -> Result<Option<Expr>, Error> {
    let [BlockItem::Expr(expression)] = element.block.items.as_slice() else {
        return Err(error_at(
            element.position,
            "Image requires a single resource name",
        ));
    };
    let ExprKind::Ident(name) = &expression.kind else {
        return Err(error_at(
            expression.position,
            "Image requires a single resource name",
        ));
    };
    if !emit.resources.iter().any(|resource| &resource.name == name) {
        return Err(error_at(
            expression.position,
            format!("image source '{name}' is not a resource declared in <use>"),
        ));
    }
    let _ = write!(
        out,
        "brul_gui::Widget::image(brul_gui::Texture::from_png({}resource_{name}()).expect(\"resource '{name}' cannot be decoded\"))",
        emit.resource_prefix
    );
    Ok(None)
}

fn parameters(
    out: &mut String,
    element: &Element,
    name: &str,
    pad: &str,
    names: &Names<'_>,
) -> Result<(), Error> {
    let mut x = None;
    let mut y = None;
    for arg in &element.args {
        let Arg::Named {
            name: parameter,
            value,
        } = arg
        else {
            continue;
        };
        let target = match parameter.as_str() {
            "checked" => "brul::BindingTarget::Checked",
            "selected" => "brul::BindingTarget::Selected",
            "disabled" => "brul::BindingTarget::Disabled",
            "visible" => "brul::BindingTarget::Visible",
            "x" => {
                x = Some(value);
                continue;
            }
            "y" => {
                y = Some(value);
                continue;
            }
            _ => {
                return Err(error_at(
                    value.position,
                    format!("parameter '{parameter}' is not supported yet"),
                ));
            }
        };
        let _ = writeln!(
            out,
            "{pad}bindings.push(brul::Binding::new(component, {name}.id(), {target}({})));",
            expression::closure(value, names)
        );
    }
    if x.is_some() || y.is_some() {
        let x = optional_expression(x, names);
        let y = optional_expression(y, names);
        let _ = writeln!(
            out,
            "{pad}bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::Offset {{ x: {x}, y: {y} }}));"
        );
    }
    Ok(())
}

fn optional_expression(value: Option<&Expr>, names: &Names<'_>) -> String {
    match value {
        Some(item) => format!("Some({})", expression::closure(item, names)),
        None => "None".to_owned(),
    }
}
