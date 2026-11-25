use std::fs;
use std::path::Path;

use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use syn::Item;

use crate::capability::CapabilityTable;
use crate::commands::Commands;
use crate::error_at;

#[derive(Clone, Debug)]
pub struct Capabilities {
    pub commands: Commands,
    pub specs: CapabilityTable,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            commands: Commands::new(),
            specs: crate::capability::builtins(),
        }
    }
}

impl Capabilities {
    pub fn discover(root: &Path, entry: &Path) -> Result<Self, Error> {
        let dir = root.join("src");
        let commands = if dir.is_dir() {
            crate::commands::collect(&dir)?
        } else {
            Commands::new()
        };
        Self::discover_with_commands(entry, commands)
    }

    pub fn discover_with_commands(entry: &Path, commands: Commands) -> Result<Self, Error> {
        let dir = entry.parent().unwrap_or(Path::new(".")).join("src");
        let mut specs = crate::capability::builtins();
        if dir.is_dir() {
            let sources = crate::commands::rust_sources(&dir)
                .map_err(|error| at(format!("cannot read '{}': {error}", dir.display())))?;
            for path in sources {
                let source = fs::read_to_string(&path)
                    .map_err(|error| at(format!("cannot read '{}': {error}", path.display())))?;
                let file = syn::parse_file(&source).map_err(|error| at(error.to_string()))?;
                for item in &file.items {
                    let Item::Fn(function) = item else {
                        continue;
                    };
                    if brul_capability::is_widget(function) || brul_capability::is_attach(function)
                    {
                        crate::capability::collect_into(&mut specs, function)?;
                    }
                }
            }
        }
        Ok(Self { commands, specs })
    }
}

fn at(message: impl Into<String>) -> Error {
    error_at(Position { line: 1, column: 1 }, message)
}
