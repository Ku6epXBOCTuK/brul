use brul_script::Names;
use brul_syntax::script::Program;
use brul_syntax::style::Stylesheet;

use crate::{Import, Registry, Resource};

use palette::Palette;

mod classes;
mod component;
mod element;
mod events;
mod expression;
mod module;
mod names;
mod palette;
mod resources;
mod script;
mod structure;
mod style;
mod view;

pub use expression::{expression, operation};
pub use module::{GeneratedFile, Module, module, module_file, split_module_file};

pub fn identifiers(expression: &brul_syntax::ast::Expr) -> Names<'_> {
    let mut names = Names::new();
    names::expression(expression, &mut names);
    names
}

pub(super) use element::items;

pub(super) struct Emit<'a> {
    registry: &'a Registry,
    imports: &'a [Import],
    resources: &'a [Resource],
    resource_prefix: &'a str,
    stylesheet: &'a Stylesheet,
    names: &'a Names<'a>,
    program: Option<&'a Program>,
    functions: &'a mut Vec<String>,
    next_structure: &'a mut usize,
    dynamic_classes: &'a mut bool,
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
}

impl<'a> Emit<'a> {
    pub(super) fn palette(&self, theme: Option<&str>) -> Palette<'a> {
        Palette::new(self.stylesheet, theme)
    }

    pub(super) fn default_theme(&self) -> Option<&'a str> {
        self.stylesheet
            .themes
            .first()
            .map(|theme| theme.name.as_str())
    }
}
