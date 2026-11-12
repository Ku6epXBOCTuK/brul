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
use super::{classes, events, names, palette, resources, script, view};

const HEADER: &str = "#[allow(unused_mut, unused_variables, clippy::let_and_return)]\n";

pub struct GeneratedFile {
    pub name: String,
    pub source: String,
}

pub struct Module {
    pub root: String,
    pub files: Vec<GeneratedFile>,
    flat: String,
}

impl Module {
    pub fn flat(&self) -> &str {
        &self.flat
    }
}

pub fn module(source: &str) -> Result<String, Error> {
    let module = module_at(source, None)?;
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
    module_at(&source, path.parent())
}

fn module_at(source: &str, base: Option<&Path>) -> Result<Module, Error> {
    let parsed = crate::parse_source(source)?;
    let registry = crate::resolve(&parsed.uses, base)?;
    let names = names::table(&parsed, &registry.definitions);
    let mut next_structure = 0;
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
            &view_name(index),
            root,
            &window_handlers,
            &mut Emit {
                registry: &registry,
                imports: &registry.imports,
                resources: &registry.resources,
                resource_prefix: "",
                stylesheet: &parsed.stylesheet,
                names: &names,
                program: parsed.program.as_ref(),
                functions: &mut functions,
                next_structure: &mut next_structure,
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
        classes::tables(&mut head, "", &parsed.stylesheet)
            .map_err(|message| error_at(Position { line: 1, column: 1 }, message))?;
    }
    palette::functions(&mut head, "Palette", "palette", &parsed.stylesheet)
        .map_err(|message| error_at(Position { line: 1, column: 1 }, message))?;
    head.push_str(&resources::functions("", &registry.resources));

    let mut files = Vec::new();
    let mut component_bodies = Vec::new();
    let mut component_functions = String::new();
    let mut used = HashSet::new();
    for (index, definition) in registry.definitions.iter().enumerate() {
        let root = definition_root(definition)?;
        let prefix = format!("component{index}_");
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
                resource_prefix: &prefix,
                stylesheet: &definition.stylesheet,
                names: &names,
                program: definition.program.as_ref(),
                functions: &mut functions,
                next_structure: &mut next_structure,
                dynamic_classes: &mut dynamic_classes,
            },
        )?;
        if dynamic_classes {
            classes::tables(&mut body, &prefix, &definition.stylesheet)
                .map_err(|message| error_at(Position { line: 1, column: 1 }, message))?;
        }
        palette::functions(
            &mut body,
            &format!("Component{index}Palette"),
            &format!("{prefix}palette"),
            &definition.stylesheet,
        )
        .map_err(|message| error_at(Position { line: 1, column: 1 }, message))?;
        body.push_str(&script::program(
            &format!("{prefix}script"),
            definition.program.as_ref(),
        ));
        body.push_str(&resources::functions(&prefix, &definition.resources));
        let mut functions_block = String::new();
        for function in &functions {
            functions_block.push('\n');
            functions_block.push_str(function);
            component_functions.push('\n');
            component_functions.push_str(function);
        }
        let name = module_name(&definition.path, &mut used);
        let mut file = String::from("use super::*;\n");
        file.push_str(&body);
        file.push_str(&functions_block);
        files.push(GeneratedFile { name, source: file });
        component_bodies.push(body);
    }

    let mut tail = String::new();
    for (index, config) in configs.iter().enumerate() {
        window_config(&mut tail, index, config);
    }
    tail.push('\n');
    window_table(&mut tail, &parsed.windows);
    tail.push_str(&script::program("script", parsed.program.as_ref()));
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

    Ok(Module { root, files, flat })
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

fn module_name(path: &Path, used: &mut HashSet<String>) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("component");
    let mut name: String = stem
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect();
    name.insert_str(0, "bc_");
    let mut candidate = name.clone();
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

fn view_name(index: usize) -> String {
    match index {
        0 => "view".to_owned(),
        index => format!("view{index}"),
    }
}

fn window_config(out: &mut String, index: usize, config: &brul_window::WindowConfig) {
    let name = if index == 0 {
        "window".to_owned()
    } else {
        format!("window{index}")
    };
    let _ = write!(
        out,
        "pub fn {name}() -> brul::WindowConfig {{
    brul::WindowConfig {{
        title: {:?}.to_owned(),
        width: {},
        height: {},
        frameless: {},
        transparent: {},
        ..brul::WindowConfig::default()
    }}
}}
",
        config.title, config.width, config.height, config.frameless, config.transparent
    );
}

fn window_table(out: &mut String, windows: &[Window]) {
    out.push_str("pub fn windows() -> Vec<brul::WindowDef> {\n    vec![\n");
    for (index, window) in windows.iter().enumerate() {
        let config = if index == 0 {
            "window()".to_owned()
        } else {
            format!("window{index}()")
        };
        let view = if index == 0 {
            "view".to_owned()
        } else {
            format!("view: {}", view_name(index))
        };
        let _ = writeln!(
            out,
            "        brul::WindowDef {{ name: {:?}, config: {config}, {view} }},",
            window.name
        );
    }
    out.push_str("    ]\n}\n");
}
