use brul_gui::Direction;
use brul_syntax::ast::{Expr, ExprKind};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Block, BlockItem, Element, Widget};
use brul_syntax::style::EffectRule;

use crate::error_at;
use crate::styles;

use super::Emit;
use super::Frame;
use super::Out;
use super::anchor;
use super::classes;
pub(super) use super::element_bindings::{HooksSpec, attachment_bindings, hooks_binding};
use super::element_bindings::{content_binding, effect_binding, handler_bindings, parameters};
use super::element_widgets::{
    image_widget, input_widget, registered_widget, slot_item, text_widget,
};
use super::{component, events, structure};

pub(super) fn debug_named(element: &Element) -> String {
    format!(".debug_named({:?})", element.name)
}

pub(super) fn widget(
    out: &mut Out<'_>,
    element: &Element,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    if let Some(index) = emit
        .context
        .imports
        .iter()
        .find(|import| import.name == element.name)
        .map(|import| import.definition)
    {
        return component::component(out, element, index, emit, counter, frame);
    }
    if element.args.iter().any(|arg| matches!(arg, Arg::Anchor(_)))
        && element.args.iter().any(|arg| {
            matches!(
                arg,
                Arg::Named { name, .. } if name == "x" || name == "y"
            )
        })
    {
        return Err(error_at(
            element.position,
            "anchor cannot be combined with x or y",
        ));
    }
    if let Some(spec) = emit.context.capabilities.specs.get(&element.name).cloned()
        && spec.view.is_some()
    {
        return registered_widget(out, element, &spec, emit, frame, counter);
    }
    let element_classes = classes(element);
    let classes = element_classes
        .iter()
        .map(|class| classes::index(emit.classes, class))
        .collect::<Vec<_>>();
    let effect = emit
        .context
        .effects
        .iter()
        .find(|rule| element_classes.contains(&rule.class.as_str()));
    let style = ElementStyle {
        classes: &classes,
        effect,
    };
    let events = events::widget(element, emit.context.program)?;
    let needs = effect.is_some()
        || content_needed(element, emit)
        || !events.handlers.is_empty()
        || element
            .args
            .iter()
            .any(|arg| matches!(arg, Arg::Named { .. } | Arg::Attach { .. }));
    if !needs {
        out.indent();
        element_expression(out, element, &style, &events, emit, frame, counter)?;
        return Ok(());
    }
    let name = format!("widget{}", *counter);
    *counter += 1;
    out.line(format_args!("{{"));
    out.indented(|out| -> Result<(), Error> {
        out.start(format_args!("let {name} = "));
        let content = element_expression(out, element, &style, &events, emit, frame, counter)?;
        out.push(format_args!(";"));
        out.end();
        content_binding(out, content, &name, emit, frame)?;
        if let Some(rule) = effect {
            effect_binding(out, rule, &name, emit, frame)?;
        }
        parameters(out, element, &name, emit, frame)?;
        handler_bindings(out, &events.handlers, &name, emit.context.script_name);
        attachment_bindings(
            out,
            element,
            &name,
            emit.context,
            &emit.scope_defs,
            frame.scope,
        )?;
        out.line(format_args!("{name}"));
        Ok(())
    })?;
    out.start(format_args!("}}"));
    Ok(())
}

struct ElementStyle<'a> {
    classes: &'a [usize],
    effect: Option<&'a EffectRule>,
}

