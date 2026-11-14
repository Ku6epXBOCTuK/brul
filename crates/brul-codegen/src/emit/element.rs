use std::fmt::Write;

use brul_script::Names;
use brul_syntax::ast::{Expr, ExprKind};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Block, BlockItem, Element, Slot, Widget};

use crate::error_at;
use crate::styles;

use super::Emit;
use super::Frame;
use super::classes;
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
    let classes = classes(element);
    let events = events::widget(element, emit.program)?;
    let pad = "    ".repeat(frame.depth());
    let inner = "    ".repeat(frame.depth() + 1);
    let needs = content_needed(element, emit)
        || !events.handlers.is_empty()
        || element
            .args
            .iter()
            .any(|arg| matches!(arg, Arg::Named { .. }));
    if !needs {
        let _ = write!(out, "{pad}");
        element_expression(
            out,
            element,
            &classes,
            &events,
            emit,
            frame.shallower(1),
            counter,
        )?;
        return Ok(());
    }
    let name = format!("widget{}", *counter);
    *counter += 1;
    let _ = writeln!(out, "{pad}{{");
    let _ = write!(out, "{inner}let {name} = ");
    let content = element_expression(out, element, &classes, &events, emit, frame, counter)?;
    out.push_str(";\n");
    content_binding(out, content, &name, &inner, emit.names);
    parameters(out, element, &name, &inner, emit.names, emit)?;
    handler_bindings(out, &events.handlers, &name, &inner);
    let _ = writeln!(out, "{inner}{name}");
    let _ = write!(out, "{pad}}}");
    Ok(())
}

