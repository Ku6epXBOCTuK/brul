use brul_syntax::ast::{Expr, ExprKind};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, BlockItem, Element, Slot};
use brul_syntax::types::Type;

use crate::capability::CapabilitySpec;
use crate::error_at;

use super::Emit;
use super::Frame;
use super::Out;
use super::anchor;
use super::classes;
use super::expression;
use super::rust_string;
use super::{child, component, events};

use super::element::{attachment_bindings, classes, debug_named};

pub(super) fn slot_item(out: &mut Out<'_>, slot: &Slot, emit: &Emit<'_>) -> Result<(), Error> {
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

pub(super) fn text_widget(out: &mut Out<'_>, element: &Element) -> Result<Option<Expr>, Error> {
    field_widget(out, element, "text", "Text")
}

pub(super) fn input_widget(out: &mut Out<'_>, element: &Element) -> Result<Option<Expr>, Error> {
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
                "brul::gui::Widget::{constructor}({})",
                rust_string(content)
            ));
            Ok(None)
        }
        _ => Err(error_at(
            element.position,
            format!("{label} requires a single content expression"),
        )),
    }
}

pub(super) fn image_widget(
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
    if let Some(name) = super::resource(expression, emit.context.resources) {
        out.push(format_args!(
            "brul::gui::Widget::image(brul::gui::Image::from_bytes({}resource_{name}()).expect(\"resource '{name}' cannot be decoded\"))",
            emit.context.resource_prefix
        ));
        return Ok(None);
    }
    out.push(format_args!(
        "brul::gui::Widget::image(brul::gui::Texture::empty())"
    ));
    Ok(Some(expression.clone()))
}

pub(super) fn registered_widget(
    out: &mut Out<'_>,
    element: &Element,
    spec: &CapabilitySpec,
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
                events::define(handler, emit.context.program, &params, element.position)?;
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
    let props: Vec<(&str, &Type, &Expr)> = owned
        .iter()
        .map(|(name, ty, value)| (*name, ty, *value))
        .collect();
    let defaults: Vec<Expr> = spec
        .props
        .iter()
        .filter_map(|prop| {
            prop.default
                .clone()
                .map(|kind| Expr::new(kind, element.position))
        })
        .collect();
    let default_args: Vec<(&str, &Type, &Expr)> = spec
        .props
        .iter()
        .filter(|prop| prop.default.is_some())
        .zip(defaults.iter())
        .map(|(prop, value)| (prop.name.as_str(), &prop.ty, value))
        .collect();
    let path = child(&element.name, "view");
    let id = emit
        .context
        .state_ids
        .borrow_mut()
        .register(emit.context.base, &path)
        .map_err(|error| error_at(element.position, error))?;
    let element_classes = classes(element);
    let class_indices = element_classes
        .iter()
        .map(|class| classes::index(&mut *emit.classes, class))
        .collect::<Vec<_>>();
    let name = format!("widget{}", *counter);
    *counter += 1;
    let mut suffix = anchor::suffix(element, emit)?;
    suffix.push_str(&debug_named(element));
    out.line(format_args!("{{"));
    out.indented(|out| -> Result<(), Error> {
        out.line(format_args!("let built = build.capability(brul::Capability::view({}, std::rc::Rc::clone(component), brul::Instance {{", id.literal()));
        out.indented(|out| -> Result<(), Error> {
            out.line(format_args!("theme,"));
            out.line(format_args!("view: {},", spec.view.as_deref().expect("view capability")));
            out.line(format_args!("script: None,"));
            out.line(format_args!("name: {:?},", spec.name));
            out.line(format_args!("names: names(),"));
            component::arguments(out, "props", &props, emit.context, &emit.scope_defs, frame.scope)?;
            if default_args.is_empty() {
                out.line(format_args!("defaults: &[],"));
            } else {
                component::arguments(
                    out,
                    "defaults",
                    &default_args,
                    emit.context,
                    &emit.scope_defs,
                    frame.scope,
                )?;
            }
            out.line(format_args!("events: &["));
            out.indented(|out| {
                for (event, handler, params) in &handlers {
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
            out.line(format_args!("slots: &[],"));
            Ok(())
        })?;
        out.line(format_args!("}}))?.expect(\"view capability\");"));
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
                emit.context.resource_prefix
            ));
        }
        if let Some(value) = class {
            *emit.dynamic_classes = true;
            out.line(format_args!(
                "bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::Class {{ table: {}, expression: {} }}));",
                classes::table(emit.context.resource_prefix),
                expression::closure(value, &emit.expr_context(), &emit.scope_defs, frame.scope)?
            ));
        }
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
