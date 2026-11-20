use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use brul_case::{identifier, pascal};
use brul_script::Names;
use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Element, Member, Window};
use brul_syntax::script::validate_build;
use brul_syntax::types::Type;

use crate::commands::{self, Commands};
use crate::error_at;
use crate::kit;
use crate::{Definition, Registry, Resource};

use super::Emit;
use super::EmitOutputs;
use super::EmitSources;
use super::Out;
use super::StructureNames;
use super::classes::ClassNames;
use super::{alloc, classes, events, names, resources, script, state, style, theme, view};

const HEADER: &str = "#[allow(unused_mut, unused_variables, clippy::let_and_return)]\n";
const ROOT_SCRIPT: &str = "view.script";

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
    module_with_commands(source, &Commands::new())
}

pub fn module_with_commands(source: &str, commands: &Commands) -> Result<String, Error> {
    let module = module_at(source, None, false, None, commands)?;
    Ok(module.flat)
}

pub fn module_file(path: &Path) -> Result<String, Error> {
    let module = split_module_file(path)?;
    Ok(module.flat)
}

pub fn module_file_with_commands(path: &Path, commands: &Commands) -> Result<String, Error> {
    let module = split_module_file_with_commands(path, commands)?;
    Ok(module.flat)
}

pub fn split_module_file(path: &Path) -> Result<Module, Error> {
    split_module_file_with_commands(path, &Commands::new())
}

pub fn split_module_file_with_commands(path: &Path, commands: &Commands) -> Result<Module, Error> {
    let position = Position { line: 1, column: 1 };
    let source = fs::read_to_string(path).map_err(|error| {
        error_at(
            position,
            format!("cannot read '{}': {error}", path.display()),
        )
    })?;
    module_at(&source, path.parent(), true, Some(path), commands)
}

pub fn component_file(path: &Path) -> Result<String, Error> {
    component_file_with_commands(path, &Commands::new())
}

pub fn component_file_with_commands(path: &Path, commands: &Commands) -> Result<String, Error> {
    Ok(split_component_file_with_commands(path, commands)?.flat)
}

pub fn split_component_file_with_commands(
    path: &Path,
    commands: &Commands,
) -> Result<Module, Error> {
    let (registry, entry) = crate::load_entry(path)?;
    let command_names = commands::names(commands);
    validate_components(&registry.definitions, &command_names)?;
    let source = crate::Source::empty();
    let names = names::table(&source, &registry.definitions);
    let mut used = HashSet::new();
    let bases: Vec<String> = registry
        .definitions
        .iter()
        .map(|definition| unique(&identifier(&stem(&definition.path)), &mut used))
        .collect();
    let mut structures = StructureNames::default();
    let mut sinks = ComponentSinks {
        structures: &mut structures,
        scripts: None,
    };
    let emitted = emit_component(
        &registry.definitions[entry],
        &bases[entry],
        &registry,
        &bases,
        &names,
        commands,
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
    commands: &Commands,
) -> Result<Module, Error> {
    let parsed = crate::parse_source(source)?;
    let mut registry = crate::resolve(&parsed.uses, base, &parsed.stylesheet.themes)?;
    crate::apply_overrides(&mut registry, &parsed.overrides, source)?;
    let names = names::table(&parsed, &registry.definitions);
    let mut root_props = Vec::new();
    let mut root_prop_types = HashMap::new();
    let mut root_external = Vec::new();
    for window in &parsed.windows {
        root_props.extend(names::declared(&window.members));
        root_prop_types.extend(prop_types(&window.members));
        root_external.extend(names::declared_events(&window.members));
    }
    let command_names = commands::names(commands);
    if let Some(program) = &parsed.program {
        root_external.extend(root_props.iter().copied());
        validate_build(program, &root_external, &command_names)?;
    }
    validate_components(&registry.definitions, &command_names)?;
    let root_state =
        state::State::new(parsed.program.as_ref(), &state_type("script"))?.with_commands(commands);
    let mut used = HashSet::new();
    let window_bases: Vec<String> = parsed
        .windows
        .iter()
        .map(|window| unique(&identifier(&window.name), &mut used))
        .collect();
    let component_bases: Vec<String> = registry
        .definitions
        .iter()
        .map(|definition| {
            let stem = stem(&definition.path);
            unique(&identifier(&stem), &mut used)
        })
        .collect();
    let mut structures = StructureNames::default();
    let mut head = String::new();
    let mut head_functions = String::new();
    let mut configs = Vec::new();
    let mut icons = Vec::new();

    alloc::include(&mut head);
    head.push_str(HEADER);
    let mut dynamic_classes = false;
    let mut class_names = ClassNames::default();
    for (index, window) in parsed.windows.iter().enumerate() {
        let (config, window_handlers) = crate::arguments(window)?;
        icons.push(icon_resource(window, &registry.resources)?);
        let window_handlers = events::window(&window_handlers, parsed.program.as_ref())?;
        let root = crate::root(window)?;
        if index > 0 {
            head.push('\n');
            head.push_str(HEADER);
        }
        let mut functions = Vec::new();
        view::view(
            &mut head,
            &format!("{}_view", window_bases[index]),
            root,
            &window_handlers,
            &mut Emit::window(
                EmitSources {
                    script_name: "script",
                    registry: &registry,
                    imports: &registry.imports,
                    resources: &registry.resources,
                    stylesheet: &parsed.stylesheet,
                    names: &names,
                    state: &root_state,
                    prop_types: &root_prop_types,
                    program: parsed.program.as_ref(),
                    base: &window_bases[index],
                    resource_prefix: "",
                },
                EmitOutputs {
                    functions: &mut functions,
                    bases: &component_bases,
                    structures: &mut structures,
                    dynamic_classes: &mut dynamic_classes,
                    classes: &mut class_names,
                },
            ),
            true,
        )?;
        append_functions(&mut head_functions, &functions);
        configs.push(config);
    }
    theme::id(&mut head, &parsed.stylesheet)?;
    classes::statics(
        &mut Out::new(&mut head),
        "",
        &parsed.stylesheet,
        &class_names,
    )?;
    if dynamic_classes {
        classes::tables(&mut Out::new(&mut head), "", &parsed.stylesheet);
    }
    style::stylesheet(&mut Out::new(&mut head), "", &parsed.stylesheet);
    head.push_str(&resources::functions("", &registry.resources));

    let mut files = Vec::new();
    let mut scripts = Vec::new();
    let mut component_bodies = Vec::new();
    let mut component_functions = String::new();
    for (index, definition) in registry.definitions.iter().enumerate() {
        let mut sinks = ComponentSinks {
            structures: &mut structures,
            scripts: Some(&mut scripts),
        };
        let emitted = emit_component(
            definition,
            &component_bases[index],
            &registry,
            &component_bases,
            &names,
            commands,
            &mut sinks,
        )?;
        files.push(emitted.file);
        component_bodies.push(emitted.body);
        component_functions.push_str(&emitted.functions);
    }

    let mut tail = String::new();
    for (index, config) in configs.iter().enumerate() {
        window_config(&mut tail, &window_bases[index], config, icons[index]);
    }
    tail.push('\n');
    window_table(&mut tail, &parsed.windows, &window_bases);
    tail.push_str(&script::module(
        "script",
        parsed.program.as_ref(),
        &names,
        &root_state,
        &root_prop_types,
    )?);
    tail.push_str(&script::handle("script", parsed.program.as_ref()));
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

    push_script(&mut scripts, ROOT_SCRIPT.to_owned(), parsed.script.as_ref());

    let mut resource_files = Vec::new();
    for resource in &registry.resources {
        resource_files.push(ResourceFile {
            name: resources::function_name("", &resource.name),
            path: resource.path.clone(),
        });
    }
    for (index, definition) in registry.definitions.iter().enumerate() {
        let prefix = format!("{}_", component_bases[index]);
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
        let mut external = names::declared(&definition.component.members);
        external.extend(names::declared_events(&definition.component.members));
        validate_build(program, &external, commands)?;
    }
    Ok(())
}

struct ComponentSinks<'a> {
    structures: &'a mut StructureNames,
    scripts: Option<&'a mut Vec<GeneratedFile>>,
}

