//! Единый путь генерации: общий для `build.rs`, CLI и тестов.

use std::path::Path;

use brul_syntax::error::Error;

use crate::commands::{self, Commands};
use crate::emit::{self, Module};

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
    match emit::split_module_file_with_commands(entry, commands) {
        Ok(module) => Ok(module),
        Err(error) if error.message() == "missing window declaration" => {
            emit::split_component_file_with_commands(entry, commands)
        }
        Err(error) => Err(error),
    }
}
