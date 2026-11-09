use std::fmt::Write;

use brul_syntax::error::Error;
use brul_syntax::markup::{EachBlock, IfBlock};

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
    let index = *emit.next_structure;
    *emit.next_structure += 1;
    emit.functions.push(value_function(index, &block.condition));
    emit.functions.push(dispatcher(index, emit));
    let themes = themes(emit);
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
    inline(out, index, false, theme_expression, depth, counter);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn structure_each(
    out: &mut String,
    block: &EachBlock,
    emit: &mut Emit<'_>,
    direction: &str,
    spacing: u32,
    theme_expression: &str,
    depth: usize,
    counter: &mut usize,
) -> Result<(), Error> {
    let index = *emit.next_structure;
    *emit.next_structure += 1;
    emit.functions
        .push(value_function(index, &block.collection));
    emit.functions.push(dispatcher(index, emit));
    let themes = themes(emit);
    for (position, theme) in themes.iter().enumerate() {
        let arm = each_arm(
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
    inline(out, index, true, theme_expression, depth, counter);
    Ok(())
}

fn value_function(index: usize, value: &brul_syntax::ast::Expr) -> String {
    format!(
        "fn structure{index}_value(component: &brul::ComponentHandle) -> Option<brul::Value> {{
    brul::evaluate(component, &{})
}}
",
        expression(value)
    )
}

fn dispatcher(index: usize, emit: &Emit<'_>) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "fn structure{index}_rebuild(theme: Option<&str>, component: &brul::ComponentHandle) -> brul::Rebuilt {{
    match theme {{
        None => structure{index}_rebuild_0(component),"
    );
    for (position, theme) in emit.stylesheet.themes.iter().enumerate() {
        let _ = writeln!(
            out,
            "        Some({:?}) => structure{index}_rebuild_{}(component),",
            theme.name,
            position + 1
        );
    }
    let _ = writeln!(
        out,
        "        Some(theme) => Err(format!(\"unknown theme '{{theme}}'\")),\n    }}\n}}"
    );
    out
}

fn themes(emit: &Emit<'_>) -> Vec<Option<String>> {
    std::iter::once(None)
        .chain(
            emit.stylesheet
                .themes
                .iter()
                .map(|theme| Some(theme.name.clone())),
        )
        .collect()
}

fn inline(
    out: &mut String,
    index: usize,
    each: bool,
    theme_expression: &str,
    depth: usize,
    counter: &mut usize,
) {
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
        "{inner}structures.push(brul::Structure {{ parent: {name}.id(), each: {each}, value: structure{index}_value, rebuild: structure{index}_rebuild }});"
    );
    let _ = writeln!(out, "{inner}{name}");
    let _ = write!(out, "{pad}}}");
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
    let mut out = arm_header(index, position);
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
    widget(&mut out, direction, spacing);
    Ok(out)
}

fn each_arm(
    index: usize,
    position: usize,
    theme: Option<&str>,
    block: &EachBlock,
    emit: &mut Emit<'_>,
    direction: &str,
    spacing: u32,
) -> Result<String, Error> {
    let mut out = arm_header(index, position);
    out.push_str("    let mut children = Vec::new();\n");
    let _ = writeln!(
        out,
        "    let items = match brul::evaluate(component, &{}) {{",
        expression(&block.collection)
    );
    out.push_str("        Some(brul::Value::Array(items)) => items,\n");
    out.push_str(
        "        Some(other) => return Err(format!(\"each expects Array, got {}\", other.type_name())),\n",
    );
    out.push_str("        None => return Err(\"each failed to evaluate\".to_owned()),\n");
    out.push_str("    };\n");
    out.push_str("    for (position, item) in items.into_iter().enumerate() {\n");
    out.push_str("        component.borrow_mut().push_build_scope(&[\n");
    let _ = writeln!(
        out,
        "            brul::Variable {{ name: {:?}.to_owned(), value: item.clone() }},",
        block.item
    );
    if let Some(index_name) = &block.index {
        let _ = writeln!(
            out,
            "            brul::Variable {{ name: {index_name:?}.to_owned(), value: brul::Value::Number(position as f64) }},"
        );
    }
    out.push_str("        ]);\n");
    match &block.key {
        Some(key) => {
            let _ = writeln!(out, "        let key = Some({});", expression(key));
        }
        None => out.push_str("        let key = None;\n"),
    }
    out.push_str("        let start = children.len();\n");
    if !block.block.items.is_empty() {
        out.push_str("        children.extend([\n");
        let mut counter = 0;
        items(
            &mut out,
            &block.block,
            emit,
            theme,
            direction,
            spacing,
            3,
            &mut counter,
        )?;
        out.push_str("        ]);\n");
    }
    out.push_str("        let count = children.len() - start;\n");
    out.push_str("        component.borrow_mut().pop_build_scope();\n");
    out.push_str("        rows.push(brul::RowInfo { key, item, start, count });\n");
    out.push_str("    }\n");
    widget(&mut out, direction, spacing);
    Ok(out)
}

fn arm_header(index: usize, position: usize) -> String {
    format!(
        "#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn structure{index}_rebuild_{position}(component: &brul::ComponentHandle) -> brul::Rebuilt {{
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut structures = Vec::new();
    let mut props = Vec::new();
    let mut rows = Vec::new();
"
    )
}

fn widget(out: &mut String, direction: &str, spacing: u32) {
    let _ = writeln!(
        out,
        "    let widget = brul_gui::Widget::view(brul_gui::Direction::{direction}, children)\n        .with_layout(brul_gui::LayoutStyle {{ spacing: {spacing}, ..Default::default() }});"
    );
    out.push_str(
        "    Ok((brul::Built { widget, bindings, handlers, structures, props }, rows))\n}\n",
    );
}