fn emit_component(
    definition: &Definition,
    base: &str,
    registry: &Registry,
    bases: &[String],
    names: &Names<'_>,
    commands: &Commands,
    sinks: &mut ComponentSinks<'_>,
) -> Result<ComponentOutput, Error> {
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
    .with_commands(commands);
    let component_prop_types = prop_types(&definition.component.members);
    body.push_str(HEADER);
    view::view(
        &mut body,
        &format!("{prefix}view"),
        root,
        &[],
        &mut Emit::component(
            EmitSources {
                script_name: &format!("{prefix}script"),
                registry,
                imports: &definition.imports,
                resources: &definition.resources,
                stylesheet: &definition.stylesheet,
                names,
                state: &component_state,
                prop_types: &component_prop_types,
                program: definition.program.as_ref(),
                base,
                resource_prefix: &prefix,
            },
            EmitOutputs {
                functions: &mut functions,
                bases,
                structures: &mut *sinks.structures,
                dynamic_classes: &mut dynamic_classes,
                classes: &mut class_names,
            },
        ),
        false,
    )?;
    if dynamic_classes {
        classes::tables(&mut Out::new(&mut body), &prefix, &definition.stylesheet);
    }
    classes::statics(
        &mut Out::new(&mut body),
        &prefix,
        &definition.stylesheet,
        &class_names,
    )?;
    style::stylesheet(&mut Out::new(&mut body), &prefix, &definition.stylesheet);
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
        .filter(|definition| !kit::is_virtual(&definition.path))
        .map(|definition| definition.path.clone())
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

fn stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("component")
        .to_owned()
}

fn definition_root(definition: &Definition) -> Result<&Element, Error> {
    let position = Position { line: 1, column: 1 };
    let mut roots = definition
        .component
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

fn icon_resource<'a>(window: &'a Window, resources: &[Resource]) -> Result<Option<&'a str>, Error> {
    let Some(value) = window.args.iter().find_map(|arg| match arg {
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

fn window_config(
    out: &mut String,
    base: &str,
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
        Some(name) => format!("Some(resource_{name}().into())"),
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

fn window_table(out: &mut String, windows: &[Window], bases: &[String]) {
    out.push_str("pub fn windows() -> Vec<brul::WindowDef<ThemeId>> {\n");
    alloc::registration(out);
    out.push_str("    vec![\n");
    for (index, window) in windows.iter().enumerate() {
        let base = &bases[index];
        let _ = writeln!(
            out,
            "        brul::WindowDef {{ name: {:?}, config: {base}_window(), view: {base}_view }},",
            window.name
        );
    }
    out.push_str("    ]\n}\n");
}
