use std::fmt::Write;
use std::fs;
use std::path::Path;

use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::markup::{Element, Member, Window};

use crate::Definition;
use crate::error_at;

use super::Emit;
use super::{classes, events, names, resources, script, view};

pub fn module(source: &str) -> Result<String, Error> {
    module_at(source, None)
}

pub fn module_file(path: &Path) -> Result<String, Error> {
    let position = Position { line: 1, column: 1 };
    let source = fs::read_to_string(path).map_err(|error| {
        error_at(
            position,
            format!("cannot read '{}': {error}", path.display()),
        )
    })?;
    module_at(&source, path.parent())
}

fn module_at(source: &str, base: Option<&Path>) -> Result<String, Error> {
    let parsed = crate::parse_source(source)?;
    let registry = crate::resolve(&parsed.uses, base)?;
    let names = names::table(&parsed, &registry.definitions);
    let mut functions = Vec::new();
    let mut next_structure = 0;
    let mut out = String::new();
    out.push_str("#[allow(unused_mut, unused_variables, clippy::let_and_return)]\n");

    let mut configs = Vec::new();
    let mut dynamic_classes = false;
    for (index, window) in parsed.windows.iter().enumerate() {
        let (config, window_handlers) = crate::arguments(window)?;
        let window_handlers = events::window(&window_handlers, parsed.program.as_ref())?;
        let root = crate::root(window)?;
        if index > 0 {
            out.push('\n');
            out.push_str("#[allow(unused_mut, unused_variables, clippy::let_and_return)]\n");
        }
        view::view(
            &mut out,
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
        configs.push(config);
    }
    if dynamic_classes {
        classes::tables(&mut out, "", &parsed.stylesheet)
            .map_err(|message| error_at(Position { line: 1, column: 1 }, message))?;
    }
    out.push_str(&resources::functions("", &registry.resources));
    for index in 0..registry.definitions.len() {
        let definition = &registry.definitions[index];
        let root = definition_root(definition)?;
        let prefix = format!("component{index}_");
        out.push('\n');
        out.push_str("#[allow(dead_code, unused_mut, unused_variables, clippy::let_and_return)]\n");
        let mut dynamic_classes = false;
        view::view(
            &mut out,
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
            classes::tables(&mut out, &prefix, &definition.stylesheet)
                .map_err(|message| error_at(Position { line: 1, column: 1 }, message))?;
        }
        out.push_str(&script::program(
            &format!("{prefix}script"),
            definition.program.as_ref(),
        ));
        out.push_str(&resources::functions(&prefix, &definition.resources));
    }
    for function in &functions {
        out.push('\n');
        out.push_str(function);
    }
    out.push('\n');
    for (index, config) in configs.iter().enumerate() {
        window_config(&mut out, index, config);
    }
    out.push('\n');
    window_table(&mut out, &parsed.windows);
    out.push_str(&script::program("script", parsed.program.as_ref()));
    out.push_str(&names::output(&names));
    Ok(out)
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
