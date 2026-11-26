use std::cell::RefCell;

use brul_script::Names;
use brul_syntax::error::Error;
use brul_syntax::markup::Declaration;
use brul_syntax::script::{Program, validate_build};
use brul_syntax::style::{EffectRule, Stylesheet};

use crate::discovery::Capabilities;
use crate::loader::Import;
use crate::source::Script;
use crate::{Definition, Registry, Resource};

use super::CodegenContext;
use super::EmitOutputs;
use super::ExprContext;
use super::StateIds;
use super::StructureNames;
use super::anchor::AnchorIds;
use super::classes::ClassNames;
use super::view::ViewSpec;
use super::{names, resources, script, state};

use super::module::{GeneratedFile, HEADER};
use super::module_windows::{
    definition_root, emit_tail, emit_view, prop_types, push_script, state_type,
};

pub(super) struct WindowView<'a> {
    pub(super) declaration: &'a Declaration,
    pub(super) stylesheet: &'a Stylesheet,
    pub(super) effects: &'a [EffectRule],
    pub(super) program: Option<&'a Program>,
    pub(super) script: Option<&'a Script>,
    pub(super) imports: &'a [Import],
    pub(super) resources: &'a [Resource],
    pub(super) base: String,
    pub(super) source: String,
}

pub(super) struct ComponentOutput {
    pub(super) file: GeneratedFile,
    pub(super) body: String,
    pub(super) functions: String,
}

pub(super) fn validate_components(
    definitions: &[Definition],
    commands: &[&str],
) -> Result<(), Error> {
    for definition in definitions {
        let Some(program) = &definition.program else {
            continue;
        };
        let mut external = names::declared(&definition.declaration.members);
        external.extend(names::declared_events(&definition.declaration.members));
        validate_build(program, &external, commands)?;
    }
    Ok(())
}

pub(super) struct ComponentSinks<'a> {
    pub(super) structures: &'a mut StructureNames,
    pub(super) scripts: Option<&'a mut Vec<GeneratedFile>>,
}

pub(super) struct ComponentSources<'a> {
    pub(super) registry: &'a Registry,
    pub(super) bases: &'a [String],
    pub(super) names: &'a Names<'a>,
    pub(super) capabilities: &'a Capabilities,
    pub(super) anchor_ids: &'a AnchorIds,
    pub(super) state_ids: &'a RefCell<StateIds>,
}

pub(super) fn emit_component(
    definition: &Definition,
    base: &str,
    sources: &ComponentSources<'_>,
    sinks: &mut ComponentSinks<'_>,
) -> Result<ComponentOutput, Error> {
    let ComponentSources {
        registry,
        bases,
        names,
        capabilities,
        anchor_ids,
        state_ids,
    } = sources;
    let root = definition_root(definition)?;
    let prefix = format!("{base}_");
    let name = format!("bc_{base}");
    let mut body = String::new();
    let mut functions = Vec::new();
    let mut dynamic_classes = false;
    let mut class_names = ClassNames::default();
    let component_state = state::State::new(
        definition.program.as_ref(),
        &state_type(&format!("{prefix}script")),
    )?
    .with_commands(&capabilities.commands);
    let component_prop_types = prop_types(&definition.declaration.members);
    body.push_str(HEADER);
    emit_view(
        &mut body,
        ViewSpec {
            name: &format!("{prefix}view"),
            source: &definition.file,
            root,
            handlers: &[],
            attaches: &[],
            attach_states: &[],
            kind: ViewKind::Component,
        },
        &CodegenContext {
            capabilities,
            registry,
            anchor_ids,
            script_name: &format!("{prefix}script"),
            imports: &definition.imports,
            resources: &definition.resources,
            base,
            resource_prefix: &prefix,
            stylesheet: &definition.stylesheet,
            effects: &definition.effects,
            names,
            state: &component_state,
            state_ids,
            prop_types: &component_prop_types,
            program: definition.program.as_ref(),
        },
        EmitOutputs {
            functions: &mut functions,
            bases,
            structures: &mut *sinks.structures,
            dynamic_classes: &mut dynamic_classes,
            classes: &mut class_names,
        },
    )?;
    emit_tail(
        &mut body,
        &prefix,
        &definition.stylesheet,
        &class_names,
        dynamic_classes,
    )?;
    body.push_str(&script::module(
        &format!("{prefix}script"),
        definition.program.as_ref(),
        &ExprContext::new(names, &component_state, &component_prop_types),
    )?);
    body.push_str(&resources::functions(&prefix, &definition.resources));
    let mut functions_block = String::new();
    append_functions(&mut functions_block, &functions);
    if let Some(scripts) = &mut sinks.scripts {
        push_script(
            scripts,
            format!("{name}.script"),
            definition.script.as_ref(),
        );
    }
    let mut file = String::from("use super::*;\n");
    file.push_str(&body);
    file.push_str(&functions_block);
    Ok(ComponentOutput {
        file: GeneratedFile { name, source: file },
        body,
        functions: functions_block,
    })
}

pub(super) fn append_functions(out: &mut String, functions: &[String]) {
    for function in functions {
        out.push('\n');
        out.push_str(function);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ViewKind {
    Window,
    Component,
}

impl ViewKind {
    pub(super) fn typed(self) -> bool {
        matches!(self, Self::Window)
    }
}
