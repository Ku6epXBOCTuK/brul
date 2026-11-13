use std::fmt::Write;

use brul_script::Names;
use brul_syntax::ast::Expr;
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Element, Event, Member, Prop, PropType};

use crate::error_at;
use crate::loader::Definition;

use super::Emit;
use super::Frame;
use super::{events, expression};

type Props<'a> = Vec<(&'a str, PropType, &'a Expr)>;
type Events<'a> = Vec<(&'a str, &'a str, Vec<PropType>)>;

struct Call<'a> {
    name: &'a str,
    props: Props<'a>,
    defaults: Props<'a>,
    events: Events<'a>,
}

pub(super) fn component(
    out: &mut String,
    element: &Element,
    index: usize,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    let definition = &emit.registry.definitions[index];
    let name = &definition.name;
    check_shape(element, name)?;
    let (props, events) = collect(element, definition, name, emit)?;
    let defaults = defaults(element, definition, name, &props)?;
    write(
        out,
        index,
        Call {
            name,
            props,
            defaults,
            events,
        },
        frame,
        counter,
        emit.names,
    );
    Ok(())
}

fn check_shape(element: &Element, name: &str) -> Result<(), Error> {
    if element.args.iter().any(|arg| matches!(arg, Arg::Class(_))) {
        return Err(error_at(
            element.position,
            format!("component '{name}' does not accept classes yet"),
        ));
    }
    if !element.block.items.is_empty() {
        return Err(error_at(
            element.position,
            format!("component '{name}' does not accept children"),
        ));
    }
    Ok(())
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
            Arg::Class(_) => {}
            Arg::Handler { event, handler } => events.push(event_of(
                element, definition, name, emit, &events, event, handler,
            )?),
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
    events::define(handler, emit.program, element.position)?;
    let params = declaration.params.iter().map(|param| param.ty).collect();
    Ok((event, handler, params))
}

fn property_of<'a>(
    definition: &'a Definition,
    name: &str,
    props: &Props<'a>,
    property: &'a str,
    value: &'a Expr,
) -> Result<(&'a str, PropType, &'a Expr), Error> {
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
    Ok((property, prop.ty, value))
}

fn find_event<'a>(definition: &'a Definition, event: &str) -> Option<&'a Event> {
    definition
        .component
        .members
        .iter()
        .find_map(|member| match member {
            Member::Event(item) if item.name == event => Some(item),
            _ => None,
        })
}

fn find_property<'a>(definition: &'a Definition, name: &str) -> Option<&'a Prop> {
    definition
        .component
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
    for member in &definition.component.members {
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
        defaults.push((prop.name.as_str(), prop.ty, default));
    }
    Ok(defaults)
}

fn write(
    out: &mut String,
    index: usize,
    call: Call<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
    names: &Names<'_>,
) {
    let Call {
        name,
        props,
        defaults,
        events,
    } = call;
    let widget = format!("widget{}", *counter);
    *counter += 1;
    let pad = "    ".repeat(frame.depth());
    let inner = "    ".repeat(frame.depth() + 1);
    let _ = writeln!(out, "{pad}{{");
    let _ = writeln!(
        out,
        "{inner}let built = brul::instance(component, brul::Instance {{"
    );
    let _ = writeln!(out, "{inner}    theme,");
    let _ = writeln!(out, "{inner}    view: component{index}_view,");
    let _ = writeln!(out, "{inner}    script: component{index}_script(),");
    let _ = writeln!(out, "{inner}    name: {name:?},");
    let _ = writeln!(out, "{inner}    names: names(),");
    arguments(out, "props", &props, &inner, names);
    arguments(out, "defaults", &defaults, &inner, names);
    let _ = writeln!(out, "{inner}    events: &[");
    for (event, handler, params) in &events {
        let params = params
            .iter()
            .map(|ty| format!("brul::PropType::{ty:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            out,
            "{inner}        brul::ComponentEvent {{ name: {event:?}, function: {handler:?}, params: &[{params}], component: std::rc::Rc::clone(component) }},"
        );
    }
    let _ = writeln!(out, "{inner}    ],");
    let _ = writeln!(out, "{inner}}})?;");
    for field in ["bindings", "handlers", "props"] {
        let _ = writeln!(out, "{inner}{field}.extend(built.{field});");
    }
    let _ = writeln!(out, "{inner}let {widget} = built.widget;");
    let _ = writeln!(out, "{inner}{widget}");
    let _ = write!(out, "{pad}}}");
}

fn arguments(
    out: &mut String,
    field: &str,
    arguments: &[(&str, PropType, &Expr)],
    inner: &str,
    names: &Names<'_>,
) {
    let _ = writeln!(out, "{inner}    {field}: &[");
    for (name, ty, value) in arguments {
        let id = names
            .get(name)
            .expect("prop is collected into the name table")
            .index();
        let _ = writeln!(
            out,
            "{inner}        brul::PropArgument {{ name: brul::VariableId::from_index({id}), ty: brul::PropType::{ty:?}, expression: {} }},",
            expression::closure(value, names)
        );
    }
    let _ = writeln!(out, "{inner}    ],");
}
