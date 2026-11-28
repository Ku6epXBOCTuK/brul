use brul_syntax::ast::Expr;
use brul_syntax::error::Error;
use brul_syntax::markup::PropType;

use super::CodegenContext;
use super::Emit;
use super::Frame;
use super::Out;
use super::ScopeInfo;

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
    let scopes = &emit.scope_defs;
    let scope = frame.scope;
    let base = bases[index].as_str();
    let Call {
        element,
        name,
        props,
        defaults,
        events,
        slots,
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
            arguments(out, "props", &props, emit.context, scopes, scope)?;
            arguments(out, "defaults", &defaults, emit.context, scopes, scope)?;
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
        out.line(format_args!("let {widget} = built.widget{suffix};"));
        element::attachment_bindings(out, element, &widget, emit.context, scopes, scope)?;
        out.line(format_args!("{widget}"));
        Ok(())
    })?;
    out.start(format_args!("}}"));
    Ok(())
}

pub(super) fn arguments(
    out: &mut Out<'_>,
    field: &str,
    arguments: &[(&str, &PropType, &Expr)],
    codegen: &CodegenContext<'_>,
    scopes: &[ScopeInfo],
    scope: Option<usize>,
) -> Result<(), Error> {
    let context = codegen.expr_context();
    let resources = codegen.resources;
    let prefix = codegen.resource_prefix;
    out.line(format_args!("{field}: &["));
    out.indented(|out| -> Result<(), Error> {
        for (name, ty, value) in arguments {
            let expression = match super::resource(value, resources) {
                Some(resource) if **ty == PropType::Image => format!(
                    "brul::PropExpression::Image(|component: &mut brul::Component| -> Result<brul::Dyn, brul::Error> {{ Ok(brul::Dyn::image_bytes({prefix}resource_{resource}())) }})"
                ),
                _ => {
                    let variant = ty.prop_expression();
                    let closure =
                        expression::typed_closure(value, &context, scopes, scope, ty)?;
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
