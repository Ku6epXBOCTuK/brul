use brul_case::constant;
use brul_event::{EventKind, RawKind};
use brul_syntax::ast::{Expr, Position};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Element};
use brul_syntax::style::EffectRule;

use crate::error_at;
use crate::window::AttachParam;

use super::CodegenContext;
use super::Emit;
use super::Frame;
use super::Out;
use super::classes;
use super::events;
use super::expression;

use super::{ScopeInfo, StateKey, child};

use super::element::Content;

pub(super) fn content_binding(
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
            &emit.expr_context(),
            &emit.scope_defs,
            frame.scope,
        )?
    } else {
        expression::typed_closure(
            &expression,
            &emit.expr_context(),
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

pub(super) fn effect_binding(
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
            &emit.expr_context(),
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

pub(super) fn handler_bindings(
    out: &mut Out<'_>,
    handlers: &[events::HandlerSpec],
    name: &str,
    script: &str,
) {
    for handler in handlers {
        let call = super::script::event_call(script, &handler.function);
        out.line(format_args!(
            "handlers.push(brul::Handler::new(component, Some({name}.id()), brul::EventKind::{:?}, {call}));",
            handler.event
        ));
    }
}

pub(super) fn attachment_bindings(
    out: &mut Out<'_>,
    element: &Element,
    widget: &str,
    context: &CodegenContext<'_>,
    scopes: &[ScopeInfo],
    scope: Option<usize>,
) -> Result<(), Error> {
    let target = format!("brul::EventTarget::Widget({widget}.id())");
    for arg in &element.args {
        if let Arg::Attach {
            name,
            params,
            position,
        } = arg
        {
            let path = child(&element.name, &format!("attach:{name}"));
            let id = context
                .state_ids
                .borrow_mut()
                .register(context.base, &path)
                .map_err(|error| error_at(*position, error))?;
            let params: Vec<AttachParam> = params
                .iter()
                .map(|(name, value)| AttachParam {
                    name: name.clone(),
                    value: value.clone(),
                })
                .collect();
            hooks_binding(
                out,
                HooksSpec {
                    name,
                    params: &params,
                    position: *position,
                    target: &target,
                    id: &id,
                    context,
                    scopes,
                    scope,
                },
            )?;
        }
    }
    Ok(())
}

pub(super) struct HooksSpec<'a> {
    pub(super) name: &'a str,
    pub(super) params: &'a [AttachParam],
    pub(super) position: Position,
    pub(super) target: &'a str,
    pub(super) id: &'a StateKey,
    pub(super) context: &'a CodegenContext<'a>,
    pub(super) scopes: &'a [ScopeInfo],
    pub(super) scope: Option<usize>,
}

pub(super) fn hooks_binding(out: &mut Out<'_>, spec: HooksSpec<'_>) -> Result<(), Error> {
    let HooksSpec {
        name,
        params,
        position,
        target,
        id,
        context,
        scopes,
        scope,
    } = spec;
    let Some(spec) = context.capabilities.specs.get(name).cloned() else {
        return Err(error_at(position, format!("unknown attachment '{name}'")));
    };
    let Some(hooks) = spec.hooks else {
        return Err(error_at(
            position,
            format!("capability '{name}' has no hooks"),
        ));
    };
    let mut expressions = Vec::with_capacity(hooks.params.len());
    for expected in &hooks.params {
        let Some(given) = params
            .iter()
            .find(|given| given.name == expected.name.as_str())
        else {
            return Err(error_at(
                position,
                format!("attachment '{name}' needs a '{}' parameter", expected.name),
            ));
        };
        expressions.push(expression::closure(
            &given.value,
            &context.expr_context(),
            scopes,
            scope,
        )?);
    }
    if params.len() != hooks.params.len() {
        return Err(error_at(
            position,
            format!(
                "attachment '{name}' takes {} parameters",
                hooks.params.len()
            ),
        ));
    }
    let events = &hooks.events.semantic;
    let raw_events = &hooks.events.raw;
    out.start(format_args!(
        "let _ = build.capability(brul::Capability::hooks(brul::Hooks {{\n"
    ));
    out.indented(|out| {
        out.line(format_args!("target: {target},"));
        out.line(format_args!("state: {},", id.literal()));
        out.line(format_args!("component: std::rc::Rc::clone(component),"));
        out.line(format_args!("params: vec![{}],", expressions.join(", ")));
        out.line(format_args!("apply: {},", hooks.apply));
        out.line(format_args!("event: {},", hooks.event));
        out.line(format_args!("raw_event: {},", hooks.raw_event));
        out.line(format_args!("cleanup: {},", hooks.cleanup));
        let kinds = EventKind::ALL
            .iter()
            .copied()
            .filter(|kind| {
                kind.dispatch_kind() == *kind
                    && (events.contains(kind) || events.contains(&kind.dispatch_kind()))
            })
            .map(|kind| format!("brul::EventKind::{}", kind.field()))
            .collect::<Vec<_>>()
            .join(", ");
        out.line(format_args!("events: brul::EventSet::from([{kinds}]),"));
        let raw_kinds = RawKind::ALL
            .iter()
            .copied()
            .filter(|kind| kind.dispatch_kind() == *kind && raw_events.contains(kind))
            .map(|kind| format!("brul::RawKind::{}", kind.field()))
            .collect::<Vec<_>>()
            .join(", ");
        out.line(format_args!(
            "raw_events: brul::RawEventSet::from([{raw_kinds}]),"
        ));
        Ok::<(), Error>(())
    })?;
    out.line(format_args!("}}));"));
    Ok(())
}

pub(super) fn parameters(
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
            &emit.expr_context(),
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
        let table = classes::table(emit.context.resource_prefix);
        out.line(format_args!(
            "bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::Class {{ table: {table}, expression: {} }}));",
            expression::closure(value, &emit.expr_context(), &emit.scope_defs, frame.scope)?
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
                &emit.expr_context(),
                &emit.scope_defs,
                frame.scope,
                &brul_syntax::types::Type::Number,
            )?
        ),
        None => "None".to_owned(),
    })
}
