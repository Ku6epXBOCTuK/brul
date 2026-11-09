use std::fmt::Write;

use brul_syntax::ast::Expr;
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Element, Member, PropType};

use crate::error_at;

use super::Emit;
use super::{events, expression};

#[allow(clippy::too_many_arguments)]
pub(super) fn component(
    out: &mut String,
    element: &Element,
    index: usize,
    emit: &mut Emit<'_>,
    theme: Option<&str>,
    depth: usize,
    counter: &mut usize,
) -> Result<(), Error> {
    let definition = &emit.registry.definitions[index];
    let name = &definition.name;
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
    let mut given: Vec<&str> = Vec::new();
    let mut props: Vec<(&str, PropType, &Expr)> = Vec::new();
    let mut events: Vec<(&str, &str, Vec<PropType>)> = Vec::new();
    for arg in &element.args {
        match arg {
            Arg::Class(_) => {}
            Arg::Handler { event, handler } => {
                if events
                    .iter()
                    .any(|item: &(&str, &str, Vec<PropType>)| item.0 == event.as_str())
                {
                    return Err(error_at(
                        element.position,
                        format!("event '{event}' is handled twice"),
                    ));
                }
                let Some(declaration) =
                    definition
                        .component
                        .members
                        .iter()
                        .find_map(|member| match member {
                            Member::Event(item) if item.name == *event => Some(item),
                            _ => None,
                        })
                else {
                    return Err(error_at(
                        element.position,
                        format!("component '{name}' has no event '{event}'"),
                    ));
                };
                events::define(handler, emit.program, element.position)?;
                events.push((
                    event.as_str(),
                    handler.as_str(),
                    declaration.params.iter().map(|param| param.ty).collect(),
                ));
            }
            Arg::Named {
                name: property,
                value,
            } => {
                if given.contains(&property.as_str()) {
                    return Err(error_at(
                        value.position,
                        format!("property '{property}' is given twice"),
                    ));
                }
                let Some(prop) =
                    definition
                        .component
                        .members
                        .iter()
                        .find_map(|member| match member {
                            Member::Prop(prop) if prop.name == *property => Some(prop),
                            _ => None,
                        })
                else {
                    return Err(error_at(
                        value.position,
                        format!("component '{name}' has no property '{property}'"),
                    ));
                };
                props.push((property.as_str(), prop.ty, value));
                given.push(property);
            }
        }
    }
    let mut defaults: Vec<(&str, PropType, &Expr)> = Vec::new();
    for member in &definition.component.members {
        let Member::Prop(prop) = member else {
            continue;
        };
        if given.contains(&prop.name.as_str()) {
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
    let widget = format!("widget{}", *counter);
    *counter += 1;
    let pad = "    ".repeat(depth);
    let inner = "    ".repeat(depth + 1);
    let theme = match theme {
        Some(theme) => format!("Some({theme:?})"),
        None => "None".to_owned(),
    };
    let _ = writeln!(out, "{pad}{{");
    let _ = writeln!(
        out,
        "{inner}let built = brul::instance(component, brul::Instance {{"
    );
    let _ = writeln!(out, "{inner}    theme: {theme},");
    let _ = writeln!(out, "{inner}    view: component{index}_view,");
    let _ = writeln!(
        out,
        "{inner}    script: component{index}_script().as_ref(),"
    );
    let _ = writeln!(out, "{inner}    name: {name:?},");
    arguments(out, "props", &props, &inner);
    arguments(out, "defaults", &defaults, &inner);
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
    for field in ["bindings", "handlers", "structures", "props"] {
        let _ = writeln!(out, "{inner}{field}.extend(built.{field});");
    }
    let _ = writeln!(out, "{inner}let {widget} = built.widget;");
    let _ = writeln!(out, "{inner}{widget}");
    let _ = write!(out, "{pad}}}");
    Ok(())
}

fn arguments(out: &mut String, field: &str, arguments: &[(&str, PropType, &Expr)], inner: &str) {
    let _ = writeln!(out, "{inner}    {field}: &[");
    for (name, ty, value) in arguments {
        let _ = writeln!(
            out,
            "{inner}        brul::PropArgument {{ name: {name:?}, ty: brul::PropType::{ty:?}, expression: {} }},",
            expression(value)
        );
    }
    let _ = writeln!(out, "{inner}    ],");
}
