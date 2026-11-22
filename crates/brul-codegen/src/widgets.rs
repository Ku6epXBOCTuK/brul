use std::collections::HashMap;
use std::fs;
use std::path::Path;

use brul_case::{pascal, snake};
use brul_command::Kind;
use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::types::Type;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{FnArg, Ident, Item, ItemFn, LitStr, Pat, Token};

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
    let mut widgets = Widgets::new();
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
            if !is_widget(function) {
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

pub fn names(widgets: &Widgets) -> Vec<&str> {
    let mut names: Vec<&str> = widgets
        .values()
        .flat_map(|widget| widget.props.iter().map(|prop| prop.name.as_str()))
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

fn is_widget(function: &ItemFn) -> bool {
    function.attrs.iter().any(|attribute| {
        attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "widget")
    })
}

fn parse(function: &ItemFn) -> Result<WidgetSpec, Error> {
    let attribute = function
        .attrs
        .iter()
        .find(|attribute| {
            attribute
                .path()
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "widget")
        })
        .expect("widget attribute");
    let arguments = if matches!(attribute.meta, syn::Meta::List(_)) {
        attribute
            .parse_args_with(Punctuated::<WidgetArg, Token![,]>::parse_terminated)
            .map_err(|error| at(error.to_string()))?
    } else {
        Punctuated::new()
    };
    let mut name = None;
    let mut events = Vec::new();
    for argument in arguments {
        match argument {
            WidgetArg::Name(value) => name = Some(value.value()),
            WidgetArg::Events(values) => events = values,
        }
    }
    let name = name.unwrap_or_else(|| pascal(&function.sig.ident.to_string()));
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
    let events = events
        .into_iter()
        .map(|event| WidgetEvent {
            name: event.name,
            params: event.params,
        })
        .collect();
    Ok(WidgetSpec {
        name,
        view: format!(
            "crate::__brul_widget_{}",
            snake(&function.sig.ident.to_string())
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

enum WidgetArg {
    Name(LitStr),
    Events(Vec<EventArg>),
}

struct EventArg {
    name: String,
    params: Vec<Type>,
}

impl Parse for WidgetArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        if key == "name" {
            input.parse::<Token![=]>()?;
            return Ok(Self::Name(input.parse()?));
        }
        if key == "events" {
            let content;
            syn::parenthesized!(content in input);
            let events = Punctuated::<EventArg, Token![,]>::parse_terminated(&content)?;
            return Ok(Self::Events(events.into_iter().collect()));
        }
        Err(syn::Error::new(key.span(), "expected `name` or `events`"))
    }
}

impl Parse for EventArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let content;
        syn::bracketed!(content in input);
        let params = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
        let params = params
            .into_iter()
            .map(|param| {
                Type::from_name(&param.to_string())
                    .ok_or_else(|| syn::Error::new(param.span(), "unknown event parameter type"))
            })
            .collect::<syn::Result<Vec<_>>>()?;
        Ok(Self {
            name: name.to_string(),
            params,
        })
    }
}
