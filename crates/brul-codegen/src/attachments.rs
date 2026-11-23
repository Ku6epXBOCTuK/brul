use std::collections::HashMap;
use std::fs;
use std::path::Path;

use brul_case::snake;
use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{FnArg, Ident, Item, ItemFn, LitStr, Pat, Token};

use crate::error_at;
use crate::runtime::AttachEvents;

#[derive(Clone, Debug)]
pub struct AttachParam {
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct AttachSpec {
    pub name: String,
    pub apply: String,
    pub event: String,
    pub cleanup: String,
    pub params: Vec<AttachParam>,
    pub events: AttachEvents,
}

pub type Attachments = HashMap<String, AttachSpec>;

pub fn collect(root: &Path) -> Result<Attachments, Error> {
    let mut attachments = Attachments::new();
    if !root.is_dir() {
        return Ok(attachments);
    }
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
            if !function.attrs.iter().any(is_attach) {
                continue;
            }
            let spec = parse(function)?;
            if attachments.contains_key(&spec.name) {
                return Err(at(format!(
                    "attachment '{}' is registered twice",
                    spec.name
                )));
            }
            attachments.insert(spec.name.clone(), spec);
        }
    }
    Ok(attachments)
}

fn is_attach(attribute: &syn::Attribute) -> bool {
    attribute
        .path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "attach")
}

fn parse(function: &ItemFn) -> Result<AttachSpec, Error> {
    let attribute = function
        .attrs
        .iter()
        .find(|attribute| is_attach(attribute))
        .expect("attach attribute");
    let arguments = if matches!(attribute.meta, syn::Meta::List(_)) {
        attribute
            .parse_args_with(Punctuated::<AttachArg, Token![,]>::parse_terminated)
            .map_err(|error| at(error.to_string()))?
    } else {
        Punctuated::new()
    };
    let mut name = None;
    let mut events = AttachEvents::default();
    for argument in arguments {
        match argument {
            AttachArg::Name(value) => name = Some(value.value()),
            AttachArg::Events(values) => {
                for value in values {
                    match value.as_str() {
                        "Key" => events.key = true,
                        "Ime" => events.ime = true,
                        "Input" => events.input = true,
                        "Submit" => events.submit = true,
                        "Focus" => events.focus = true,
                        "Blur" => events.blur = true,
                        "Click" => events.click = true,
                        _ => return Err(at(format!("unknown attachment event '{value}'"))),
                    }
                }
            }
        }
    }
    let Some(name) = name else {
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
            brul_command::Kind::String | brul_command::Kind::Number | brul_command::Kind::Bool
        ) {
            return Err(at("an attachment param must be Number, String or Bool"));
        }
        params.push(AttachParam {
            name: pattern.ident.to_string(),
        });
    }
    let base = format!("__brul_attach_{}", snake(&function.sig.ident.to_string()));
    Ok(AttachSpec {
        name,
        apply: format!("crate::{base}_apply"),
        event: format!("crate::{base}_event"),
        cleanup: format!("crate::{base}_cleanup"),
        params,
        events,
    })
}

fn at(message: impl Into<String>) -> Error {
    error_at(Position { line: 1, column: 1 }, message)
}

enum AttachArg {
    Name(LitStr),
    Events(Vec<String>),
}

impl Parse for AttachArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        if key == "name" {
            input.parse::<Token![=]>()?;
            return Ok(Self::Name(input.parse()?));
        }
        if key == "events" {
            let content;
            syn::parenthesized!(content in input);
            let events = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
            return Ok(Self::Events(
                events.into_iter().map(|item| item.to_string()).collect(),
            ));
        }
        Err(syn::Error::new(key.span(), "expected `name` or `events`"))
    }
}
