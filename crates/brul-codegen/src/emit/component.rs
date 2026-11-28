use brul_syntax::ast::Expr;
use brul_syntax::error::Error;
use brul_syntax::markup::{
    Arg, Block, BlockItem, Element, Event, Member, Prop, PropType, Slot, StructureRef,
};

use crate::error_at;
use crate::loader::Definition;

use super::Emit;
use super::Frame;
use super::Out;
use super::anchor;
use super::{element, events, items_spliced};

pub(super) use super::component_write::arguments;
use super::component_write::write;

pub(super) type Props<'a> = Vec<(&'a str, &'a PropType, &'a Expr)>;
pub(super) type Events<'a> = Vec<(&'a str, &'a str, Vec<PropType>)>;

pub(super) struct Call<'a> {
    pub(super) element: &'a Element,
    pub(super) name: &'a str,
    pub(super) props: Props<'a>,
    pub(super) defaults: Props<'a>,
    pub(super) events: Events<'a>,
    pub(super) slots: Vec<SlotFill<'a>>,
    pub(super) suffix: String,
}

pub(super) struct SlotFill<'a> {
    pub(super) name: Option<&'a str>,
    pub(super) view: String,
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
    if element.args.iter().any(|arg| {
        matches!(arg, Arg::Class(_)) || matches!(arg, Arg::Named { name, .. } if name == "class")
    }) {
        return Err(error_at(
            element.position,
            format!("component '{name}' does not accept classes"),
        ));
    }
    if matches!(
        crate::root::root_shape(&definition.declaration)?,
        crate::root::RootShape::Fragment(_)
    ) {
        for arg in &element.args {
            let position = match arg {
                Arg::Attach { position, .. } => Some(*position),
                Arg::Name(anchor) | Arg::Anchor(anchor) => Some(anchor.position),
                _ => None,
            };
            if let Some(position) = position {
                return Err(error_at(
                    position,
                    format!("component '{name}' has no widget for this argument"),
                ));
            }
        }
    }
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
        match member.structure() {
            Some(StructureRef::Element(root)) => walk_slots(root, definition, &mut declared)?,
            Some(StructureRef::Slot(slot)) => declare_slot(slot, &mut declared)?,
            Some(StructureRef::If(block)) => {
                for branch in &block.branches {
                    walk_block(&branch.block, definition, &mut declared, false)?;
                }
                if let Some(else_block) = &block.else_block {
                    walk_block(else_block, definition, &mut declared, false)?;
                }
            }
            Some(StructureRef::Each(block)) => {
                walk_block(&block.block, definition, &mut declared, false)?;
                if let Some(else_block) = &block.else_block {
                    walk_block(else_block, definition, &mut declared, false)?;
                }
            }
            None => {}
        }
    }
    Ok(declared)
}

fn declare_slot<'a>(slot: &'a Slot, declared: &mut Declared<'a>) -> Result<(), Error> {
    if slot.block.is_some() {
        return Err(error_at(
            slot.position,
            "slot content is written at the call site",
        ));
    }
    match &slot.name {
        Some(name) => declared.named.push(name.as_str()),
        None => declared.default = true,
    }
    Ok(())
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
