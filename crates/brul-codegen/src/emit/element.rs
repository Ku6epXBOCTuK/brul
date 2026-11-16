use brul_script::Names;
use brul_syntax::ast::{Expr, ExprKind};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Block, BlockItem, Element, Slot, Widget};

use crate::error_at;
use crate::styles;

use super::Emit;
use super::Frame;
use super::Out;
use super::classes;
use super::{component, events, expression, structure};
use crate::runtime::HandlerEvent;

pub(super) fn widget(
    out: &mut Out<'_>,
    element: &Element,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    if let Some(index) = emit
        .imports
        .iter()
        .find(|import| import.name == element.name)
        .map(|import| import.definition)
    {
        return component::component(out, element, index, emit, counter);
    }
    let classes = classes(element)
        .iter()
        .map(|class| classes::index(emit.classes, class))
        .collect::<Vec<_>>();
    let events = events::widget(element, emit.program)?;
    let needs = content_needed(element, emit)
        || !events.handlers.is_empty()
        || element
            .args
            .iter()
            .any(|arg| matches!(arg, Arg::Named { .. }));
    if !needs {
        out.indent();
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
    out.line(format_args!("{{"));
    out.indented(|out| -> Result<(), Error> {
        out.start(format_args!("let {name} = "));
        let content = element_expression(out, element, &classes, &events, emit, frame, counter)?;
        out.push(format_args!(";"));
        out.end();
        content_binding(out, content, &name, emit.names);
        parameters(out, element, &name, emit.names, emit)?;
        handler_bindings(out, &events.handlers, &name);
        out.line(format_args!("{name}"));
        Ok(())
    })?;
    out.start(format_args!("}}"));
    Ok(())
}

fn element_expression(
    out: &mut Out<'_>,
    element: &Element,
    classes: &[usize],
    events: &events::Events,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<Option<Content>, Error> {
    let styled = !classes.is_empty();
    if styled {
        out.push(format_args!("{}styled(", emit.resource_prefix));
    }
    let content = kind(out, element, emit, frame, counter)?;
    if styled {
        let list = classes
            .iter()
            .map(|index| index.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        out.push(format_args!(", &[{list}], theme)"));
    }
    if events.drag {
        out.push(format_args!(".draggable(true)"));
    }
    if events.focusable {
        out.push(format_args!(".focusable(true)"));
    }
    if events.multiline {
        out.push(format_args!(".multiline(true)"));
    }
    if events.modal {
        out.push(format_args!(".modal(true)"));
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
        Some(Widget::View | Widget::Row | Widget::Column | Widget::Layer) | None => false,
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
    out: &mut Out<'_>,
    element: &Element,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<Option<Content>, Error> {
    match element.widget {
        Some(Widget::Row) => {
            container(out, element, Group::Row, emit, frame, counter)?;
            Ok(None)
        }
        Some(Widget::View | Widget::Column) => {
            container(out, element, Group::Column, emit, frame, counter)?;
            Ok(None)
        }
        Some(Widget::Layer) => {
            container(out, element, Group::Layer, emit, frame, counter)?;
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

#[derive(Clone, Copy)]
enum Group {
    Row,
    Column,
    Layer,
}

impl Group {
    fn call(self) -> &'static str {
        match self {
            Self::Row => "row",
            Self::Column => "column",
            Self::Layer => "layer",
        }
    }

    fn direction(self) -> &'static str {
        match self {
            Self::Row => "Row",
            Self::Column | Self::Layer => "Column",
        }
    }
}

fn container(
    out: &mut Out<'_>,
    element: &Element,
    group: Group,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    let call = group.call();
    if element.block.items.is_empty() {
        out.push(format_args!("brul::gui::Widget::{call}(vec![])"));
        return Ok(());
    }
    let direction = group.direction();
    let gap = styles::resolve(emit.stylesheet, &classes(element), frame.theme())
        .map_err(|error| error_at(element.position, error.to_string()))?
        .layout
        .gap;
    let children = frame.deeper(2).inside(direction, gap);
    let mut out = Out::at(out.raw(), frame.depth());
    if contains_slot(&element.block) {
        out.push(format_args!("brul::gui::Widget::{call}({{"));
        out.end();
        out.indented(|out| -> Result<(), Error> {
            out.line(format_args!("let mut children = Vec::new();"));
            items_spliced(out, &element.block, emit, children.shallower(1), counter)?;
            out.line(format_args!("children"));
            Ok(())
        })?;
        out.start(format_args!("}}}})"));
        return Ok(());
    }
    out.push(format_args!("brul::gui::Widget::{call}(vec!["));
    out.end();
    out.indented(|out| -> Result<(), Error> {
        out.indented(|out| items(out, &element.block, emit, children, counter))?;
        out.start(format_args!("])"));
        Ok(())
    })?;
    Ok(())
}

fn content_binding(out: &mut Out<'_>, content: Option<Content>, name: &str, names: &Names<'_>) {
    let Some(content) = content else {
        return;
    };
    let (target, expression) = match content {
        Content::Text(expression) => ("Text", expression),
        Content::Input(expression) => ("Input", expression),
        Content::Image(expression) => ("Image", expression),
    };
    out.line(format_args!(
        "bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::{target}({})));",
        expression::closure(&expression, names)
    ));
}

fn handler_bindings(out: &mut Out<'_>, handlers: &[(HandlerEvent, String)], name: &str) {
    for (event, handler) in handlers {
        let handler = handler.as_str();
        out.line(format_args!(
            "handlers.push(brul::Handler::new(component, Some({name}.id()), brul::HandlerEvent::{event:?}, {handler:?}));"
        ));
    }
}

pub(crate) fn items(
    out: &mut Out<'_>,
    block: &Block,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    for item in &block.items {
        match item {
            BlockItem::Element(child) => {
                widget(out, child, emit, frame, counter)?;
                out.push(format_args!(",\n"));
            }
            BlockItem::If(structure) => {
                structure::structure_if(out, structure, emit, frame, counter)?;
                out.push(format_args!(",\n"));
            }
            BlockItem::Each(structure) => {
                structure::structure_each(out, structure, emit, frame, counter)?;
                out.push(format_args!(",\n"));
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
            structure
                .branches
                .iter()
                .any(|branch| contains_slot(&branch.block))
                || structure.else_block.as_ref().is_some_and(contains_slot)
        }
        BlockItem::Each(structure) => {
            contains_slot(&structure.block)
                || structure.else_block.as_ref().is_some_and(contains_slot)
        }
        BlockItem::Element(_) | BlockItem::Expr(_) => false,
    })
}

pub(crate) fn items_spliced(
    out: &mut Out<'_>,
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
                structure::structure_if(&mut Out::new(&mut arm), structure, emit, frame, counter)?;
                push_text(out, &arm);
            }
            BlockItem::Each(structure) => {
                let mut arm = String::new();
                structure::structure_each(
                    &mut Out::new(&mut arm),
                    structure,
                    emit,
                    frame,
                    counter,
                )?;
                push_text(out, &arm);
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
    out: &mut Out<'_>,
    child: &Element,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    let mut item = String::new();
    widget(
        &mut Out::at(&mut item, frame.depth()),
        child,
        emit,
        frame,
        counter,
    )?;
    push_text(out, &item);
    Ok(())
}

fn push_text(out: &mut Out<'_>, item: &str) {
    out.line(format_args!("children.push({});", item.trim_start()));
}

fn slot_item(
    out: &mut Out<'_>,
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
    if slot.block.is_some() {
        return Err(error_at(
            slot.position,
            "slot content is written at the call site",
        ));
    }
    let target = match &slot.name {
        Some(name) => format!("{name:?}"),
        None => "brul::DEFAULT_SLOT".to_owned(),
    };
    let mut out = Out::at(out.raw(), frame.depth());
    out.line(format_args!("{{"));
    out.indented(|out| {
        out.line(format_args!("let built = brul::slot(theme, {target})?;"));
        for field in ["bindings", "handlers", "props"] {
            out.line(format_args!("{field}.extend(built.{field});"));
        }
        out.line(format_args!(
            "children.extend(built.widget.into_children());"
        ));
    });
    out.start(format_args!("}}"));
    Ok(())
}

fn text_widget(out: &mut Out<'_>, element: &Element) -> Result<Option<Expr>, Error> {
    field_widget(out, element, "text", "Text")
}

fn input_widget(out: &mut Out<'_>, element: &Element) -> Result<Option<Expr>, Error> {
    field_widget(out, element, "input", "TextInput")
}

fn field_widget(
    out: &mut Out<'_>,
    element: &Element,
    constructor: &str,
    label: &str,
) -> Result<Option<Expr>, Error> {
    match element.block.items.as_slice() {
        [BlockItem::Expr(expression)] => {
            let ExprKind::String(content) = &expression.kind else {
                out.push(format_args!("brul::gui::Widget::{constructor}(\"\")"));
                return Ok(Some(expression.clone()));
            };
            out.push(format_args!(
                "brul::gui::Widget::{constructor}({content:?})"
            ));
            Ok(None)
        }
        _ => Err(error_at(
            element.position,
            format!("{label} requires a single content expression"),
        )),
    }
}

fn image_widget(
    out: &mut Out<'_>,
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
        out.push(format_args!(
            "brul::gui::Widget::image(brul::gui::Image::from_bytes({}resource_{name}()).expect(\"resource '{name}' cannot be decoded\"))",
            emit.resource_prefix
        ));
        return Ok(None);
    }
    out.push(format_args!(
        "brul::gui::Widget::image(brul::gui::Texture::empty())"
    ));
    Ok(Some(expression.clone()))
}

fn parameters(
    out: &mut Out<'_>,
    element: &Element,
    name: &str,
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
            "modal" => continue,
            _ => {
                return Err(error_at(
                    value.position,
                    format!("parameter '{parameter}' is not supported yet"),
                ));
            }
        };
        out.line(format_args!(
            "bindings.push(brul::Binding::new(component, {name}.id(), {target}({})));",
            expression::closure(value, names)
        ));
    }
    if x.is_some() || y.is_some() {
        let x = optional_expression(x, names);
        let y = optional_expression(y, names);
        out.line(format_args!(
            "bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::Offset {{ x: {x}, y: {y} }}));"
        ));
    }
    if let Some(value) = class {
        *emit.dynamic_classes = true;
        let table = classes::table(emit.resource_prefix);
        out.line(format_args!(
            "bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::Class {{ table: {table}, expression: {} }}));",
            expression::closure(value, names)
        ));
    }
    Ok(())
}

fn optional_expression(value: Option<&Expr>, names: &Names<'_>) -> String {
    match value {
        Some(item) => format!("Some({})", expression::closure(item, names)),
        None => "None".to_owned(),
    }
}