fn element_expression(
    out: &mut Out<'_>,
    element: &Element,
    style: &ElementStyle<'_>,
    events: &events::Events,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<Option<Content>, Error> {
    let classes = style.classes;
    let styled = !classes.is_empty();
    if styled {
        out.push(format_args!("{}styled(", emit.context.resource_prefix));
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
        out.push(format_args!(".wrap(true)"));
    }
    if events.modal {
        out.push(format_args!(".modal(true)"));
    }
    if let Some(rule) = style.effect {
        out.push(format_args!(
            ".effect(brul::gui::EffectSpec {{ name: {:?}, params: [[0.0; 4]; 16], animated: false }})",
            rule.name
        ));
    }
    out.push(format_args!("{}", debug_named(element)));
    let suffix = anchor::suffix(element, emit)?;
    if !suffix.is_empty() {
        out.push(format_args!("{suffix}"));
    }
    Ok(content)
}

fn content_needed(element: &Element, emit: &Emit<'_>) -> bool {
    let [BlockItem::Expr(expression)] = element.block.items.as_slice() else {
        return false;
    };
    match element.widget {
        Some(Widget::Text | Widget::TextInput) => !matches!(expression.kind, ExprKind::String(_)),
        Some(Widget::Image) => super::resource(expression, emit.context.resources).is_none(),
        Some(
            Widget::Container | Widget::Layer | Widget::Video | Widget::WebView | Widget::Canvas,
        )
        | None => false,
    }
}

pub(super) fn classes(element: &Element) -> Vec<&str> {
    element
        .args
        .iter()
        .filter_map(|arg| match arg {
            Arg::Class(class) => Some(class.as_str()),
            _ => None,
        })
        .collect()
}

pub(super) enum Content {
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
        Some(Widget::Container) => {
            container(out, element, Group::Container, emit, frame, counter)?;
            Ok(None)
        }
        Some(Widget::Layer) => {
            container(out, element, Group::Layer, emit, frame, counter)?;
            Ok(None)
        }
        Some(Widget::Text) => Ok(text_widget(out, element)?.map(Content::Text)),
        Some(Widget::TextInput) => Ok(input_widget(out, element)?.map(Content::Input)),
        Some(Widget::Image) => Ok(image_widget(out, element, emit)?.map(Content::Image)),
        Some(Widget::Video) => Err(error_at(
            element.position,
            "video element is not generated by this view".to_owned(),
        )),
        Some(Widget::WebView) => Err(error_at(
            element.position,
            "webview element is not generated by this view".to_owned(),
        )),
        Some(Widget::Canvas) => Err(error_at(
            element.position,
            "canvas element is not generated by this view".to_owned(),
        )),
        None => Err(error_at(
            element.position,
            format!("widget '{}' is not generated yet", element.name),
        )),
    }
}

#[derive(Clone, Copy)]
enum Group {
    Container,
    Layer,
}

impl Group {
    fn call(self) -> &'static str {
        match self {
            Self::Container => "view",
            Self::Layer => "layer",
        }
    }

    fn args(self, direction: Direction) -> String {
        match self {
            Self::Container => format!("brul::gui::Direction::{direction:?}, "),
            Self::Layer => String::new(),
        }
    }

    fn direction(self, direction: Direction) -> &'static str {
        match self {
            Self::Container => match direction {
                Direction::Row => "Row",
                Direction::Column => "Column",
            },
            Self::Layer => "Column",
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
    let resolved = styles::resolve(emit.context.stylesheet, &classes(element), frame.theme())
        .map_err(|error| error_at(element.position, error.to_string()))?;
    let direction = match group {
        Group::Layer => Direction::Column,
        Group::Container => resolved.layout.flex_direction,
    };
    let args = group.args(direction);
    if element.block.items.is_empty() {
        out.push(format_args!("brul::gui::Widget::{call}({args}vec![])"));
        return Ok(());
    }
    let frame_direction = group.direction(direction);
    let row_gap = resolved.layout.row_gap();
    let column_gap = resolved.layout.column_gap();
    let children = frame.inside(frame_direction, row_gap, column_gap);
    if contains_slot(&element.block) {
        out.push(format_args!("brul::gui::Widget::{call}({args}{{"));
        out.end();
        out.indented(|out| -> Result<(), Error> {
            out.line(format_args!("let mut children = Vec::new();"));
            items_spliced(out, &element.block, emit, children, counter)?;
            out.line(format_args!("children"));
            Ok(())
        })?;
        out.start(format_args!("}})"));
        return Ok(());
    }
    out.push(format_args!("brul::gui::Widget::{call}({args}vec!["));
    out.end();
    out.indented(|out| items(out, &element.block, emit, children, counter))?;
    out.start(format_args!("])"));
    Ok(())
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
                structure::structure_if(
                    &mut Out::at(&mut arm, out.depth()),
                    structure,
                    emit,
                    frame,
                    counter,
                )?;
                push_text(out, &arm);
            }
            BlockItem::Each(structure) => {
                let mut arm = String::new();
                structure::structure_each(
                    &mut Out::at(&mut arm, out.depth()),
                    structure,
                    emit,
                    frame,
                    counter,
                )?;
                push_text(out, &arm);
            }
            BlockItem::Slot(slot) => slot_item(out, slot, emit)?,
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
        &mut Out::at(&mut item, out.depth()),
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