fn element_expression(
    out: &mut String,
    element: &Element,
    classes: &[&str],
    events: &events::Events,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<Option<Content>, Error> {
    let styled = !classes.is_empty();
    if styled {
        let _ = write!(out, "{}styled(", emit.resource_prefix);
    }
    let content = kind(out, element, emit, frame, counter)?;
    if styled {
        let list = classes
            .iter()
            .map(|class| format!("{class:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, ", &[{list}], theme)?");
    }
    if events.drag {
        out.push_str(".draggable(true)");
    }
    if events.focusable {
        out.push_str(".focusable(true)");
    }
    if events.multiline {
        out.push_str(".multiline(true)");
    }
    Ok(content)
}

fn content_needed(element: &Element, emit: &Emit<'_>) -> bool {
    let [BlockItem::Expr(expression)] = element.block.items.as_slice() else {
        return false;
    };
    match element.widget {
        Some(Widget::Text | Widget::TextInput) => !matches!(expression.kind, ExprKind::String(_)),
        Some(Widget::Image) => resource(expression, emit).is_none(),
        Some(Widget::View | Widget::Row | Widget::Column) | None => false,
    }
}

fn resource<'a>(expression: &'a Expr, emit: &Emit<'_>) -> Option<&'a str> {
    let ExprKind::Ident(name) = &expression.kind else {
        return None;
    };
    emit.resources
        .iter()
        .any(|resource| &resource.name == name)
        .then_some(name.as_str())
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

enum Content {
    Text(Expr),
    Input(Expr),
    Image(Expr),
}

fn kind(
    out: &mut String,
    element: &Element,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<Option<Content>, Error> {
    match element.widget {
        Some(Widget::Row) => {
            container(out, element, true, emit, frame, counter)?;
            Ok(None)
        }
        Some(Widget::View | Widget::Column) => {
            container(out, element, false, emit, frame, counter)?;
            Ok(None)
        }
        Some(Widget::Text) => Ok(text_widget(out, element)?.map(Content::Text)),
        Some(Widget::TextInput) => Ok(input_widget(out, element)?.map(Content::Input)),
        Some(Widget::Image) => Ok(image_widget(out, element, emit)?.map(Content::Image)),
        None => Err(error_at(
            element.position,
            format!("widget '{}' is not generated yet", element.name),
        )),
    }
}

fn container(
    out: &mut String,
    element: &Element,
    row: bool,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    let call = if row { "row" } else { "column" };
    if element.block.items.is_empty() {
        let _ = write!(out, "brul::gui::Widget::{call}(vec![])");
        return Ok(());
    }
    let direction = if row { "Row" } else { "Column" };
    let gap = styles::resolve(emit.stylesheet, &classes(element), frame.theme())
        .map_err(|error| error_at(element.position, error.to_string()))?
        .layout
        .gap;
    let children = frame.deeper(2).inside(direction, gap);
    if contains_slot(&element.block) {
        let inner = "    ".repeat(frame.depth() + 1);
        let _ = writeln!(out, "brul::gui::Widget::{call}({{");
        let _ = writeln!(out, "{inner}let mut children = Vec::new();");
        items_spliced(out, &element.block, emit, children.shallower(1), counter)?;
        let _ = writeln!(out, "{inner}children");
        let _ = write!(out, "{}}})", "    ".repeat(frame.depth()));
        return Ok(());
    }
    let _ = writeln!(out, "brul::gui::Widget::{call}(vec![");
    items(out, &element.block, emit, children, counter)?;
    let inner = "    ".repeat(frame.depth() + 1);
    let _ = write!(out, "{inner}])");
    Ok(())
}

fn content_binding(
    out: &mut String,
    content: Option<Content>,
    name: &str,
    inner: &str,
    names: &Names<'_>,
) {
    let Some(content) = content else {
        return;
    };
    let (target, expression) = match content {
        Content::Text(expression) => ("Text", expression),
        Content::Input(expression) => ("Input", expression),
        Content::Image(expression) => ("Image", expression),
    };
    let _ = writeln!(
        out,
        "{inner}bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::{target}({})));",
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
            BlockItem::Slot(slot) => {
                return Err(error_at(
                    slot.position,
                    "a slot cannot be spliced into this container",
                ));
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

pub(crate) fn contains_slot(block: &Block) -> bool {
    block.items.iter().any(|item| match item {
        BlockItem::Slot(_) => true,
        BlockItem::If(structure) => {
            contains_slot(&structure.then_block)
                || structure.else_block.as_ref().is_some_and(contains_slot)
        }
        BlockItem::Each(structure) => contains_slot(&structure.block),
        BlockItem::Element(_) | BlockItem::Expr(_) => false,
    })
}

pub(crate) fn items_spliced(
    out: &mut String,
    block: &Block,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    for item in &block.items {
        match item {
            BlockItem::Element(child) => {
                push(out, child, emit, frame, counter)?;
            }
            BlockItem::If(structure) => {
                let mut arm = String::new();
                structure::structure_if(&mut arm, structure, emit, frame, counter)?;
                push_text(out, &arm, frame);
            }
            BlockItem::Each(structure) => {
                let mut arm = String::new();
                structure::structure_each(&mut arm, structure, emit, frame, counter)?;
                push_text(out, &arm, frame);
            }
            BlockItem::Slot(slot) => slot_item(out, slot, emit, frame)?,
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

fn push(
    out: &mut String,
    child: &Element,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    let mut item = String::new();
    widget(&mut item, child, emit, frame, counter)?;
    push_text(out, &item, frame);
    Ok(())
}

fn push_text(out: &mut String, item: &str, frame: Frame<'_>) {
    let pad = "    ".repeat(frame.depth());
    let _ = writeln!(out, "{pad}children.push({});", item.trim_start());
}

fn slot_item(
    out: &mut String,
    slot: &Slot,
    emit: &Emit<'_>,
    frame: Frame<'_>,
) -> Result<(), Error> {
    if !emit.slots {
        return Err(error_at(
            slot.position,
            "slot is only allowed inside a component",
        ));
    }
    if slot.name.is_some() {
        return Err(error_at(slot.position, "named slots are not supported yet"));
    }
    let pad = "    ".repeat(frame.depth());
    let _ = writeln!(out, "{pad}{{");
    let _ = writeln!(
        out,
        "{pad}    let built = brul::slot(theme, brul::DEFAULT_SLOT)?;"
    );
    for field in ["bindings", "handlers", "props"] {
        let _ = writeln!(out, "{pad}    {field}.extend(built.{field});");
    }
    let _ = writeln!(
        out,
        "{pad}    children.extend(built.widget.into_children());"
    );
    let _ = write!(out, "{pad}}}");
    Ok(())
}

fn text_widget(out: &mut String, element: &Element) -> Result<Option<Expr>, Error> {
    field_widget(out, element, "text", "Text")
}

fn input_widget(out: &mut String, element: &Element) -> Result<Option<Expr>, Error> {
    field_widget(out, element, "input", "TextInput")
}

fn field_widget(
    out: &mut String,
    element: &Element,
    constructor: &str,
    label: &str,
) -> Result<Option<Expr>, Error> {
    match element.block.items.as_slice() {
        [BlockItem::Expr(expression)] => {
            let ExprKind::String(content) = &expression.kind else {
                let _ = write!(out, "brul::gui::Widget::{constructor}(\"\")");
                return Ok(Some(expression.clone()));
            };
            let _ = write!(out, "brul::gui::Widget::{constructor}({content:?})");
            Ok(None)
        }
        _ => Err(error_at(
            element.position,
            format!("{label} requires a single content expression"),
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
            "Image requires a single content expression",
        ));
    };
    if let Some(name) = resource(expression, emit) {
        let _ = write!(
            out,
            "brul::gui::Widget::image(brul::gui::Image::from_bytes({}resource_{name}()).expect(\"resource '{name}' cannot be decoded\"))",
            emit.resource_prefix
        );
        return Ok(None);
    }
    out.push_str("brul::gui::Widget::image(brul::gui::Texture::empty())");
    Ok(Some(expression.clone()))
}

fn parameters(
    out: &mut String,
    element: &Element,
    name: &str,
    pad: &str,
    names: &Names<'_>,
    emit: &mut Emit<'_>,
) -> Result<(), Error> {
    let mut x = None;
    let mut y = None;
    let mut class = None;
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
            "class" => {
                class = Some(value);
                continue;
            }
            "x" => {
                x = Some(value);
                continue;
            }
            "y" => {
                y = Some(value);
                continue;
            }
            "focusable" => continue,
            "multiline" => continue,
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
    if let Some(value) = class {
        *emit.dynamic_classes = true;
        let table = classes::table(emit.resource_prefix);
        let _ = writeln!(
            out,
            "{pad}bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::Class {{ table: {table}, expression: {} }}));",
            expression::closure(value, names)
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
