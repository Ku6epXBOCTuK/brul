use std::fmt::Write;

use brul_syntax::error::Error;
use brul_syntax::markup::IfBlock;

use super::Emit;
use super::expression;
use super::items;

#[allow(clippy::too_many_arguments)]
pub(super) fn structure_if(
    out: &mut String,
    block: &IfBlock,
    emit: &mut Emit<'_>,
    direction: &str,
    spacing: u32,
    theme_expression: &str,
    depth: usize,
    counter: &mut usize,
) -> Result<(), Error> {
    let index = emit.next_structure;
    emit.next_structure += 1;
    let condition = expression(&block.condition);
    emit.functions.push(format!(
        "fn structure{index}_value(component: &brul::ComponentHandle) -> Option<brul::Value> {{
    brul::evaluate(component, &{condition})
}}
"
    ));
    let mut dispatcher = String::new();
    let _ = writeln!(
        dispatcher,
        "fn structure{index}_rebuild(theme: Option<&str>, component: &brul::ComponentHandle) -> brul::Rebuilt {{
    match theme {{
        None => structure{index}_rebuild_0(component),"
    );
    for (position, name) in emit.stylesheet.themes.iter().enumerate() {
        let _ = writeln!(
            dispatcher,
            "        Some({:?}) => structure{index}_rebuild_{}(component),",
            name.name,
            position + 1
        );
    }
    let _ = writeln!(
        dispatcher,
        "        Some(theme) => Err(format!(\"unknown theme '{{theme}}'\")),\n    }}\n}}"
    );
    emit.functions.push(dispatcher);
    let themes: Vec<Option<String>> = std::iter::once(None)
        .chain(
            emit.stylesheet
                .themes
                .iter()
                .map(|theme| Some(theme.name.clone())),
        )
        .collect();
    for (position, theme) in themes.iter().enumerate() {
        let arm = rebuild_arm(
            index,
            position,
            theme.as_deref(),
            block,
            emit,
            direction,
            spacing,
        )?;
        emit.functions.push(arm);
    }
    let pad = "    ".repeat(depth);
    let inner = "    ".repeat(depth + 1);
    let name = format!("widget{}", *counter);
    *counter += 1;
    let _ = writeln!(out, "{pad}{{");
    let _ = writeln!(
        out,
        "{inner}let (built, _rows) = structure{index}_rebuild({theme_expression}, component)?;"
    );
    let _ = writeln!(out, "{inner}bindings.extend(built.bindings);");
    let _ = writeln!(out, "{inner}handlers.extend(built.handlers);");
    let _ = writeln!(out, "{inner}structures.extend(built.structures);");
    let _ = writeln!(out, "{inner}props.extend(built.props);");
    let _ = writeln!(out, "{inner}let {name} = built.widget;");
    let _ = writeln!(
        out,
        "{inner}structures.push(brul::Structure {{ parent: {name}.id(), each: false, value: structure{index}_value, rebuild: structure{index}_rebuild }});"
    );
    let _ = writeln!(out, "{inner}{name}");
    let _ = write!(out, "{pad}}}");
    Ok(())
}

fn rebuild_arm(
    index: usize,
    position: usize,
    theme: Option<&str>,
    block: &IfBlock,
    emit: &mut Emit<'_>,
    direction: &str,
    spacing: u32,
) -> Result<String, Error> {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn structure{index}_rebuild_{position}(component: &brul::ComponentHandle) -> brul::Rebuilt {{"
    );
    out.push_str("    let mut bindings = Vec::new();\n");
    out.push_str("    let mut handlers = Vec::new();\n");
    out.push_str("    let mut structures = Vec::new();\n");
    out.push_str("    let mut props = Vec::new();\n");
    out.push_str("    let mut rows = Vec::new();\n");
    let _ = writeln!(
        out,
        "    let children = match brul::evaluate(component, &{}) {{",
        expression(&block.condition)
    );
    out.push_str("        Some(brul::Value::Bool(true)) => vec![\n");
    let mut counter = 0;
    items(
        &mut out,
        &block.then_block,
        emit,
        theme,
        direction,
        spacing,
        3,
        &mut counter,
    )?;
    out.push_str("        ],\n");
    out.push_str("        Some(brul::Value::Bool(false)) => vec![\n");
    if let Some(else_block) = &block.else_block {
        items(
            &mut out,
            else_block,
            emit,
            theme,
            direction,
            spacing,
            3,
            &mut counter,
        )?;
    }
    out.push_str("        ],\n");
    out.push_str(
        "        Some(other) => return Err(format!(\"if condition expects Bool, got {}\", other.type_name())),\n",
    );
    out.push_str("        None => return Err(\"if condition failed to evaluate\".to_owned()),\n");
    out.push_str("    };\n");
    let _ = writeln!(
        out,
        "    let widget = brul_gui::Widget::view(brul_gui::Direction::{direction}, children)\n        .with_layout(brul_gui::LayoutStyle {{ spacing: {spacing}, ..Default::default() }});"
    );
    out.push_str(
        "    Ok((brul::Built { widget, bindings, handlers, structures, props }, rows))\n}\n",
    );
    Ok(out)
}
