use std::collections::HashMap;
use std::fs;
use std::path::Path;

use brul_case::pascal;
use brul_command::Kind;
use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::types::Type;
use syn::{FnArg, Item, ItemFn, Pat};

use crate::error_at;

#[derive(Clone, Debug)]
pub struct WidgetProp {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct WidgetEvent {
    pub name: String,
    pub params: Vec<Type>,
}

#[derive(Clone, Debug)]
pub struct WidgetSpec {
    pub name: String,
    pub view: String,
    pub props: Vec<WidgetProp>,
    pub events: Vec<WidgetEvent>,
}

pub type Widgets = HashMap<String, WidgetSpec>;

pub fn collect(root: &Path) -> Result<Widgets, Error> {
    let mut widgets = builtins();
    let sources = crate::commands::rust_sources(root)
        .map_err(|error| at(format!("cannot read '{}': {error}", root.display())))?;
    for path in sources {
        let source = fs::read_to_string(&path)
            .map_err(|error| at(format!("cannot read '{}': {error}", path.display())))?;
        let file = syn::parse_file(&source).map_err(|error| at(error.to_string()))?;
        for item in &file.items {
            let Item::Fn(function) = item else {
                continue;
            };
            if !brul_capability::is_widget(function) {
                continue;
            }
            let spec = parse(function)?;
            if widgets.contains_key(&spec.name) {
                return Err(at(format!("widget '{}' is registered twice", spec.name)));
            }
            widgets.insert(spec.name.clone(), spec);
        }
    }
    Ok(widgets)
}

pub(crate) fn builtins() -> Widgets {
    let video = WidgetSpec {
        name: "Video".to_owned(),
        view: "::brul::video::view".to_owned(),
        props: vec![
            WidgetProp {
                name: "source".to_owned(),
                ty: Type::String,
            },
            WidgetProp {
                name: "playing".to_owned(),
                ty: Type::Bool,
            },
        ],
        events: vec![
            WidgetEvent {
                name: "ended".to_owned(),
                params: Vec::new(),
            },
            WidgetEvent {
                name: "error".to_owned(),
                params: vec![Type::String],
            },
        ],
    };
    let webview = WidgetSpec {
        name: "WebView".to_owned(),
        view: "::brul::webview::view".to_owned(),
        props: vec![
            WidgetProp {
                name: "source".to_owned(),
                ty: Type::String,
            },
            WidgetProp {
                name: "visible".to_owned(),
                ty: Type::Bool,
            },
        ],
        events: vec![WidgetEvent {
            name: "message".to_owned(),
            params: vec![Type::String],
        }],
    };
    Widgets::from([(video.name.clone(), video), (webview.name.clone(), webview)])
}

fn parse(function: &ItemFn) -> Result<WidgetSpec, Error> {
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
        props.push(WidgetProp {
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
            Ok(WidgetEvent {
                name: event.name,
                params,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(WidgetSpec {
        name,
        view: format!(
            "crate::{}",
            brul_capability::widget_shim(&function.sig.ident)
        ),
        props,
        events,
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
