use std::collections::HashMap;

use brul_case::constant;
use brul_gui::Direction;
use brul_script::Names;
use brul_syntax::ast::{Expr, ExprKind};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Block, BlockItem, Element, Slot, Widget};
use brul_syntax::style::EffectRule;
use brul_syntax::types::Type;

use crate::attachments::Attachments;
use crate::error_at;
use crate::styles;
use crate::widgets::WidgetSpec;

use super::Emit;
use super::Frame;
use super::Out;
use super::anchor;
use super::classes;
use super::state::State;
use super::{ScopeInfo, component, events, expression, structure};
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
    if let Some(spec) = emit.widgets.get(&element.name).cloned() {
        return registered_widget(out, element, &spec, emit, frame, counter);
    }
    let element_classes = classes(element);
    let classes = element_classes
        .iter()
        .map(|class| classes::index(emit.classes, class))
        .collect::<Vec<_>>();
    let effect = emit
        .effects
        .iter()
        .find(|rule| element_classes.contains(&rule.class.as_str()));
    let style = ElementStyle {
        classes: &classes,
        effect,
    };
    let events = events::widget(element, emit.program)?;
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
        handler_bindings(out, &events.handlers, &name, emit.script_name);
        attachment_bindings(
            out,
            element,
            &name,
            AttachmentContext {
                attachments: emit.attachments,
                names: emit.names,
                state: emit.state,
                prop_types: emit.prop_types,
                scopes: &emit.scope_defs,
                scope: frame.scope,
            },
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
    if let Some(rule) = style.effect {
        out.push(format_args!(
            ".effect(brul::gui::EffectSpec {{ name: {:?}, params: [[0.0; 4]; 16], animated: false }})",
            rule.name
        ));
    }
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
        Some(Widget::Image) => super::resource(expression, emit.resources).is_none(),
        Some(Widget::Container | Widget::Layer | Widget::Video | Widget::WebView) | None => false,
    }
}

