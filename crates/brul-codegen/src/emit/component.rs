use brul_script::Names;
use std::collections::HashMap;

use brul_syntax::ast::Expr;
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Block, BlockItem, Element, Event, Member, Prop, PropType};
use brul_syntax::types::Type;

use crate::Resource;
use crate::error_at;
use crate::loader::Definition;

use super::Emit;
use super::Frame;
use super::Out;
use super::anchor;
use super::{element, events, expression, items_spliced};

type Props<'a> = Vec<(&'a str, &'a PropType, &'a Expr)>;
type Events<'a> = Vec<(&'a str, &'a str, Vec<PropType>)>;

struct Call<'a> {
    element: &'a Element,
    name: &'a str,
    props: Props<'a>,
    defaults: Props<'a>,
    events: Events<'a>,
    slots: Vec<SlotFill<'a>>,
    classes: Vec<&'a str>,
    class: Option<&'a Expr>,
    suffix: String,
}

struct SlotFill<'a> {
    name: Option<&'a str>,
    view: String,
}

#[derive(Default)]
struct Declared<'a> {
    default: bool,
    named: Vec<&'a str>,
}

impl Declared<'_> {
    fn any(&self) -> bool {
        self.default || !self.named.is_empty()
    }
}

pub(super) fn component(
    out: &mut Out<'_>,
    element: &Element,
    index: usize,
    emit: &mut Emit<'_>,
    counter: &mut usize,
    frame: super::Frame<'_>,
) -> Result<(), Error> {
    let definition = &emit.context.registry.definitions[index];
    if definition.is_window() {
        return Err(error_at(
            element.position,
            format!("window '{}' cannot be used as a component", definition.name),
        ));
    }
    let name = &definition.name;
    let classes = element
        .args
        .iter()
        .filter_map(|arg| match arg {
            Arg::Class(class) => Some(class.as_str()),
            _ => None,
        })
        .collect();
    let class = element.args.iter().find_map(|arg| match arg {
        Arg::Named { name, value } if name == "class" => Some(value),
        _ => None,
    });
    let mut suffix = anchor::suffix(element, emit)?;
    suffix.push_str(&element::debug_named(element));
    let (props, events) = collect(element, definition, name, emit)?;
    let defaults = defaults(element, definition, name, &props)?;
    let mut slots = Vec::new();
    for (name, block) in fills(element, definition)? {
        slots.push(SlotFill {
            name,
            view: filler(&block, emit)?,
        });
    }
    write(
        out,
        Call {
            element,
            name,
            props,
            defaults,
            events,
            slots,
            classes,
            class,
            suffix,
        },
        counter,
        emit,
        index,
        frame,
    )?;
    Ok(())
}

fn fills<'a>(
    element: &'a Element,
    definition: &'a Definition,
) -> Result<Vec<(Option<&'a str>, Block)>, Error> {
    let declared = declared_slots(definition)?;
    let mut named = Vec::new();
    let mut default = Vec::new();
    for item in &element.block.items {
        match item {
            BlockItem::Slot(slot) => match &slot.block {
                Some(block) => {
                    let Some(name) = slot.name.as_deref() else {
                        return Err(error_at(
                            slot.position,
                            "fill the default slot with plain children",
                        ));
                    };
                    if !declared.named.contains(&name) {
                        return Err(error_at(
                            slot.position,
                            format!("component '{}' has no slot '{name}'", definition.name),
                        ));
                    }
                    if named.iter().any(|(existing, _)| *existing == Some(name)) {
                        return Err(error_at(
                            slot.position,
                            format!("slot '{name}' is filled twice"),
                        ));
                    }
                    named.push((Some(name), block.clone()));
                }
                None => default.push(item.clone()),
            },
            _ => default.push(item.clone()),
        }
    }
    let mut fills = Vec::new();
    if !default.is_empty() {
        if !declared.default {
            let message = if declared.any() {
                format!("component '{}' has no default slot", definition.name)
            } else {
                format!("component '{}' does not accept children", definition.name)
            };
            return Err(error_at(element.position, message));
        }
        fills.push((None, Block { items: default }));
    }
    fills.extend(named);
    Ok(fills)
}

