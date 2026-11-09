use std::fmt::Write;

use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Element};

use crate::error_at;

use super::Emit;

#[allow(clippy::too_many_arguments)]
pub(super) fn component(
    out: &mut String,
    element: &Element,
    index: usize,
    emit: &mut Emit<'_>,
    theme: Option<&str>,
    depth: usize,
    counter: &mut usize,
) -> Result<(), Error> {
    let name = &emit.registry.definitions[index].name;
    if element.args.iter().any(|arg| matches!(arg, Arg::Class(_))) {
        return Err(error_at(
            element.position,
            format!("component '{name}' does not accept classes yet"),
        ));
    }
    if !element.block.items.is_empty() {
        return Err(error_at(
            element.position,
            format!("component '{name}' does not accept children"),
        ));
    }
    for arg in &element.args {
        match arg {
            Arg::Class(_) => {}
            Arg::Named { value, .. } => {
                return Err(error_at(
                    value.position,
                    "component properties are not generated yet",
                ));
            }
            Arg::Handler { .. } => {
                return Err(error_at(
                    element.position,
                    "component events are not generated yet",
                ));
            }
        }
    }
    let widget = format!("widget{}", *counter);
    *counter += 1;
    let pad = "    ".repeat(depth);
    let inner = "    ".repeat(depth + 1);
    let theme = match theme {
        Some(theme) => format!("Some({theme:?})"),
        None => "None".to_owned(),
    };
    let _ = writeln!(out, "{pad}{{");
    let _ = writeln!(
        out,
        "{inner}let built = brul::instance({theme}, component{index}_view, component{index}_script().as_ref())?;"
    );
    for field in ["bindings", "handlers", "structures", "props"] {
        let _ = writeln!(out, "{inner}{field}.extend(built.{field});");
    }
    let _ = writeln!(out, "{inner}let {widget} = built.widget;");
    let _ = writeln!(out, "{inner}{widget}");
    let _ = write!(out, "{pad}}}");
    Ok(())
}
