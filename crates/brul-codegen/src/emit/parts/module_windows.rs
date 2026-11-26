use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use brul_case::pascal;
use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Declaration, Element, Member};
use brul_syntax::style::Stylesheet;
use brul_syntax::types::Type;

use crate::error_at;
use crate::{Definition, Resource};

use super::CodegenContext;
use super::Emit;
use super::EmitOutputs;
use super::Out;
use super::classes::ClassNames;
use super::view::ViewSpec;
use super::{alloc, classes, style, view};

use super::module::GeneratedFile;
use super::module_component::{ViewKind, WindowView};

pub(super) fn emit_view<'a>(
    out: &mut String,
    spec: ViewSpec<'a>,
    context: &'a CodegenContext<'a>,
    outputs: EmitOutputs<'a>,
) -> Result<(), Error> {
    let slots = matches!(spec.kind, ViewKind::Component);
    let mut emit = Emit::new(context, outputs, slots);
    view::view(out, &spec, &mut emit)
}

pub(super) fn emit_tail(
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

pub(super) fn push_script(
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

pub(super) fn module_sources(registry: &crate::Registry, entry: Option<&Path>) -> Vec<PathBuf> {
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

pub(super) fn module_table(root: &mut String, files: &[GeneratedFile]) {
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

pub(super) fn state_type(name: &str) -> String {
    format!("{}State", pascal(name))
}

pub(super) fn prop_types(members: &[Member]) -> HashMap<String, Type> {
    members
        .iter()
        .filter_map(|member| match member {
            Member::Prop(prop) => Some((prop.name.clone(), prop.ty.clone())),
            _ => None,
        })
        .collect()
}

pub(super) fn unique(name: &str, used: &mut HashSet<String>) -> String {
    let mut candidate = name.to_owned();
    let mut index = 1;
    while !used.insert(candidate.clone()) {
        candidate = format!("{name}{index}");
        index += 1;
    }
    candidate
}

pub(super) fn definition_root(definition: &Definition) -> Result<&Element, Error> {
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

pub(super) fn icon_resource<'a>(
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

pub(super) fn effects_registry(out: &mut String, names: &BTreeSet<String>) {
    out.push_str(
        "pub fn __brul_effects() -> brul::effect::Effects {\n    brul::effect::Effects::new(&[\n",
    );
    for name in names {
        let _ = writeln!(out, "        crate::BRUL_EFFECT_{name},");
    }
    out.push_str("    ])\n}\n\n");
}

pub(super) fn window_config(
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

pub(super) fn window_table(out: &mut String, views: &[WindowView<'_>]) {
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
