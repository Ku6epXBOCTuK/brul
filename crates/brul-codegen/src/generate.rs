//! Single generation path shared by `build.rs`, the CLI and tests.

use std::fs;
use std::path::Path;

use brul_syntax::ast::Position;
use brul_syntax::error::Error;

use crate::commands::{self, Commands};
use crate::emit::{self, Module};
use crate::{error_at, parse_source};

pub fn generate(entry: &Path, root: &Path) -> Result<Module, Error> {
    let dir = root.join("src");
    let commands = if dir.is_dir() {
        commands::collect(&dir)?
    } else {
        Commands::new()
    };
    generate_with(entry, &commands)
}

pub fn generate_with(entry: &Path, commands: &Commands) -> Result<Module, Error> {
    if is_application(entry)? {
        emit::split_module_file_with_commands(entry, commands)
    } else {
        emit::split_component_file_with_commands(entry, commands)
    }
}

fn is_application(entry: &Path) -> Result<bool, Error> {
    let source = fs::read_to_string(entry).map_err(|error| {
        error_at(
            Position { line: 1, column: 1 },
            format!("cannot read '{}': {error}", entry.display()),
        )
    })?;
    Ok(parse_source(&source)?
        .declaration
        .is_some_and(|declaration| declaration.is_window()))
}
