use std::collections::HashSet;
use std::fmt::Write;
use std::fs;
use std::path::Path;

use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::markup::{Element, Member, Window};

use crate::Definition;
use crate::error_at;

use super::Emit;
use super::StructureNames;
use super::{classes, events, names, resources, script, style, view};

const HEADER: &str = "#[allow(unused_mut, unused_variables, clippy::let_and_return)]\n";
const ROOT_SCRIPT: &str = "view.script";

pub struct GeneratedFile {
    pub name: String,
    pub source: String,
}

pub struct Module {
    pub root: String,
    pub files: Vec<GeneratedFile>,
    pub scripts: Vec<GeneratedFile>,
    flat: String,
}

impl Module {
    pub fn flat(&self) -> &str {
        &self.flat
    }
}

pub fn module(source: &str) -> Result<String, Error> {
    let module = module_at(source, None, false)?;
    Ok(module.flat)
}

pub fn module_file(path: &Path) -> Result<String, Error> {
    let module = split_module_file(path)?;
    Ok(module.flat)
}

pub fn split_module_file(path: &Path) -> Result<Module, Error> {
    let position = Position { line: 1, column: 1 };
    let source = fs::read_to_string(path).map_err(|error| {
        error_at(
            position,
            format!("cannot read '{}': {error}", path.display()),
        )
    })?;
    module_at(&source, path.parent(), true)
}

fn module_at(source: &str, base: Option<&Path>, split: bool) -> Result<Module, Error> {
    let parsed = crate::parse_source(source)?;
    let registry = crate::resolve(&parsed.uses, base)?;
    let names = names::table(&parsed, &registry.definitions);
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

    head.push_str(HEADER);
    let mut dynamic_classes = false;
    for (index, window) in parsed.windows.iter().enumerate() {
        let (config, window_handlers) = crate::arguments(window)?;
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
            &mut Emit {
                registry: &registry,
                imports: &registry.imports,
                resources: &registry.resources,
                base: &window_bases[index],
                resource_prefix: "",
                stylesheet: &parsed.stylesheet,
                names: &names,
                program: parsed.program.as_ref(),
                functions: &mut functions,
                bases: &component_bases,
                structures: &mut structures,
                dynamic_classes: &mut dynamic_classes,
            },
        )?;
        for function in &functions {
            head_functions.push('\n');
            head_functions.push_str(function);
        }
        configs.push(config);
    }
    if dynamic_classes {
        classes::tables(&mut head, "", &parsed.stylesheet);
    }
    style::stylesheet(&mut head, "", &parsed.stylesheet);
    head.push_str(&resources::functions("", &registry.resources));

    let mut files = Vec::new();
    let mut scripts = Vec::new();
    let mut component_bodies = Vec::new();
    let mut component_functions = String::new();
    for (index, definition) in registry.definitions.iter().enumerate() {
        let root = definition_root(definition)?;
        let base = &component_bases[index];
        let prefix = format!("{base}_");
        let name = format!("bc_{base}");
        let mut body = String::new();
        let mut functions = Vec::new();
        let mut dynamic_classes = false;
        body.push_str(HEADER);
        view::view(
            &mut body,
            &format!("{prefix}view"),
            root,
            &[],
            &mut Emit {
                registry: &registry,
                imports: &definition.imports,
                resources: &definition.resources,
                base,
                resource_prefix: &prefix,
                stylesheet: &definition.stylesheet,
                names: &names,
                program: definition.program.as_ref(),
                functions: &mut functions,
                bases: &component_bases,
                structures: &mut structures,
                dynamic_classes: &mut dynamic_classes,
            },
        )?;
        if dynamic_classes {
            classes::tables(&mut body, &prefix, &definition.stylesheet);
        }
        style::stylesheet(&mut body, &prefix, &definition.stylesheet);
        let script_file = format!("{name}.script");
        body.push_str(&script::program(
            &format!("{prefix}script"),
            definition.script.as_ref(),
            split.then_some(script_file.as_str()),
        ));
        body.push_str(&resources::functions(&prefix, &definition.resources));
        let mut functions_block = String::new();
        for function in &functions {
            functions_block.push('\n');
            functions_block.push_str(function);
            component_functions.push('\n');
            component_functions.push_str(function);
        }
        if let Some(script) = &definition.script {
            scripts.push(GeneratedFile {
                name: script_file,
                source: script.text.clone(),
            });
        }
        let mut file = String::from("use super::*;\n");
        file.push_str(&body);
        file.push_str(&functions_block);
        files.push(GeneratedFile { name, source: file });
        component_bodies.push(body);
    }

    let mut tail = String::new();
    for (index, config) in configs.iter().enumerate() {
        window_config(&mut tail, &window_bases[index], config);
    }
    tail.push('\n');
    window_table(&mut tail, &parsed.windows, &window_bases);
    tail.push_str(&script::program(
        "script",
        parsed.script.as_ref(),
        split.then_some(ROOT_SCRIPT),
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

    if let Some(script) = &parsed.script {
        scripts.push(GeneratedFile {
            name: ROOT_SCRIPT.to_owned(),
            source: script.text.clone(),
        });
    }

    Ok(Module {
        root,
        files,
        scripts,
        flat,
    })
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

fn identifier(text: &str) -> String {
    let mut name: String = text
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if name.is_empty() || name.starts_with(|character: char| character.is_ascii_digit()) {
        name.insert(0, '_');
    }
    name
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

fn window_config(out: &mut String, base: &str, config: &brul_window::WindowConfig) {
    let position = match config.position {
        Some(position) => format!(
            "Some(brul::WindowPosition {{ x: {:?}, y: {:?} }})",
            position.x, position.y
        ),
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
        position: {position},
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
        config.centered
    );
}

fn window_table(out: &mut String, windows: &[Window], bases: &[String]) {
    out.push_str("pub fn windows() -> Vec<brul::WindowDef> {\n    vec![\n");
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
