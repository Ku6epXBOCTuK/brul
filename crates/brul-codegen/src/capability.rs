use std::collections::{BTreeSet, HashMap};

use brul_capability::AttachEvents;
use brul_case::pascal;
use brul_command::Kind;
use brul_event::RawKind;
use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::types::Type;
use syn::{FnArg, ItemFn, Pat};

use crate::error_at;

#[derive(Clone, Debug)]
pub struct CapabilityProp {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct CapabilityEvent {
    pub name: String,
    pub params: Vec<Type>,
}

#[derive(Clone, Debug)]
pub struct CapabilityParam {
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct HooksSpec {
    pub apply: String,
    pub event: String,
    pub raw_event: String,
    pub cleanup: String,
    pub params: Vec<CapabilityParam>,
    pub events: AttachEvents,
}

#[derive(Clone, Debug)]
pub struct CapabilitySpec {
    pub name: String,
    pub view: Option<String>,
    pub props: Vec<CapabilityProp>,
    pub events: Vec<CapabilityEvent>,
    pub hooks: Option<HooksSpec>,
    pub builtin: bool,
}

pub type CapabilityTable = HashMap<String, CapabilitySpec>;

pub const POLICY_NAMES: [&str; 4] = ["editor", "keyboard", "pointer", "scroll"];

pub fn policy_events(name: &str) -> AttachEvents {
    let raw = match name {
        "editor" => vec![RawKind::Key, RawKind::Ime, RawKind::Paste],
        "keyboard" => vec![RawKind::Key],
        "pointer" => vec![
            RawKind::Press,
            RawKind::Release,
            RawKind::PointerMove,
            RawKind::PointerEnter,
            RawKind::PointerLeave,
            RawKind::PointerCancel,
            RawKind::Drag,
        ],
        "scroll" => vec![RawKind::Wheel],
        _ => unreachable!("unknown builtin policy"),
    };
    AttachEvents {
        semantic: BTreeSet::new(),
        raw: raw.into_iter().collect(),
    }
}

pub(crate) fn builtins() -> CapabilityTable {
    let video = CapabilitySpec {
        name: "Video".to_owned(),
        view: Some("::brul::video::view".to_owned()),
        props: vec![
            CapabilityProp {
                name: "source".to_owned(),
                ty: Type::String,
            },
            CapabilityProp {
                name: "playing".to_owned(),
                ty: Type::Bool,
            },
        ],
        events: vec![
            CapabilityEvent {
                name: "ended".to_owned(),
                params: Vec::new(),
            },
            CapabilityEvent {
                name: "error".to_owned(),
                params: vec![Type::String],
            },
        ],
        hooks: None,
        builtin: true,
    };
    let webview = CapabilitySpec {
        name: "WebView".to_owned(),
        view: Some("::brul::webview::view".to_owned()),
        props: vec![
            CapabilityProp {
                name: "source".to_owned(),
                ty: Type::String,
            },
            CapabilityProp {
                name: "visible".to_owned(),
                ty: Type::Bool,
            },
        ],
        events: vec![CapabilityEvent {
            name: "message".to_owned(),
            params: vec![Type::String],
        }],
        hooks: None,
        builtin: true,
    };
    let mut table =
        CapabilityTable::from([(video.name.clone(), video), (webview.name.clone(), webview)]);
    for name in POLICY_NAMES {
        let shims = format!("::brul::policy::__brul_attach_{name}");
        let spec = CapabilitySpec {
            name: name.to_owned(),
            view: None,
            props: Vec::new(),
            events: Vec::new(),
            hooks: Some(HooksSpec {
                apply: format!("{shims}_apply"),
                event: format!("{shims}_event"),
                raw_event: format!("{shims}_raw_event"),
                cleanup: format!("{shims}_cleanup"),
                params: Vec::new(),
                events: policy_events(name),
            }),
            builtin: true,
        };
        table.insert(spec.name.clone(), spec);
    }
    table
}

pub(crate) fn collect_into(
    capabilities: &mut CapabilityTable,
    function: &ItemFn,
) -> Result<(), Error> {
    let spec = if brul_capability::is_widget(function) {
        parse_view(function)?
    } else {
        parse_hooks(function)?
    };
    if let Some(existing) = capabilities.get(&spec.name) {
        let replaceable = existing.builtin
            && existing.view.is_none()
            && existing.hooks.is_some()
            && !spec.builtin
            && spec.view.is_none()
            && spec.hooks.is_some();
        if !replaceable {
            return Err(at(format!(
                "capability '{}' is registered twice",
                spec.name
            )));
        }
    }
    capabilities.insert(spec.name.clone(), spec);
    Ok(())
}

fn parse_view(function: &ItemFn) -> Result<CapabilitySpec, Error> {
    let attribute = brul_capability::widget_attribute(function).expect("widget attribute");
    let arguments =
        brul_capability::widget_attr(attribute).map_err(|error| at(error.to_string()))?;
    let name = arguments
        .name
        .unwrap_or_else(|| pascal(&function.sig.ident.to_string()));
    let mut inputs = function.sig.inputs.iter();
    for _ in 0..3 {
        if !matches!(inputs.next(), Some(FnArg::Typed(_))) {
            return Err(at("a widget takes theme, build and component first"));
        }
    }
    let mut props = Vec::new();
    for input in inputs {
        let FnArg::Typed(argument) = input else {
            return Err(at("a widget has no receiver"));
        };
        let Pat::Ident(pattern) = &*argument.pat else {
            return Err(at("a widget prop needs a name"));
        };
        let ty = prop_type(brul_command::kind(&argument.ty))?;
        props.push(CapabilityProp {
            name: pattern.ident.to_string(),
            ty,
        });
    }
    let events = arguments
        .events
        .into_iter()
        .map(|event| {
            let params = event
                .params
                .into_iter()
                .map(|param| {
                    Type::from_name(&param)
                        .ok_or_else(|| at(format!("unknown event parameter type '{param}'")))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(CapabilityEvent {
                name: event.name,
                params,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(CapabilitySpec {
        name,
        view: Some(format!(
            "crate::{}",
            brul_capability::widget_shim(&function.sig.ident)
        )),
        props,
        events,
        hooks: None,
        builtin: false,
    })
}

fn parse_hooks(function: &ItemFn) -> Result<CapabilitySpec, Error> {
    let attribute = brul_capability::attach_attribute(function).expect("attach attribute");
    let arguments =
        brul_capability::attach_attr(attribute).map_err(|error| at(error.to_string()))?;
    let Some(name) = arguments.name else {
        return Err(at("an attachment needs a `name`"));
    };
    let mut inputs = function.sig.inputs.iter();
    if !matches!(inputs.next(), Some(FnArg::Typed(_))) {
        return Err(at("an attachment takes a context first"));
    }
    let mut params = Vec::new();
    for input in inputs {
        let FnArg::Typed(argument) = input else {
            return Err(at("an attachment has no receiver"));
        };
        let Pat::Ident(pattern) = &*argument.pat else {
            return Err(at("an attachment param needs a name"));
        };
        if !matches!(
            brul_command::kind(&argument.ty),
            Kind::String | Kind::Number | Kind::Bool
        ) {
            return Err(at("an attachment param must be Number, String or Bool"));
        }
        params.push(CapabilityParam {
            name: pattern.ident.to_string(),
        });
    }
    let shims = brul_capability::attach_shims(&function.sig.ident);
    Ok(CapabilitySpec {
        name,
        view: None,
        props: Vec::new(),
        events: Vec::new(),
        hooks: Some(HooksSpec {
            apply: format!("crate::{}", shims.apply),
            event: format!("crate::{}", shims.event),
            raw_event: format!("crate::{}", shims.raw_event),
            cleanup: format!("crate::{}", shims.cleanup),
            params,
            events: arguments.events,
        }),
        builtin: false,
    })
}

fn prop_type(kind: Kind) -> Result<Type, Error> {
    match kind {
        Kind::String => Ok(Type::String),
        Kind::Number => Ok(Type::Number),
        Kind::Bool => Ok(Type::Bool),
        Kind::Value => Ok(Type::Object),
        Kind::Null | Kind::Other => Err(at("a widget prop must be Number, String, Bool or Object")),
    }
}

fn at(message: impl Into<String>) -> Error {
    error_at(Position { line: 1, column: 1 }, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attach(source: &str) -> ItemFn {
        syn::parse_str(source).expect("attach function")
    }

    #[test]
    fn builtin_policies_are_hooks_only_in_canonical_order() {
        let specs = builtins();
        for name in POLICY_NAMES {
            let spec = specs.get(name).expect("builtin policy");
            assert!(spec.builtin);
            assert!(spec.view.is_none());
            let hooks = spec.hooks.as_ref().expect("hooks");
            assert_eq!(
                hooks.apply,
                format!("::brul::policy::__brul_attach_{name}_apply")
            );
            assert!(hooks.params.is_empty());
        }
        assert!(specs["Video"].builtin);
        assert!(specs["Video"].view.is_some());
        assert!(specs["WebView"].builtin);
        assert!(specs["WebView"].view.is_some());
    }

    #[test]
    fn a_user_attach_replaces_a_builtin_policy() {
        let mut specs = builtins();
        let function = attach(
            "#[brul::attach(name = \"editor\", raw_events(Key))]\n\
             pub fn custom(ctx: &mut Build) -> Result<Editor, ViewError> { Ok(Editor) }",
        );
        collect_into(&mut specs, &function).expect("override");
        let spec = &specs["editor"];
        assert!(!spec.builtin);
        assert!(
            spec.hooks
                .as_ref()
                .is_some_and(|hooks| hooks.apply.starts_with("crate::"))
        );
    }

    #[test]
    fn a_user_attach_cannot_replace_a_view_or_another_user_spec() {
        let mut specs = builtins();
        let view: ItemFn = syn::parse_str(
            "#[brul::widget(name = \"Video\")]\n\
             pub fn video(theme: &Theme, build: &mut Build, component: ComponentHandle, source: String) {}",
        )
        .expect("view function");
        let error = collect_into(&mut specs, &view).expect_err("view collision");
        assert!(error.to_string().contains("registered twice"));

        let first = attach(
            "#[brul::attach(name = \"custom\")]\n\
             pub fn custom(ctx: &mut Build) -> Result<Custom, ViewError> { Ok(Custom) }",
        );
        collect_into(&mut specs, &first).expect("first attachment");
        let second = attach(
            "#[brul::attach(name = \"custom\")]\n\
             pub fn other(ctx: &mut Build) -> Result<Custom, ViewError> { Ok(Custom) }",
        );
        assert!(collect_into(&mut specs, &second).is_err());
    }
}
