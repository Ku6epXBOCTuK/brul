use std::collections::HashMap;

use brul_script::Names;
use brul_syntax::ast::{Expr, ExprKind};
use brul_syntax::types::Type;

use crate::Resource;

pub(super) use structure::StructureNames;

mod alloc;
mod anchor;
mod builtins;
mod class;
mod classes;
mod component;
mod context;
mod element;
mod events;
mod expression;
mod module;
mod names;
mod out;
mod resources;
mod scopes;
mod script;
mod state;
mod state_ids;
mod statement;
mod structure;
mod style;
mod theme;
mod view;

pub(super) use scopes::ScopeInfo;
pub(super) use state_ids::{StateIds, StateKey, child};
pub(super) type PropTypes = HashMap<String, Type>;

pub use state::State;

pub use alloc::allocations;
pub(super) use context::{CodegenContext, Emit, EmitOutputs};
pub use expression::operation;
pub use module::{
    GeneratedFile, Module, ResourceFile, component_file, component_file_with_capabilities, module,
    module_file, module_file_with_capabilities, module_with_capabilities,
    split_component_file_with_capabilities, split_module_file, split_module_file_with_capabilities,
};
pub(super) use out::Out;

pub(super) fn rust_string(text: &str) -> String {
    let mut out = String::from("\"");
    for character in text.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if character.is_ascii_graphic() || character == ' ' => out.push(character),
            character => out.push_str(&format!("\\u{{{:x}}}", character as u32)),
        }
    }
    out.push('"');
    out
}

pub(super) fn resource<'a>(expression: &'a Expr, resources: &[Resource]) -> Option<&'a str> {
    let ExprKind::Ident(name) = &expression.kind else {
        return None;
    };
    resources
        .iter()
        .any(|resource| &resource.name == name)
        .then_some(name.as_str())
}

pub fn identifiers(expression: &brul_syntax::ast::Expr) -> Names<'_> {
    let mut names = Names::new();
    names::expression(expression, &mut names);
    names
}

pub(super) use element::{contains_slot, items, items_spliced};

#[derive(Clone, Copy)]
pub(super) struct Frame<'a> {
    theme: Option<&'a str>,
    direction: &'a str,
    row_gap: u32,
    column_gap: u32,
    pub(super) scope: Option<usize>,
}

impl<'a> Frame<'a> {
    pub(super) fn root(theme: Option<&'a str>) -> Self {
        Self {
            theme,
            direction: "Column",
            row_gap: 0,
            column_gap: 0,
            scope: None,
        }
    }

    pub(super) fn inside(self, direction: &'a str, row_gap: u32, column_gap: u32) -> Self {
        Self {
            direction,
            row_gap,
            column_gap,
            ..self
        }
    }

    pub(super) fn theme(&self) -> Option<&'a str> {
        self.theme
    }

    pub(super) fn direction(&self) -> &'a str {
        self.direction
    }

    pub(super) fn row_gap(&self) -> u32 {
        self.row_gap
    }

    pub(super) fn column_gap(&self) -> u32 {
        self.column_gap
    }
}