fn declared_slots(definition: &Definition) -> Result<Declared<'_>, Error> {
    let mut declared = Declared::default();
    for member in &definition.declaration.members {
        if let Member::Element(root) = member {
            walk_slots(root, definition, &mut declared)?;
        }
    }
    Ok(declared)
}

fn walk_slots<'a>(
    element: &'a Element,
    definition: &Definition,
    declared: &mut Declared<'a>,
) -> Result<(), Error> {
    let call = element.widget.is_none()
        && definition
            .imports
            .iter()
            .any(|import| import.name == element.name);
    walk_block(&element.block, definition, declared, call)
}

fn walk_block<'a>(
    block: &'a Block,
    definition: &Definition,
    declared: &mut Declared<'a>,
    call: bool,
) -> Result<(), Error> {
    for item in &block.items {
        match item {
            BlockItem::Slot(slot) => {
                if slot.block.is_some() {
                    if call {
                        continue;
                    }
                    return Err(error_at(
                        slot.position,
                        "slot content is written at the call site",
                    ));
                }
                match &slot.name {
                    Some(name) => declared.named.push(name.as_str()),
                    None => declared.default = true,
                }
            }
            BlockItem::If(structure) => {
                for branch in &structure.branches {
                    walk_block(&branch.block, definition, declared, false)?;
                }
                if let Some(else_block) = &structure.else_block {
                    walk_block(else_block, definition, declared, false)?;
                }
            }
            BlockItem::Each(structure) => {
                walk_block(&structure.block, definition, declared, false)?;
                if let Some(else_block) = &structure.else_block {
                    walk_block(else_block, definition, declared, false)?;
                }
            }
            BlockItem::Element(child) => walk_slots(child, definition, declared)?,
            BlockItem::Expr(_) => {}
        }
    }
    Ok(())
}

fn filler(block: &Block, emit: &mut Emit<'_>) -> Result<String, Error> {
    let name = emit.structures.next(emit.context.base, "slot");
    let theme = emit.default_theme();
    let mut text = String::new();
    let mut out = Out::new(&mut text);
    out.line(format_args!(
        "#[allow(unused_mut, unused_variables, clippy::let_and_return)]"
    ));
    out.line(format_args!(
        "fn {name}(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {{"
    ));
    out.indented(|out| -> Result<(), Error> {
        out.line(format_args!("let mut bindings = Vec::new();"));
        out.line(format_args!("let mut handlers = Vec::new();"));
        out.line(format_args!("let mut props = Vec::new();"));
        let mut counter = 0;
        if element::contains_slot(block) {
            out.line(format_args!("let mut children = Vec::new();"));
            items_spliced(out, block, emit, Frame::root(theme), &mut counter)?;
        } else {
            out.line(format_args!("let children = vec!["));
            out.indented(|out| element::items(out, block, emit, Frame::root(theme), &mut counter))?;
            out.line(format_args!("];"));
        }
        out.line(format_args!(
            "let widget = brul::gui::Widget::view(brul::gui::Direction::Column, children);"
        ));
        out.line(format_args!(
            "Ok(brul::Built {{ widget, bindings, handlers, props }})"
        ));
        Ok(())
    })?;
    out.line(format_args!("}}"));
    emit.functions.push(text);
    Ok(name)
}

fn collect<'a>(
    element: &'a Element,
    definition: &'a Definition,
    name: &str,
    emit: &mut Emit<'_>,
) -> Result<(Props<'a>, Events<'a>), Error> {
    let mut props = Props::new();
    let mut events = Events::new();
    for arg in &element.args {
        match arg {
            Arg::Class(_) | Arg::Anchor(_) | Arg::Name(_) | Arg::Attach { .. } => {}
            Arg::Handler { event, handler } => events.push(event_of(
                element, definition, name, emit, &events, event, handler,
            )?),
            Arg::Named {
                name: property,
                value,
            } if property == "class" => {}
            Arg::Named {
                name: property,
                value,
            } => props.push(property_of(definition, name, &props, property, value)?),
        }
    }
    Ok((props, events))
}

