use std::collections::HashMap;

use brul_script::Names;
use brul_syntax::ast::{Expr, ExprKind};
use brul_syntax::script::Program;
use brul_syntax::style::Stylesheet;
use brul_syntax::types::Type;

use crate::{Import, Registry, Resource};

pub(super) use structure::StructureNames;

mod alloc;
mod builtin;
mod builtins;
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
mod scopes;
mod script;
mod state;
mod statement;
mod structure;
mod style;
mod theme;
mod view;

pub(super) use scopes::ScopeInfo;
pub(super) type PropTypes = HashMap<String, Type>;

pub use state::State;

pub use alloc::allocations;
pub use expression::operation;
pub use module::{
    GeneratedFile, Module, ResourceFile, component_file, component_file_with_commands, module,
    module_file, module_file_with_commands, module_with_commands,
    split_component_file_with_commands, split_module_file, split_module_file_with_commands,
};
pub(super) use out::Out;

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

pub(super) struct Emit<'a> {
    script_name: &'a str,
    registry: &'a Registry,
    imports: &'a [Import],
    resources: &'a [Resource],
    base: &'a str,
    resource_prefix: &'a str,
    stylesheet: &'a Stylesheet,
    names: &'a Names<'a>,
    state: &'a State,
    prop_types: &'a HashMap<String, Type>,
    program: Option<&'a Program>,
    scope_defs: Vec<ScopeInfo>,
    functions: &'a mut Vec<String>,
    bases: &'a [String],
    structures: &'a mut StructureNames,
    dynamic_classes: &'a mut bool,
    classes: &'a mut classes::ClassNames,
    slots: bool,
}

struct EmitSources<'a> {
    script_name: &'a str,
    registry: &'a Registry,
    imports: &'a [Import],
    resources: &'a [Resource],
    stylesheet: &'a Stylesheet,
    names: &'a Names<'a>,
    state: &'a State,
    prop_types: &'a HashMap<String, Type>,
    program: Option<&'a Program>,
    base: &'a str,
    resource_prefix: &'a str,
}

struct EmitOutputs<'a> {
    functions: &'a mut Vec<String>,
    bases: &'a [String],
    structures: &'a mut StructureNames,
    dynamic_classes: &'a mut bool,
    classes: &'a mut classes::ClassNames,
}

impl<'a> Emit<'a> {
    fn window(sources: EmitSources<'a>, outputs: EmitOutputs<'a>) -> Self {
        Self::new(sources, outputs, false)
    }

    fn component(sources: EmitSources<'a>, outputs: EmitOutputs<'a>) -> Self {
        Self::new(sources, outputs, true)
    }

    fn new(sources: EmitSources<'a>, outputs: EmitOutputs<'a>, slots: bool) -> Self {
        Self {
            script_name: sources.script_name,
            registry: sources.registry,
            imports: sources.imports,
            resources: sources.resources,
            base: sources.base,
            resource_prefix: sources.resource_prefix,
            stylesheet: sources.stylesheet,
            names: sources.names,
            state: sources.state,
            prop_types: sources.prop_types,
            scope_defs: Vec::new(),
            program: sources.program,
            functions: outputs.functions,
            bases: outputs.bases,
            structures: outputs.structures,
            dynamic_classes: outputs.dynamic_classes,
            classes: outputs.classes,
            slots,
        }
    }
}

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

impl<'a> Emit<'a> {
    pub(super) fn prop_type(&self, name: &str) -> Option<&Type> {
        self.prop_types.get(name)
    }

    pub(super) fn item_type(&self, collection: &Expr) -> Type {
        let ExprKind::Ident(name) = &collection.kind else {
            return Type::Object;
        };
        let ty = self.prop_type(name).or_else(|| self.state.field_type(name));
        match ty {
            Some(Type::Array(inner)) => (**inner).clone(),
            _ => Type::Object,
        }
    }

    pub(super) fn scope_def(&self, index: usize) -> Option<&ScopeInfo> {
        self.scope_defs.get(index)
    }

    pub(super) fn add_scope(&mut self, info: ScopeInfo) -> usize {
        self.scope_defs.push(info);
        self.scope_defs.len() - 1
    }

    pub(super) fn default_theme(&self) -> Option<&'a str> {
        self.stylesheet
            .themes
            .first()
            .map(|theme| theme.name.as_str())
    }
}
