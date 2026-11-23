use std::collections::{BTreeSet, HashMap};

use brul_capability::AttachEventKind;
use brul_case::pascal;
use brul_command::Kind;
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
    pub cleanup: String,
    pub params: Vec<CapabilityParam>,
    pub events: BTreeSet<AttachEventKind>,
}

#[derive(Clone, Debug)]
pub struct CapabilitySpec {
    pub name: String,
    pub view: Option<String>,
    pub props: Vec<CapabilityProp>,
    pub events: Vec<CapabilityEvent>,
    pub hooks: Option<HooksSpec>,
}

pub type CapabilityTable = HashMap<String, CapabilitySpec>;

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
    };
    CapabilityTable::from([(video.name.clone(), video), (webview.name.clone(), webview)])
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
    if capabilities.contains_key(&spec.name) {
        return Err(at(format!(
            "capability '{}' is registered twice",
            spec.name
        )));
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
            cleanup: format!("crate::{}", shims.cleanup),
            params,
            events: arguments.events,
        }),
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
