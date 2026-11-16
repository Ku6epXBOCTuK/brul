use brul_script::Names;
use brul_syntax::script::Program;
use brul_syntax::style::Stylesheet;

use crate::{Import, Registry, Resource};

pub(super) use structure::StructureNames;

mod class;
mod classes;
mod component;
mod element;
mod events;
mod expression;
mod module;
mod names;
mod out;
mod resources;
mod script;
mod structure;
mod style;
mod theme;
mod view;

pub use expression::{expression, operation};
pub use module::{GeneratedFile, Module, module, module_file, split_module_file};
pub(super) use out::Out;

pub fn identifiers(expression: &brul_syntax::ast::Expr) -> Names<'_> {
    let mut names = Names::new();
    names::expression(expression, &mut names);
    names
}

pub(super) use element::{contains_slot, items, items_spliced};

pub(super) struct Emit<'a> {
    registry: &'a Registry,
    imports: &'a [Import],
    resources: &'a [Resource],
    base: &'a str,
    resource_prefix: &'a str,
    stylesheet: &'a Stylesheet,
    names: &'a Names<'a>,
    program: Option<&'a Program>,
    functions: &'a mut Vec<String>,
    bases: &'a [String],
    structures: &'a mut StructureNames,
    dynamic_classes: &'a mut bool,
    classes: &'a mut classes::ClassNames,
    slots: bool,
}

#[derive(Clone, Copy)]
pub(super) struct Frame<'a> {
    theme: Option<&'a str>,
    direction: &'a str,
    gap: u32,
}

impl<'a> Frame<'a> {
    pub(super) fn root(theme: Option<&'a str>) -> Self {
        Self {
            theme,
            direction: "Column",
            gap: 0,
        }
    }

    pub(super) fn inside(self, direction: &'a str, gap: u32) -> Self {
        Self {
            direction,
            gap,
            ..self
        }
    }

    pub(super) fn theme(&self) -> Option<&'a str> {
        self.theme
    }

    pub(super) fn direction(&self) -> &'a str {
        self.direction
    }

    pub(super) fn gap(&self) -> u32 {
        self.gap
    }
}

impl<'a> Emit<'a> {
    pub(super) fn default_theme(&self) -> Option<&'a str> {
        self.stylesheet
            .themes
            .first()
            .map(|theme| theme.name.as_str())
    }
}
