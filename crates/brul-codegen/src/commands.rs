use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::types::Type;
use syn::{GenericArgument, Item, PathArguments, ReturnType, Type as SynType};

use crate::error_at;

#[derive(Clone, Debug)]
pub struct Command {
    pub name: String,
    pub marker: String,
    pub params: Vec<Type>,
    pub result: Type,
}

pub type Commands = HashMap<String, Command>;

pub fn collect(root: &Path) -> Result<Commands, Error> {
    let mut commands = Commands::new();
    let sources = rust_sources(root).map_err(|error| {
        error_at(
            Position { line: 1, column: 1 },
            format!("cannot read '{}': {error}", root.display()),
        )
    })?;
    for path in sources {
        let source = fs::read_to_string(&path).map_err(|error| {
            error_at(
                Position { line: 1, column: 1 },
                format!("cannot read '{}': {error}", path.display()),
            )
        })?;
        let file = syn::parse_file(&source).map_err(|error| {
            error_at(
                Position { line: 1, column: 1 },
                format!("cannot parse '{}': {error}", path.display()),
            )
        })?;
        for item in &file.items {
            let Item::Fn(function) = item else {
                continue;
            };
            if !function.attrs.iter().any(|attribute| {
                attribute
                    .path()
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "command")
            }) {
                continue;
            }
            let name = function.sig.ident.to_string();
            let marker = format!("__brul_command_{}", snake(&name));
            let mut inputs = function.sig.inputs.iter();
            inputs.next();
            let params = inputs
                .filter_map(|input| match input {
                    syn::FnArg::Typed(argument) => Some(rust_type(&argument.ty)),
                    syn::FnArg::Receiver(_) => None,
                })
                .collect();
            let result = match &function.sig.output {
                ReturnType::Default => Type::Null,
                ReturnType::Type(_, ty) => result_type(ty),
            };
            let command = Command {
                name: camel(&name),
                marker,
                params,
                result,
            };
            commands.insert(command.name.clone(), command.clone());
            commands.insert(name, command);
        }
    }
    Ok(commands)
}

pub fn names(commands: &Commands) -> Vec<&str> {
    let mut names: Vec<&str> = commands.keys().map(String::as_str).collect();
    names.sort_unstable();
    names
}

fn rust_sources(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(rust_sources(&path)?);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    Ok(found)
}

fn result_type(ty: &SynType) -> Type {
    let SynType::Path(path) = ty else {
        return rust_type(ty);
    };
    let Some(segment) = path.path.segments.last() else {
        return Type::Object;
    };
    if segment.ident != "Result" {
        return rust_type(ty);
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Type::Object;
    };
    arguments
        .args
        .iter()
        .find_map(|argument| match argument {
            GenericArgument::Type(ty) => Some(rust_type(ty)),
            _ => None,
        })
        .unwrap_or(Type::Object)
}

fn rust_type(ty: &SynType) -> Type {
    match ty {
        SynType::Reference(reference) => rust_type(&reference.elem),
        SynType::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| match segment.ident.to_string().as_str() {
                "str" | "String" => Type::String,
                "f64" | "f32" => Type::Number,
                "bool" => Type::Bool,
                "usize" | "u64" | "u32" | "i64" | "i32" => Type::Number,
                _ => Type::Object,
            })
            .unwrap_or(Type::Object),
        SynType::Tuple(tuple) if tuple.elems.is_empty() => Type::Null,
        _ => Type::Object,
    }
}

fn camel(name: &str) -> String {
    let name = name.trim_start_matches("r#");
    let mut output = String::with_capacity(name.len());
    let mut capitalize = false;
    for character in name.chars() {
        if character == '_' {
            capitalize = true;
        } else if capitalize {
            output.extend(character.to_uppercase());
            capitalize = false;
        } else {
            output.push(character);
        }
    }
    output
}

fn snake(name: &str) -> String {
    let mut output = String::with_capacity(name.len());
    for (index, character) in name.chars().enumerate() {
        if character.is_uppercase() {
            if index > 0 {
                output.push('_');
            }
            output.extend(character.to_lowercase());
        } else {
            output.push(character);
        }
    }
    output
}
