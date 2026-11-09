use brul_syntax::script::Program;
use brul_syntax::style::Stylesheet;

use crate::{Import, Registry};

mod component;
mod element;
mod events;
mod expression;
mod module;
mod script;
mod structure;
mod style;
mod view;

pub use expression::expression;
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
