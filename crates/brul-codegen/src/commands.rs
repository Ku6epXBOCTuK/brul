use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use brul_case::{camel, snake};
use brul_command::{Kind, is_command};
use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::types::Type;
use syn::Item;

use crate::error_at;

#[derive(Clone, Debug)]
pub struct Command {
    pub name: String,
    pub path: String,
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
            if !is_command(function) {
                continue;
            }
            let signature = brul_command::parse(function)
                .map_err(|error| error_at(Position { line: 1, column: 1 }, error.to_string()))?;
            let name = signature.name.to_string();
            let path = format!("crate::__brul_command_{}", snake(&name));
            let params = signature
                .params
                .iter()
                .map(|(_, ty)| command_type(brul_command::kind(ty)))
                .collect();
            let result = match signature.output {
                syn::ReturnType::Default => Type::Null,
                _ => brul_command::result_type(signature.output)
                    .map(|ty| command_type(brul_command::kind(ty)))
                    .unwrap_or(Type::Object),
            };
            let command = Command {
                name: camel(&name),
                path,
                params,
                result,
            };
            commands.insert(command.name.clone(), command.clone());
            commands.insert(name, command);
        }
    }
    Ok(commands)
}

pub fn read(path: &Path) -> Result<Commands, Error> {
    let text = fs::read_to_string(path).map_err(|error| {
        error_at(
            Position { line: 1, column: 1 },
            format!("cannot read '{}': {error}", path.display()),
        )
    })?;
    let mut commands = Commands::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let position = Position {
            line: index + 1,
            column: 1,
        };
        let fields: Vec<&str> = line.split('|').collect();
        let [name, address, params, result] = fields.as_slice() else {
            return Err(error_at(position, format!("invalid command line '{line}'")));
        };
        let params = params
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(|name| parse_type(name, position))
            .collect::<Result<Vec<_>, _>>()?;
        let command = Command {
            name: name.trim().to_owned(),
            path: address.trim().to_owned(),
            params,
            result: parse_type(result.trim(), position)?,
        };
        commands.insert(command.name.clone(), command);
    }
    Ok(commands)
}

fn parse_type(name: &str, position: Position) -> Result<Type, Error> {
    Type::from_name(name)
        .ok_or_else(|| error_at(position, format!("unknown command type '{name}'")))
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

fn command_type(kind: Kind) -> Type {
    match kind {
        Kind::String => Type::String,
        Kind::Number => Type::Number,
        Kind::Bool => Type::Bool,
        Kind::Null => Type::Null,
        Kind::Value | Kind::Other => Type::Object,
    }
}
