use std::fmt::Write;

use brul_syntax::error::Error;
use brul_syntax::markup::Element;

use crate::runtime::HandlerEvent;

use super::Emit;
use super::Frame;
use super::element::widget;

pub(super) fn view(
    out: &mut String,
    name: &str,
    root: &Element,
    handlers: &[(HandlerEvent, String)],
    emit: &mut Emit<'_>,
) -> Result<(), Error> {
    let _ = writeln!(
        out,
        "pub fn {name}(theme: Option<&str>, component: &brul::ComponentHandle) -> Result<brul::Built, String> {{"
    );
    out.push_str("    let mut bindings = Vec::new();\n");
    out.push_str("    let mut handlers = Vec::new();\n");
    out.push_str("    let mut structures = Vec::new();\n");
    out.push_str("    let mut props = Vec::new();\n");
    out.push_str("    let widget = match theme {\n");
    branch(out, "None", root, emit, None)?;
    let themes: Vec<String> = emit
        .stylesheet
        .themes
        .iter()
        .map(|theme| theme.name.clone())
        .collect();
    for theme in themes {
        let pattern = format!("Some({theme:?})");
        branch(out, &pattern, root, emit, Some(&theme))?;
    }
    out.push_str("        Some(theme) => return Err(format!(\"unknown theme '{theme}'\")),\n");
    out.push_str("    };\n");
    for (event, handler) in handlers {
        let _ = writeln!(
            out,
            "    handlers.push(brul::Handler::new(component, None, brul::HandlerEvent::{event:?}, {handler:?}));"
        );
    }
    out.push_str("    Ok(brul::Built {\n");
    out.push_str("        widget,\n");
    out.push_str("        bindings,\n");
    out.push_str("        handlers,\n");
    out.push_str("        structures,\n");
    out.push_str("        props,\n");
    out.push_str("    })\n}\n");
    Ok(())
}

fn branch(
    out: &mut String,
    pattern: &str,
    root: &Element,
    emit: &mut Emit<'_>,
    theme: Option<&str>,
) -> Result<(), Error> {
    let mut counter = 0;
    let _ = writeln!(out, "        {pattern} => {{");
    widget(out, root, emit, Frame::root(theme, 3), &mut counter)?;
    out.push_str("\n        }\n");
    Ok(())
}
