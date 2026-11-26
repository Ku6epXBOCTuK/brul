use std::cell::RefCell;
use std::collections::{BTreeSet, HashSet};

use std::fs;
use std::path::{Path, PathBuf};

use brul_case::{constant, identifier, pascal};

use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::script::validate_build;

use crate::commands;
use crate::discovery::Capabilities;
use crate::error_at;

use super::CodegenContext;
use super::ExprContext;

use super::EmitOutputs;

use super::StateIds;
use super::StateKey;
use super::StructureNames;
use super::anchor;
use super::classes::ClassNames;
use super::view::ViewSpec;
use super::{alloc, events, names, resources, script, state, theme};

pub(super) use super::module_component::ViewKind;
use super::module_component::{
    ComponentSinks, ComponentSources, WindowView, append_functions, emit_component,
    validate_components,
};
use super::module_windows::{
    effects_registry, emit_tail, emit_view, icon_resource, module_sources, module_table,
    prop_types, push_script, state_type, unique, window_config, window_table,
};

pub(super) const HEADER: &str = "#[allow(unused_mut, unused_variables, clippy::let_and_return)]\n";

pub struct GeneratedFile {
    pub name: String,
    pub source: String,
}

#[derive(Debug)]
pub struct ResourceFile {
    pub name: String,
    pub path: PathBuf,
}

pub struct Module {
    pub root: String,
    pub files: Vec<GeneratedFile>,
    pub scripts: Vec<GeneratedFile>,
    pub sources: Vec<PathBuf>,
    pub resources: Vec<ResourceFile>,
    flat: String,
}

impl Module {
    pub fn flat(&self) -> &str {
        &self.flat
    }
}

pub fn module(source: &str) -> Result<String, Error> {
    module_with_capabilities(source, &Capabilities::default())
}

pub fn module_with_capabilities(
    source: &str,
    capabilities: &Capabilities,
) -> Result<String, Error> {
    let module = module_at(source, None, false, None, capabilities)?;
    Ok(module.flat)
}

pub fn module_file(path: &Path) -> Result<String, Error> {
    module_file_with_capabilities(path, &Capabilities::default())
}

pub fn module_file_with_capabilities(
    path: &Path,
    capabilities: &Capabilities,
) -> Result<String, Error> {
    let module = split_module_file_with_capabilities(path, capabilities)?;
    Ok(module.flat)
}

pub fn split_module_file(path: &Path) -> Result<Module, Error> {
    split_module_file_with_capabilities(path, &Capabilities::default())
}

pub fn split_module_file_with_capabilities(
    path: &Path,
    capabilities: &Capabilities,
) -> Result<Module, Error> {
    let position = Position { line: 1, column: 1 };
    let source = fs::read_to_string(path).map_err(|error| {
        error_at(
            position,
            format!("cannot read '{}': {error}", path.display()),
        )
    })?;
    module_at(&source, path.parent(), true, Some(path), capabilities)
}

pub fn component_file(path: &Path) -> Result<String, Error> {
    component_file_with_capabilities(path, &Capabilities::default())
}

pub fn component_file_with_capabilities(
    path: &Path,
    capabilities: &Capabilities,
) -> Result<String, Error> {
    Ok(split_component_file_with_capabilities(path, capabilities)?.flat)
}

pub fn split_component_file_with_capabilities(
    path: &Path,
    capabilities: &Capabilities,
) -> Result<Module, Error> {
    let (mut registry, entry) = crate::load_entry(path)?;
    let command_names = commands::names(&capabilities.commands);
    validate_components(&registry.definitions, &command_names)?;
    let mut source = crate::Source::empty();
    crate::builtin::compose(&mut source, &mut registry, &capabilities.specs)?;
    let names = names::table(&source, &registry.definitions, &capabilities.specs);
    let anchor_table = anchor::collect(&source, &registry.definitions)?;
    let mut used = HashSet::new();
    let bases: Vec<String> = registry
        .definitions
        .iter()
        .map(|definition| unique(&identifier(&definition.name), &mut used))
        .collect();
    let mut structures = StructureNames::default();
    let state_ids = RefCell::new(StateIds::default());
    let mut sinks = ComponentSinks {
        structures: &mut structures,
        scripts: None,
    };
    let emitted = emit_component(
        &registry.definitions[entry],
        &bases[entry],
        &ComponentSources {
            registry: &registry,
            bases: &bases,
            names: &names,
            capabilities,
            anchor_ids: &anchor_table,
            state_ids: &state_ids,
        },
        &mut sinks,
    )?;
    let source = emitted.file.source;
    Ok(Module {
        root: source.clone(),
        files: Vec::new(),
        scripts: Vec::new(),
        sources: vec![fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())],
        resources: Vec::new(),
        flat: source,
    })
}

