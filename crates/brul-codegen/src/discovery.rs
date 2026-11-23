use std::fs;
use std::path::Path;

use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use syn::Item;

use crate::attachments::Attachments;
use crate::commands::Commands;
use crate::error_at;
use crate::widgets::Widgets;

#[derive(Clone, Debug, Default)]
pub struct Capabilities {
    pub commands: Commands,
    pub widgets: Widgets,
    pub attachments: Attachments,
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
        let mut widgets = crate::widgets::builtins();
        let mut attachments = Attachments::new();
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
                    if brul_capability::is_widget(function) {
                        crate::widgets::collect_into(&mut widgets, function)?;
                    }
                    if brul_capability::is_attach(function) {
                        crate::attachments::collect_into(&mut attachments, function)?;
                    }
                }
            }
        }
        Ok(Self {
            commands,
            widgets,
            attachments,
        })
    }
}

fn at(message: impl Into<String>) -> Error {
    error_at(Position { line: 1, column: 1 }, message)
}
