use brul_syntax::script::Program;
use brul_syntax::style::Stylesheet;

use crate::{Import, Registry};

mod component;
mod element;
mod events;
mod expression;
mod module;
mod names;
mod script;
mod structure;
mod style;
mod view;

pub use expression::{expression, operation};
pub use module::{module, module_file};

pub(super) use element::items;

pub(super) struct Emit<'a> {
    registry: &'a Registry,
    imports: &'a [Import],
    stylesheet: &'a Stylesheet,
    program: Option<&'a Program>,
    functions: &'a mut Vec<String>,
    next_structure: &'a mut usize,
}

#[derive(Clone, Copy)]
pub(super) struct Frame<'a> {
    theme: Option<&'a str>,
    direction: &'a str,
    spacing: u32,
    depth: usize,
}

impl<'a> Frame<'a> {
    pub(super) fn root(theme: Option<&'a str>, depth: usize) -> Self {
        Self {
            theme,
            direction: "Column",
            spacing: 0,
            depth,
        }
    }

    pub(super) fn deeper(self, levels: usize) -> Self {
        Self {
            depth: self.depth + levels,
            ..self
        }
    }

    pub(super) fn inside(self, direction: &'a str, spacing: u32) -> Self {
        Self {
            direction,
            spacing,
            ..self
        }
    }

    pub(super) fn theme(&self) -> Option<&'a str> {
        self.theme
    }

    pub(super) fn direction(&self) -> &'a str {
        self.direction
    }

    pub(super) fn spacing(&self) -> u32 {
        self.spacing
    }

    pub(super) fn depth(&self) -> usize {
        self.depth
    }

    pub(super) fn theme_expression(&self) -> String {
        match self.theme {
            Some(name) => format!("Some({name:?})"),
            None => "None".to_owned(),
        }
    }
}
