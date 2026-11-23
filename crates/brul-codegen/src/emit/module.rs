use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use brul_case::{constant, identifier, pascal};
use brul_script::Names;
use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Declaration, Element, Member};
use brul_syntax::script::{Program, validate_build};
use brul_syntax::style::{EffectRule, Stylesheet};
use brul_syntax::types::Type;

use crate::commands;
use crate::discovery::Capabilities;
use crate::error_at;
use crate::loader::Import;
use crate::runtime::HandlerEvent;
use crate::source::Script;
use crate::{Definition, Registry, Resource};

use super::CodegenContext;
use super::Emit;
use super::EmitOutputs;
use super::Out;
use super::StructureNames;
use super::anchor::{self, AnchorIds};
use super::classes::ClassNames;
use super::{alloc, classes, events, names, resources, script, state, style, theme, view};

const HEADER: &str = "#[allow(unused_mut, unused_variables, clippy::let_and_return)]\n";

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

    let mut views = vec![WindowView {
        declaration,
        stylesheet: &parsed.stylesheet,
        effects: &parsed.effects,
        program: parsed.program.as_ref(),
        script: parsed.script.as_ref(),
        imports: &registry.imports,
        resources: &registry.resources,
        base: entry_base.clone(),
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
        let (config, window_handlers) = crate::arguments(view.declaration)?;
        icons.push(icon_resource(view.declaration, view.resources)?);
        let window_handlers = events::window(&window_handlers, view.program)?;
        let root = crate::root(view.declaration)?;
        let script_name = format!("{}_script", view.base);
        let prefix = format!("{}_", view.base);
        let view_state = state::State::new(view.program, &state_type(&script_name))?
            .with_commands(&capabilities.commands);
        let types = prop_types(&view.declaration.members);
        head.push('\n');
        head.push_str(HEADER);
        let mut functions = Vec::new();
        let mut dynamic_classes = false;
        let mut class_names = ClassNames::default();
        emit_view(
            &mut head,
            &format!("{}_view", view.base),
            root,
            &window_handlers,
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
            ViewKind::Window,
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
            &names,
            &view_state,
            &types,
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

struct WindowView<'a> {
    declaration: &'a Declaration,
    stylesheet: &'a Stylesheet,
    effects: &'a [EffectRule],
    program: Option<&'a Program>,
    script: Option<&'a Script>,
    imports: &'a [Import],
    resources: &'a [Resource],
    base: String,
}

struct ComponentOutput {
    file: GeneratedFile,
    body: String,
    functions: String,
}

fn validate_components(definitions: &[Definition], commands: &[&str]) -> Result<(), Error> {
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

struct ComponentSinks<'a> {
    structures: &'a mut StructureNames,
    scripts: Option<&'a mut Vec<GeneratedFile>>,
}

struct ComponentSources<'a> {
    registry: &'a Registry,
    bases: &'a [String],
    names: &'a Names<'a>,
    capabilities: &'a Capabilities,
    anchor_ids: &'a AnchorIds,
}

fn emit_component(
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
        &format!("{prefix}view"),
        root,
        &[],
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
        ViewKind::Component,
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
        names,
        &component_state,
        &component_prop_types,
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

fn append_functions(out: &mut String, functions: &[String]) {
    for function in functions {
        out.push('\n');
        out.push_str(function);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewKind {
    Window,
    Component,
}

impl ViewKind {
    fn typed(self) -> bool {
        matches!(self, Self::Window)
    }
}

fn emit_view<'a>(
    out: &mut String,
    name: &str,
    root: &Element,
    handlers: &[(HandlerEvent, String)],
    context: &'a CodegenContext<'a>,
    outputs: EmitOutputs<'a>,
    kind: ViewKind,
) -> Result<(), Error> {
    let slots = matches!(kind, ViewKind::Component);
    let mut emit = Emit::new(context, outputs, slots);
    view::view(out, name, root, handlers, &mut emit, kind.typed())
}

fn emit_tail(
    out: &mut String,
    prefix: &str,
    stylesheet: &Stylesheet,
    class_names: &ClassNames,
    dynamic: bool,
) -> Result<(), Error> {
    let mut out = Out::new(out);
    classes::statics(&mut out, prefix, stylesheet, class_names)?;
    if dynamic {
        classes::tables(&mut out, prefix, stylesheet);
    }
    style::stylesheet(&mut out, prefix, stylesheet);
    Ok(())
}

fn push_script(
    scripts: &mut Vec<GeneratedFile>,
    name: String,
    script: Option<&crate::source::Script>,
) {
    if let Some(script) = script {
        scripts.push(GeneratedFile {
            name,
            source: script.text.clone(),
        });
    }
}

fn module_sources(registry: &crate::Registry, entry: Option<&Path>) -> Vec<PathBuf> {
    let mut sources: Vec<PathBuf> = registry
        .definitions
        .iter()
        .filter_map(|definition| definition.origin.path().map(Path::to_path_buf))
        .collect();
    if let Some(entry) = entry {
        sources.push(fs::canonicalize(entry).unwrap_or_else(|_| entry.to_path_buf()));
    }
    sources
}

fn module_table(root: &mut String, files: &[GeneratedFile]) {
    for file in files {
        let _ = write!(
            root,
            "pub mod {name} {{
    include!(concat!(env!(\"OUT_DIR\"), \"/{name}.rs\"));
}}
pub use self::{name}::*;
",
            name = file.name
        );
    }
}

fn state_type(name: &str) -> String {
    format!("{}State", pascal(name))
}

fn prop_types(members: &[Member]) -> HashMap<String, Type> {
    members
        .iter()
        .filter_map(|member| match member {
            Member::Prop(prop) => Some((prop.name.clone(), prop.ty.clone())),
            _ => None,
        })
        .collect()
}

fn unique(name: &str, used: &mut HashSet<String>) -> String {
    let mut candidate = name.to_owned();
    let mut index = 1;
    while !used.insert(candidate.clone()) {
        candidate = format!("{name}{index}");
        index += 1;
    }
    candidate
}

fn definition_root(definition: &Definition) -> Result<&Element, Error> {
    let position = Position { line: 1, column: 1 };
    let mut roots = definition
        .declaration
        .members
        .iter()
        .filter_map(|member| match member {
            Member::Element(element) => Some(element),
            Member::Prop(_) | Member::Event(_) => None,
        });
    let Some(root) = roots.next() else {
        return Err(error_at(
            position,
            format!("component '{}' has no root element", definition.name),
        ));
    };
    if roots.next().is_some() {
        return Err(error_at(
            position,
            format!(
                "component '{}' has more than one root element",
                definition.name
            ),
        ));
    }
    Ok(root)
}

fn icon_resource<'a>(
    declaration: &'a Declaration,
    resources: &[Resource],
) -> Result<Option<&'a str>, Error> {
    let Some(value) = declaration.args().iter().find_map(|arg| match arg {
        Arg::Named { name, value } if name == "icon" => Some(value),
        _ => None,
    }) else {
        return Ok(None);
    };
    super::resource(value, resources).map(Some).ok_or_else(|| {
        error_at(
            value.position,
            "window 'icon' must name a declared resource",
        )
    })
}

fn effects_registry(out: &mut String, names: &BTreeSet<String>) {
    out.push_str(
        "pub fn __brul_effects() -> brul::effect::Effects {\n    brul::effect::Effects::new(&[\n",
    );
    for name in names {
        let _ = writeln!(out, "        crate::BRUL_EFFECT_{name},");
    }
    out.push_str("    ])\n}\n\n");
}

fn window_config(
    out: &mut String,
    base: &str,
    prefix: &str,
    config: &brul_window::WindowConfig,
    icon: Option<&str>,
) {
    let position = match config.position {
        Some(position) => format!(
            "Some(brul::WindowPosition {{ x: {:?}, y: {:?} }})",
            position.x, position.y
        ),
        None => String::from("None"),
    };
    let icon = match icon {
        Some(name) => format!("Some({prefix}resource_{name}().into())"),
        None => String::from("None"),
    };
    let _ = write!(
        out,
        "pub fn {base}_window() -> brul::WindowConfig {{
    brul::WindowConfig {{
        title: {:?}.to_owned(),
        width: {},
        height: {},
        frameless: {},
        transparent: {},
        visible: {},
        centered: {},
        scroll: {},
        position: {position},
        icon: {icon},
        effects: __brul_effects(),
        ..brul::WindowConfig::default()
    }}
}}
",
        config.title,
        config.width,
        config.height,
        config.frameless,
        config.transparent,
        config.visible,
        config.centered,
        config.scroll
    );
}

fn window_table(out: &mut String, views: &[WindowView<'_>]) {
    out.push_str("pub fn windows() -> Vec<brul::WindowDef<ThemeId>> {\n");
    alloc::registration(out);
    out.push_str("    vec![\n");
    for view in views {
        let base = &view.base;
        let _ = writeln!(
            out,
            "        brul::WindowDef {{ name: {:?}, config: {base}_window(), view: {base}_view }},",
            view.declaration.name
        );
    }
    out.push_str("    ]\n}\n");
}
