use std::cell::RefCell;

use brul_script::Names;
use brul_syntax::ast::{Expr, ExprKind};
use brul_syntax::script::Program;
use brul_syntax::style::{EffectRule, Stylesheet};
use brul_syntax::types::Type;

use crate::discovery::Capabilities;
use crate::loader::Import;
use crate::{Registry, Resource};

use super::PropTypes;
use super::anchor::AnchorIds;
use super::classes::ClassNames;
use super::scopes::ScopeInfo;
use super::state::State;
use super::state_ids::StateIds;
use super::structure::StructureNames;

pub(crate) struct CodegenContext<'a> {
    pub(super) capabilities: &'a Capabilities,
    pub(super) registry: &'a Registry,
    pub(super) anchor_ids: &'a AnchorIds,
    pub(super) script_name: &'a str,
    pub(super) imports: &'a [Import],
    pub(super) resources: &'a [Resource],
    pub(super) base: &'a str,
    pub(super) resource_prefix: &'a str,
    pub(super) stylesheet: &'a Stylesheet,
    pub(super) effects: &'a [EffectRule],
    pub(super) names: &'a Names<'a>,
    pub(super) state: &'a State,
    pub(super) state_ids: &'a RefCell<StateIds>,
    pub(super) prop_types: &'a PropTypes,
    pub(super) program: Option<&'a Program>,
}

pub(crate) struct EmitOutputs<'a> {
    pub(super) functions: &'a mut Vec<String>,
    pub(super) bases: &'a [String],
    pub(super) structures: &'a mut StructureNames,
    pub(super) dynamic_classes: &'a mut bool,
    pub(super) classes: &'a mut ClassNames,
}

pub(crate) struct Emit<'a> {
    pub(super) context: &'a CodegenContext<'a>,
    pub(super) scope_defs: Vec<ScopeInfo>,
    pub(super) functions: &'a mut Vec<String>,
    pub(super) bases: &'a [String],
    pub(super) structures: &'a mut StructureNames,
    pub(super) dynamic_classes: &'a mut bool,
    pub(super) classes: &'a mut ClassNames,
    pub(super) slots: bool,
}

impl<'a> Emit<'a> {
    pub(super) fn new(
        context: &'a CodegenContext<'a>,
        outputs: EmitOutputs<'a>,
        slots: bool,
    ) -> Self {
        Self {
            context,
            scope_defs: Vec::new(),
            functions: outputs.functions,
            bases: outputs.bases,
            structures: outputs.structures,
            dynamic_classes: outputs.dynamic_classes,
            classes: outputs.classes,
            slots,
        }
    }

    pub(super) fn prop_type(&self, name: &str) -> Option<&Type> {
        self.context.prop_types.get(name)
    }

    pub(super) fn item_type(&self, collection: &Expr) -> Type {
        let ExprKind::Ident(name) = &collection.kind else {
            return Type::Object;
        };
        let ty = self
            .prop_type(name)
            .or_else(|| self.context.state.field_type(name));
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
        self.context
            .stylesheet
            .themes
            .first()
            .map(|theme| theme.name.as_str())
    }
}
