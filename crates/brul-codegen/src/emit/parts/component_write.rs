use brul_script::Names;
use std::collections::HashMap;

use brul_syntax::ast::Expr;
use brul_syntax::error::Error;
use brul_syntax::markup::PropType;
use brul_syntax::types::Type;

use crate::Resource;

use super::Emit;
use super::Frame;
use super::Out;

use super::{element, expression};

use super::component::Call;

pub(super) fn write(
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
