use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use brul_capability::AttachEventKind;
use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use syn::{FnArg, Item, ItemFn, Pat};

use crate::error_at;

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
    pub events: BTreeSet<AttachEventKind>,
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
            if !brul_capability::is_attach(function) {
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

fn parse(function: &ItemFn) -> Result<AttachSpec, Error> {
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
            brul_command::Kind::String | brul_command::Kind::Number | brul_command::Kind::Bool
        ) {
            return Err(at("an attachment param must be Number, String or Bool"));
        }
        params.push(AttachParam {
            name: pattern.ident.to_string(),
        });
    }
    let shims = brul_capability::attach_shims(&function.sig.ident);
    Ok(AttachSpec {
        name,
        apply: format!("crate::{}", shims.apply),
        event: format!("crate::{}", shims.event),
        cleanup: format!("crate::{}", shims.cleanup),
        params,
        events: arguments.events,
    })
}

fn at(message: impl Into<String>) -> Error {
    error_at(Position { line: 1, column: 1 }, message)
}
