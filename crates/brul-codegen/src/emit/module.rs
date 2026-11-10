use std::fmt::Write;
use std::fs;
use std::path::Path;

use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::markup::{Element, Member};

use crate::Definition;
use crate::error_at;

use super::Emit;
use super::{events, names, script, view};

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
    let (config, window_handlers) = crate::arguments(&parsed.window)?;
    let window_handlers = events::window(&window_handlers, parsed.program.as_ref())?;
    let root = crate::root(&parsed.window)?;
    let mut functions = Vec::new();
    let mut next_structure = 0;
    let mut out = String::new();
    out.push_str("#[allow(unused_mut, unused_variables, clippy::let_and_return)]\n");
    view::view(
        &mut out,
        "view",
        root,
        &window_handlers,
        &mut Emit {
            registry: &registry,
            imports: &registry.imports,
            stylesheet: &parsed.stylesheet,
            program: parsed.program.as_ref(),
            functions: &mut functions,
            next_structure: &mut next_structure,
        },
    )?;
    for index in 0..registry.definitions.len() {
        let definition = &registry.definitions[index];
        let root = definition_root(definition)?;
        out.push('\n');
        out.push_str("#[allow(dead_code, unused_mut, unused_variables, clippy::let_and_return)]\n");
        view::view(
            &mut out,
            &format!("component{index}_view"),
            root,
            &[],
            &mut Emit {
                registry: &registry,
                imports: &definition.imports,
                stylesheet: &definition.stylesheet,
                program: definition.program.as_ref(),
                functions: &mut functions,
                next_structure: &mut next_structure,
            },
        )?;
        out.push_str(&script::program(
            &format!("component{index}_script"),
            definition.program.as_ref(),
        ));
    }
    for function in &functions {
        out.push('\n');
        out.push_str(function);
    }
    out.push('\n');
    window_config(&mut out, &config);
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

fn window_config(out: &mut String, config: &brul_window::WindowConfig) {
    let _ = write!(
        out,
        "pub fn window() -> brul::WindowConfig {{
    brul::WindowConfig {{
        title: {:?}.to_owned(),
        width: {},
        height: {},
        ..brul::WindowConfig::default()
    }}
}}
",
        config.title, config.width, config.height
    );
}