fn classes(element: &Element) -> Vec<&str> {
    element
        .args
        .iter()
        .filter_map(|arg| match arg {
            Arg::Class(class) => Some(class.as_str()),
            _ => None,
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
    let resolved = styles::resolve(emit.stylesheet, &classes(element), frame.theme())
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

fn content_binding(
    out: &mut Out<'_>,
    content: Option<Content>,
    name: &str,
    emit: &Emit<'_>,
    frame: Frame<'_>,
) -> Result<(), Error> {
    let Some(content) = content else {
        return Ok(());
    };
    let (target, expression, ty) = match content {
        Content::Text(expression) => ("Text", expression, brul_syntax::types::Type::String),
        Content::Input(expression) => ("Input", expression, brul_syntax::types::Type::String),
        Content::Image(expression) => ("Image", expression, brul_syntax::types::Type::Image),
    };
    let closure = if ty == brul_syntax::types::Type::Image {
        expression::closure(
            &expression,
            emit.names,
            emit.state,
            emit.prop_types,
            &emit.scope_defs,
            frame.scope,
        )?
    } else {
        expression::typed_closure(
            &expression,
            emit.names,
            emit.state,
            emit.prop_types,
            &emit.scope_defs,
            frame.scope,
            &ty,
        )?
    };
    out.line(format_args!(
        "bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::{target}({closure})));"
    ));
    Ok(())
}

fn effect_binding(
    out: &mut Out<'_>,
    rule: &EffectRule,
    name: &str,
    emit: &Emit<'_>,
    frame: Frame<'_>,
) -> Result<(), Error> {
    let mut args = Vec::with_capacity(rule.args.len());
    for expression in &rule.args {
        args.push(expression::typed_closure(
            expression,
            emit.names,
            emit.state,
            emit.prop_types,
            &emit.scope_defs,
            frame.scope,
            &brul_syntax::types::Type::Number,
        )?);
    }
    out.line(format_args!(
        "bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::Effect {{ def: crate::BRUL_EFFECT_{}, args: vec![{}] }}));",
        constant(&rule.name),
        args.join(", ")
    ));
    Ok(())
}

fn handler_bindings(
    out: &mut Out<'_>,
    handlers: &[(HandlerEvent, String)],
    name: &str,
    script: &str,
) {
    for (event, handler) in handlers {
        let call = super::script::event_call(script, handler);
        out.line(format_args!(
            "handlers.push(brul::Handler::new(component, Some({name}.id()), brul::HandlerEvent::{event:?}, {call}));"
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

fn slot_item(out: &mut Out<'_>, slot: &Slot, emit: &Emit<'_>) -> Result<(), Error> {
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
    out.line(format_args!("{{"));
    out.indented(|out| {
        out.line(format_args!(
            "let built = brul::slot(theme, build, {target})?;"
        ));
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
    if let Some(name) = super::resource(expression, emit.resources) {
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

const NUMBER_TYPE: Type = Type::Number;

fn state_id(element: &Element) -> u64 {
    let mut id = 0x811c9dc5u64;
    for byte in element.name.bytes() {
        id = (id ^ u64::from(byte)).wrapping_mul(0x01000193);
    }
    let line = u64::try_from(element.position.line).unwrap_or(u64::MAX);
    let column = u64::try_from(element.position.column).unwrap_or(u64::MAX);
    let position = (line << 32) ^ column;
    (id ^ position) & 0x001f_ffff_ffff_ffff
}

fn registered_widget(
    out: &mut Out<'_>,
    element: &Element,
    spec: &WidgetSpec,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    if !element.block.items.is_empty() {
        return Err(error_at(
            element.position,
            format!("widget '{}' does not accept children", spec.name),
        ));
    }
    let mut owned: Vec<(&str, Type, &Expr)> = Vec::new();
    let mut handlers: Vec<(&str, &str, Vec<Type>)> = Vec::new();
    let mut class = None;
    for arg in &element.args {
        match arg {
            Arg::Class(_) | Arg::Anchor(_) | Arg::Name(_) | Arg::Attach { .. } => {}
            Arg::Handler { event, handler } => {
                if handlers.iter().any(|(name, _, _)| *name == event.as_str()) {
                    return Err(error_at(
                        element.position,
                        format!("event '{event}' is handled twice"),
                    ));
                }
                let Some(definition) = spec.events.iter().find(|item| item.name == *event) else {
                    return Err(error_at(
                        element.position,
                        format!("widget '{}' has no event '{event}'", spec.name),
                    ));
                };
                let params: Vec<Type> = definition.params.clone();
                events::define(handler, emit.program, &params, element.position)?;
                handlers.push((event, handler, params));
            }
            Arg::Named { name, value } if name == "class" => class = Some(value),
            Arg::Named { name, value } => {
                if owned
                    .iter()
                    .any(|(existing, _, _)| *existing == name.as_str())
                {
                    return Err(error_at(
                        value.position,
                        format!("property '{name}' is set twice"),
                    ));
                }
                let Some(prop) = spec.props.iter().find(|item| item.name == *name) else {
                    return Err(error_at(
                        value.position,
                        format!("widget '{}' has no property '{name}'", spec.name),
                    ));
                };
                owned.push((name.as_str(), prop.ty.clone(), value));
            }
        }
    }
    let mut props: Vec<(&str, &Type, &Expr)> = owned
        .iter()
        .map(|(name, ty, value)| (*name, ty, *value))
        .collect();
    let state_expr;
    if matches!(spec.name.as_str(), "Video" | "WebView") {
        state_expr = Expr::new(ExprKind::Number(state_id(element) as f64), element.position);
        props.push(("state", &NUMBER_TYPE, &state_expr));
    }
    let element_classes = classes(element);
    let class_indices = element_classes
        .iter()
        .map(|class| classes::index(&mut *emit.classes, class))
        .collect::<Vec<_>>();
    let name = format!("widget{}", *counter);
    *counter += 1;
    let suffix = anchor::suffix(element, emit)?;
    out.line(format_args!("{{"));
    out.indented(|out| -> Result<(), Error> {
        out.line(format_args!("let built = brul::instance(build, component, brul::Instance {{"));
        out.indented(|out| -> Result<(), Error> {
            out.line(format_args!("theme,"));
            out.line(format_args!("view: {},", spec.view));
            out.line(format_args!("script: None,"));
            out.line(format_args!("name: {:?},", spec.name));
            out.line(format_args!("names: names(),"));
            component::arguments(
                out,
                "props",
                &props,
                emit.names,
                emit.state,
                emit.prop_types,
                emit.resources,
                emit.resource_prefix,
                &emit.scope_defs,
                frame.scope,
            )?;
            out.line(format_args!("defaults: &[],"));
            out.line(format_args!("events: &["));
            out.indented(|out| {
                for (event, handler, params) in &handlers {
                    let params = params
                        .iter()
                        .map(|ty| format!("brul::{}", ty.rust_path()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let call = super::script::event_call(emit.script_name, handler);
                    out.line(format_args!(
                        "brul::ComponentEvent {{ name: {event:?}, call: {call}, params: &[{params}], component: std::rc::Rc::clone(component) }},"
                    ));
                }
            });
            out.line(format_args!("],"));
            out.line(format_args!("slots: &[],"));
            Ok(())
        })?;
        out.line(format_args!("}})?;"));
        for field in ["bindings", "handlers", "props"] {
            out.line(format_args!("{field}.extend(built.{field});"));
        }
        if class_indices.is_empty() {
            out.line(format_args!("let {name} = built.widget{suffix};"));
        } else {
            let list = class_indices
                .iter()
                .map(|index| index.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            out.line(format_args!(
                "let {name} = {}styled(built.widget{suffix}, &[{list}], theme);",
                emit.resource_prefix
            ));
        }
        if let Some(value) = class {
            *emit.dynamic_classes = true;
            out.line(format_args!(
                "bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::Class {{ table: {}, expression: {} }}));",
                classes::table(emit.resource_prefix),
                expression::closure(
                    value,
                    emit.names,
                    emit.state,
                    emit.prop_types,
                    &emit.scope_defs,
                    frame.scope
                )?
            ));
        }
        attachment_bindings(
            out,
            element,
            &name,
            AttachmentContext {
                attachments: emit.attachments,
                names: emit.names,
                state: emit.state,
                prop_types: emit.prop_types,
                scopes: &emit.scope_defs,
                scope: frame.scope,
            },
        )?;
        out.line(format_args!("{name}"));
        Ok(())
    })?;
    out.start(format_args!("}}"));
    Ok(())
}

pub(super) struct AttachmentContext<'a> {
    pub(super) attachments: &'a Attachments,
    pub(super) names: &'a Names<'a>,
    pub(super) state: &'a State,
    pub(super) prop_types: &'a HashMap<String, Type>,
    pub(super) scopes: &'a [ScopeInfo],
    pub(super) scope: Option<usize>,
}

pub(super) fn attachment_bindings(
    out: &mut Out<'_>,
    element: &Element,
    widget: &str,
    context: AttachmentContext<'_>,
) -> Result<(), Error> {
    let AttachmentContext {
        attachments,
        names,
        state,
        prop_types,
        scopes,
        scope,
    } = context;
    let id = state_id(element);
    for arg in &element.args {
        let Arg::Attach {
            name,
            params,
            position,
        } = arg
        else {
            continue;
        };
        let Some(spec) = attachments.get(name).cloned() else {
            return Err(error_at(*position, format!("unknown attachment '{name}'")));
        };
        let mut expressions = Vec::with_capacity(spec.params.len());
        for expected in &spec.params {
            let Some((_, value)) = params
                .iter()
                .find(|(given, _)| *given == expected.name.as_str())
            else {
                return Err(error_at(
                    *position,
                    format!("attachment '{name}' needs a '{}' parameter", expected.name),
                ));
            };
            expressions.push(expression::closure(
                value, names, state, prop_types, scopes, scope,
            )?);
        }
        if params.len() != spec.params.len() {
            return Err(error_at(
                *position,
                format!("attachment '{name}' takes {} parameters", spec.params.len()),
            ));
        }
        let events = &spec.events;
        out.start(format_args!("build.attach(brul::Attachment {{\n"));
        out.indented(|out| {
            out.line(format_args!("widget: {widget}.id(),"));
            out.line(format_args!("state: brul::StateId::new({id}u64),"));
            out.line(format_args!("component: std::rc::Rc::clone(component),"));
            out.line(format_args!(
                "params: vec![{}],",
                expressions.join(", ")
            ));
            out.line(format_args!("apply: {},", spec.apply));
            out.line(format_args!("event: {},", spec.event));
            out.line(format_args!("cleanup: {},", spec.cleanup));
            out.line(format_args!(
                "events: brul::AttachEvents {{ key: {}, ime: {}, input: {}, submit: {}, focus: {}, blur: {}, click: {} }},",
                events.key,
                events.ime,
                events.input,
                events.submit,
                events.focus,
                events.blur,
                events.click
            ));
            Ok::<(), Error>(())
        })?;
        out.line(format_args!("}});"));
    }
    Ok(())
}

fn parameters(
    out: &mut Out<'_>,
    element: &Element,
    name: &str,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
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
        let closure = expression::typed_closure(
            value,
            emit.names,
            emit.state,
            emit.prop_types,
            &emit.scope_defs,
            frame.scope,
            &brul_syntax::types::Type::Bool,
        )?;
        out.line(format_args!(
            "bindings.push(brul::Binding::new(component, {name}.id(), {target}({closure})));"
        ));
    }
    if x.is_some() || y.is_some() {
        let x = optional_expression(x, emit, frame)?;
        let y = optional_expression(y, emit, frame)?;
        out.line(format_args!(
            "bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::Offset {{ x: {x}, y: {y} }}));"
        ));
    }
    if let Some(value) = class {
        *emit.dynamic_classes = true;
        let table = classes::table(emit.resource_prefix);
        out.line(format_args!(
            "bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::Class {{ table: {table}, expression: {} }}));",
            expression::closure(value, emit.names, emit.state, emit.prop_types, &emit.scope_defs, frame.scope)?
        ));
    }
    Ok(())
}

fn optional_expression(
    value: Option<&Expr>,
    emit: &Emit<'_>,
    frame: Frame<'_>,
) -> Result<String, Error> {
    Ok(match value {
        Some(item) => format!(
            "Some({})",
            expression::typed_closure(
                item,
                emit.names,
                emit.state,
                emit.prop_types,
                &emit.scope_defs,
                frame.scope,
                &brul_syntax::types::Type::Number,
            )?
        ),
        None => "None".to_owned(),
    })
}