fn event_of<'a>(
    element: &'a Element,
    definition: &'a Definition,
    name: &str,
    emit: &mut Emit<'_>,
    events: &Events<'a>,
    event: &'a str,
    handler: &'a str,
) -> Result<(&'a str, &'a str, Vec<PropType>), Error> {
    if events.iter().any(|item| item.0 == event) {
        return Err(error_at(
            element.position,
            format!("event '{event}' is handled twice"),
        ));
    }
    let Some(declaration) = find_event(definition, event) else {
        return Err(error_at(
            element.position,
            format!("component '{name}' has no event '{event}'"),
        ));
    };
    let params: Vec<PropType> = declaration
        .params
        .iter()
        .map(|param| param.ty.clone())
        .collect();
    events::define(handler, emit.context.program, &params, element.position)?;
    Ok((event, handler, params))
}

fn property_of<'a>(
    definition: &'a Definition,
    name: &str,
    props: &Props<'a>,
    property: &'a str,
    value: &'a Expr,
) -> Result<(&'a str, &'a PropType, &'a Expr), Error> {
    if props.iter().any(|item| item.0 == property) {
        return Err(error_at(
            value.position,
            format!("property '{property}' is given twice"),
        ));
    }
    let Some(prop) = find_property(definition, property) else {
        return Err(error_at(
            value.position,
            format!("component '{name}' has no property '{property}'"),
        ));
    };
    Ok((property, &prop.ty, value))
}

fn find_event<'a>(definition: &'a Definition, event: &str) -> Option<&'a Event> {
    definition
        .declaration
        .members
        .iter()
        .find_map(|member| match member {
            Member::Event(item) if item.name == event => Some(item),
            _ => None,
        })
}

fn find_property<'a>(definition: &'a Definition, name: &str) -> Option<&'a Prop> {
    definition
        .declaration
        .members
        .iter()
        .find_map(|member| match member {
            Member::Prop(item) if item.name == name => Some(item),
            _ => None,
        })
}

fn defaults<'a>(
    element: &'a Element,
    definition: &'a Definition,
    name: &str,
    props: &Props<'a>,
) -> Result<Props<'a>, Error> {
    let mut defaults = Props::new();
    for member in &definition.declaration.members {
        let Member::Prop(prop) = member else {
            continue;
        };
        if props.iter().any(|item| item.0 == prop.name) {
            continue;
        }
        let Some(default) = &prop.default else {
            return Err(error_at(
                element.position,
                format!(
                    "property '{}' of component '{name}' requires a default value",
                    prop.name
                ),
            ));
        };
        defaults.push((prop.name.as_str(), &prop.ty, default));
    }
    Ok(defaults)
}