fn module_at(
    source: &str,
    base: Option<&Path>,
    _split: bool,
    entry: Option<&Path>,
    capabilities: &Capabilities,
) -> Result<Module, Error> {
    let mut parsed = crate::parse_source(source)?;
    let mut registry = crate::resolve(&parsed.uses, base, &parsed.stylesheet.themes)?;
    crate::apply_overrides(&mut registry, &parsed.overrides, source)?;
    crate::builtin::compose(&mut parsed, &mut registry, &capabilities.specs)?;
    let entry_source = entry
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "entry.brul".to_owned());
    let declaration = parsed
        .declaration
        .as_ref()
        .ok_or_else(|| error_at(Position { line: 1, column: 1 }, "missing declaration"))?;
    if !declaration.is_window() {
        return Err(error_at(
            declaration.position,
            "entry declaration must be a window",
        ));
    }
    let names = names::table(&parsed, &registry.definitions, &capabilities.specs);
    let anchor_table = anchor::collect(&parsed, &registry.definitions)?;
    let root_props = names::declared(&declaration.members);
    let mut root_external = names::declared_events(&declaration.members);
    let command_names = commands::names(&capabilities.commands);
    let effect_names: BTreeSet<String> = parsed
        .effects
        .iter()
        .chain(
            registry
                .definitions
                .iter()
                .flat_map(|definition| definition.effects.iter()),
        )
        .map(|rule| constant(&rule.name))
        .collect();
    if let Some(program) = &parsed.program {
        root_external.extend(root_props.iter().copied());
        validate_build(program, &root_external, &command_names)?;
    }
    validate_components(&registry.definitions, &command_names)?;

    let mut used = HashSet::new();
    let entry_base = unique(&identifier(&declaration.name), &mut used);
    let entry_script_name = format!("{entry_base}_script");
    let definition_bases: Vec<String> = registry
        .definitions
        .iter()
        .map(|definition| unique(&identifier(&definition.name), &mut used))
        .collect();

    let state_ids = RefCell::new(StateIds::default());
    let mut views = vec![WindowView {
        declaration,
        stylesheet: &parsed.stylesheet,
        effects: &parsed.effects,
        program: parsed.program.as_ref(),
        script: parsed.script.as_ref(),
        imports: &registry.imports,
        resources: &registry.resources,
        base: entry_base.clone(),
        source: entry_source,
    }];
    for (index, definition) in registry.definitions.iter().enumerate() {
        if definition.is_window() {
            views.push(WindowView {
                declaration: &definition.declaration,
                stylesheet: &definition.stylesheet,
                effects: &definition.effects,
                program: definition.program.as_ref(),
                script: definition.script.as_ref(),
                imports: &definition.imports,
                resources: &definition.resources,
                base: definition_bases[index].clone(),
                source: definition.file.clone(),
            });
        }
    }
    let mut window_names = HashSet::new();
    for view in &views {
        if !window_names.insert(view.declaration.name.as_str()) {
            return Err(error_at(
                view.declaration.position,
                format!("duplicate window '{}'", view.declaration.name),
            ));
        }
    }

    let mut structures = StructureNames::default();
    let mut head = String::new();
    let mut head_functions = String::new();
    let mut scripts = Vec::new();
    let mut configs = Vec::new();
    let mut icons = Vec::new();

    alloc::include(&mut head);
    head.push_str(HEADER);
    theme::id(&mut head, &parsed.stylesheet)?;

    for (index, view) in views.iter().enumerate() {
        let window_args = crate::arguments(view.declaration)?;
        let config = window_args.config;
        icons.push(icon_resource(view.declaration, view.resources)?);
        let window_handlers = events::window(&window_args.handlers, view.program)?;
        let root = crate::root(view.declaration)?;
        let script_name = format!("{}_script", view.base);
        let prefix = format!("{}_", view.base);
        let view_state = state::State::new(view.program, &state_type(&script_name))?
            .with_commands(&capabilities.commands);
        let attach_states = window_args
            .attaches
            .iter()
            .map(|attach| {
                let path = format!("attach:{}", attach.name);
                state_ids
                    .borrow_mut()
                    .register(&view.base, &path)
                    .map_err(|error| error_at(view.declaration.position, error))
            })
            .collect::<Result<Vec<StateKey>, Error>>()?;
        let types = prop_types(&view.declaration.members);
        head.push('\n');
        head.push_str(HEADER);
        let mut functions = Vec::new();
        let mut dynamic_classes = false;
        let mut class_names = ClassNames::default();
        emit_view(
            &mut head,
            ViewSpec {
                name: &format!("{}_view", view.base),
                source: &view.source,
                root,
                handlers: &window_handlers,
                attaches: &window_args.attaches,
                attach_states: &attach_states,
                kind: ViewKind::Window,
            },
            &CodegenContext {
                capabilities,
                registry: &registry,
                anchor_ids: &anchor_table,
                script_name: &script_name,
                imports: view.imports,
                resources: view.resources,
                base: &view.base,
                resource_prefix: &prefix,
                stylesheet: view.stylesheet,
                effects: view.effects,
                names: &names,
                state: &view_state,
                state_ids: &state_ids,
                prop_types: &types,
                program: view.program,
            },
            EmitOutputs {
                functions: &mut functions,
                bases: &definition_bases,
                structures: &mut structures,
                dynamic_classes: &mut dynamic_classes,
                classes: &mut class_names,
            },
        )?;
        append_functions(&mut head_functions, &functions);
        emit_tail(
            &mut head,
            &prefix,
            view.stylesheet,
            &class_names,
            dynamic_classes,
        )?;
        head.push_str(&resources::functions(&prefix, view.resources));
        head.push_str(&script::module(
            &script_name,
            view.program,
            &ExprContext::new(&names, &view_state, &types),
        )?);
        head.push_str(&script::handle(&script_name, view.program));
        if index == 0 && view.program.is_some() {
            head.push_str(&format!(
                "\n#[allow(dead_code)]\npub type Script = {};\n",
                pascal(&script_name)
            ));
        }
        push_script(&mut scripts, format!("{}.script", view.base), view.script);
        configs.push(config);
    }

    let mut files = Vec::new();
    let mut component_bodies = Vec::new();
    let mut component_functions = String::new();
    for (index, definition) in registry.definitions.iter().enumerate() {
        if definition.is_window() {
            continue;
        }
        let mut sinks = ComponentSinks {
            structures: &mut structures,
            scripts: Some(&mut scripts),
        };
        let emitted = emit_component(
            definition,
            &definition_bases[index],
            &ComponentSources {
                registry: &registry,
                bases: &definition_bases,
                names: &names,
                capabilities,
                anchor_ids: &anchor_table,
                state_ids: &state_ids,
            },
            &mut sinks,
        )?;
        files.push(emitted.file);
        component_bodies.push(emitted.body);
        component_functions.push_str(&emitted.functions);
    }

    let mut tail = String::new();
    effects_registry(&mut tail, &effect_names);
    for (index, config) in configs.iter().enumerate() {
        let prefix = format!("{}_", views[index].base);
        window_config(&mut tail, &views[index].base, &prefix, config, icons[index]);
    }
    tail.push('\n');
    window_table(&mut tail, &views);
    tail.push_str(&format!(
        "\npub fn script() -> Option<brul::Script> {{\n    {entry_script_name}()\n}}\n"
    ));
    tail.push_str(&names::output(&names));

    let mut root = head.clone();
    root.push('\n');
    module_table(&mut root, &files);
    root.push_str(&head_functions);
    root.push('\n');
    root.push_str(&tail);

    let mut flat = head;
    for body in &component_bodies {
        flat.push('\n');
        flat.push_str(body);
    }
    flat.push_str(&head_functions);
    flat.push_str(&component_functions);
    flat.push('\n');
    flat.push_str(&tail);

    let mut resource_files = Vec::new();
    for view in &views {
        let prefix = format!("{}_", view.base);
        for resource in view.resources {
            resource_files.push(ResourceFile {
                name: resources::function_name(&prefix, &resource.name),
                path: resource.path.clone(),
            });
        }
    }
    for (index, definition) in registry.definitions.iter().enumerate() {
        if definition.is_window() {
            continue;
        }
        let prefix = format!("{}_", definition_bases[index]);
        for resource in &definition.resources {
            resource_files.push(ResourceFile {
                name: resources::function_name(&prefix, &resource.name),
                path: resource.path.clone(),
            });
        }
    }

    Ok(Module {
        root,
        files,
        scripts,
        sources: module_sources(&registry, entry),
        resources: resource_files,
        flat,
    })
}