fn write(
    out: &mut Out<'_>,
    call: Call<'_>,
    counter: &mut usize,
    emit: &mut Emit<'_>,
    index: usize,
    frame: Frame<'_>,
) -> Result<(), Error> {
    let bases = emit.bases;
    let names = emit.context.names;
    let state = emit.context.state;
    let prefix = emit.context.resource_prefix;
    let resources = emit.context.resources;
    let scopes = &emit.scope_defs;
    let scope = frame.scope;
    let prop_types = emit.context.prop_types;
    let dynamic_classes = &mut *emit.dynamic_classes;
    let base = bases[index].as_str();
    let Call {
        element,
        name,
        props,
        defaults,
        events,
        slots,
        classes,
        class,
        suffix,
    } = call;
    let widget = format!("widget{}", *counter);
    *counter += 1;
    out.line(format_args!("{{"));
    out.indented(|out| -> Result<(), Error> {
        out.line(format_args!(
            "let built = brul::instance(build, component, brul::Instance {{"
        ));
        out.indented(|out| -> Result<(), Error> {
            out.line(format_args!("theme,"));
            out.line(format_args!("view: {base}_view,"));
            out.line(format_args!("script: {base}_script(),"));
            out.line(format_args!("name: {name:?},"));
            out.line(format_args!("names: names(),"));
            arguments(
                out,
                "props",
                &props,
                names,
                emit.context.state,
                emit.context.prop_types,
                resources,
                prefix,
                scopes,
                scope,
            )?;
            arguments(
                out,
                "defaults",
                &defaults,
                names,
                emit.context.state,
                emit.context.prop_types,
                resources,
                prefix,
                scopes,
                scope,
            )?;
            out.line(format_args!("events: &["));
            out.indented(|out| {
                for (event, handler, params) in &events {
                    let params = params
                        .iter()
                        .map(|ty| format!("brul::{}", ty.rust_path()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let call = super::script::event_call(emit.context.script_name, handler);
                    out.line(format_args!(
                        "brul::ComponentEvent {{ name: {event:?}, call: {call}, params: &[{params}], component: std::rc::Rc::clone(component) }},"
                    ));
                }
            });
            out.line(format_args!("],"));
            if slots.is_empty() {
                out.line(format_args!("slots: &[],"));
            } else {
                out.line(format_args!("slots: &["));
                out.indented(|out| {
                    for slot in &slots {
                        let name = match slot.name {
                            Some(name) => format!("{name:?}"),
                            None => "brul::DEFAULT_SLOT".to_owned(),
                        };
                        out.line(format_args!(
                            "brul::SlotArgument {{ name: {name}, view: {} }},",
                            slot.view
                        ));
                    }
                });
                out.line(format_args!("],"));
            }
            Ok(())
        })?;
        out.line(format_args!("}})?;"));
        for field in ["bindings", "handlers", "props"] {
            out.line(format_args!("{field}.extend(built.{field});"));
        }
        if classes.is_empty() {
            out.line(format_args!("let {widget} = built.widget{suffix};"));
        } else {
            let classes = classes
                .iter()
                .map(|class| super::classes::index(emit.classes, class).to_string())
                .collect::<Vec<_>>()
                .join(", ");
            out.line(format_args!(
                "let {widget} = {prefix}styled(built.widget{suffix}, &[{classes}], theme);"
            ));
        }
        if let Some(value) = class {
            *dynamic_classes = true;
            out.line(format_args!(
                "bindings.push(brul::Binding::new(component, {widget}.id(), brul::BindingTarget::Class {{ table: {prefix}class_table(theme), expression: {} }}));",
                expression::closure(value, names, state, prop_types, scopes, scope)?
            ));
        }
        element::attachment_bindings(out, element, &widget, emit.context, scopes, scope)?;
        out.line(format_args!("{widget}"));
        Ok(())
    })?;
    out.start(format_args!("}}"));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn arguments(
    out: &mut Out<'_>,
    field: &str,
    arguments: &[(&str, &PropType, &Expr)],
    names: &Names<'_>,
    state: &super::state::State,
    props: &HashMap<String, Type>,
    resources: &[Resource],
    prefix: &str,
    scopes: &[super::scopes::ScopeInfo],
    scope: Option<usize>,
) -> Result<(), Error> {
    out.line(format_args!("{field}: &["));
    out.indented(|out| -> Result<(), Error> {
        for (name, ty, value) in arguments {
            let expression = match super::resource(value, resources) {
                Some(resource) if **ty == PropType::Image => format!(
                    "brul::PropExpression::Image(|component: &mut brul::Component| -> Result<brul::Dyn, brul::Error> {{ Ok(brul::Dyn::image_bytes({prefix}resource_{resource}())) }})"
                ),
                _ => {
                    let variant = ty.prop_expression();
                    let closure = expression::typed_closure(
                        value, names, state, props, scopes, scope, ty,
                    )?;
                    format!("brul::PropExpression::{variant}({closure})")
                }
            };
            out.line(format_args!(
                "brul::PropArgument {{ name: {name:?}, ty: brul::{}, expression: {expression} }},",
                ty.rust_path()
            ));
        }
        Ok(())
    })?;
    out.line(format_args!("],"));
    Ok(())
}
