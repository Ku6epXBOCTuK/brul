use std::fmt::Write;

use brul_syntax::error::Error;
use brul_syntax::markup::{EachBlock, IfBlock};

use super::Emit;
use super::Frame;
use super::expression::operation;
use super::items;

use brul_script::Names;

pub(super) fn structure_if(
    out: &mut String,
    block: &IfBlock,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    let index = *emit.next_structure;
    *emit.next_structure += 1;
    emit.functions
        .push(value_function(index, &block.condition, emit.names));
    let arm = rebuild_arm(index, block, emit, frame)?;
    emit.functions.push(arm);
    inline(out, index, false, frame, counter);
    Ok(())
}

pub(super) fn structure_each(
    out: &mut String,
    block: &EachBlock,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    let index = *emit.next_structure;
    *emit.next_structure += 1;
    emit.functions
        .push(value_function(index, &block.collection, emit.names));
    if let Some(key) = &block.key {
        emit.functions.push(key_function(index, key, emit.names));
    }
    let arm = each_arm(index, block, emit, frame)?;
    emit.functions.push(arm);
    inline(out, index, true, frame, counter);
    Ok(())
}

fn value_function(index: usize, value: &brul_syntax::ast::Expr, names: &Names<'_>) -> String {
    format!(
        "fn structure{index}_value(component: &mut brul::Component) -> Result<brul::Value, brul::Error> {{
    let value = {};
    Ok(value)
}}
",
        operation(value, names)
    )
}

fn key_function(index: usize, key: &brul_syntax::ast::Expr, names: &Names<'_>) -> String {
    format!(
        "fn structure{index}_key(component: &mut brul::Component) -> Result<brul::Value, brul::Error> {{
    let value = {};
    Ok(value)
}}
",
        operation(key, names)
    )
}

fn inline(out: &mut String, index: usize, each: bool, frame: Frame<'_>, counter: &mut usize) {
    let pad = "    ".repeat(frame.depth());
    let inner = "    ".repeat(frame.depth() + 1);
    let name = format!("widget{}", *counter);
    *counter += 1;
    let _ = writeln!(out, "{pad}{{");
    let _ = writeln!(
        out,
        "{inner}let (built, _rows) = structure{index}_rebuild(theme, component)?;"
    );
    let _ = writeln!(out, "{inner}bindings.extend(built.bindings);");
    let _ = writeln!(out, "{inner}handlers.extend(built.handlers);");
    let _ = writeln!(out, "{inner}props.extend(built.props);");
    let _ = writeln!(out, "{inner}let {name} = built.widget;");
    let _ = writeln!(
        out,
        "{inner}brul::register_structure(component, {name}.id(), {each}, structure{index}_value, structure{index}_rebuild);"
    );
    let _ = writeln!(out, "{inner}{name}");
    let _ = write!(out, "{pad}}}");
}

fn rebuild_arm(
    index: usize,
    block: &IfBlock,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
) -> Result<String, Error> {
    let arm = arm_frame(emit.default_theme(), frame);
    let mut out = arm_header(index, emit);
    let _ = writeln!(
        out,
        "    let condition = structure{index}_value(&mut component.borrow_mut())\n        .map_err(|error| error.reason())?;\n    let children = match condition {{"
    );
    out.push_str("        brul::Value::Bool(true) => vec![\n");
    let mut counter = 0;
    items(&mut out, &block.then_block, emit, arm, &mut counter)?;
    out.push_str("        ],\n");
    out.push_str("        brul::Value::Bool(false) => vec![\n");
    if let Some(else_block) = &block.else_block {
        items(&mut out, else_block, emit, arm, &mut counter)?;
    }
    out.push_str("        ],\n");
    out.push_str(
        "        other => return Err(format!(\"if condition expects Bool, got {}\", other.type_name())),\n    };\n",
    );
    widget(&mut out, arm);
    Ok(out)
}

fn each_arm(
    index: usize,
    block: &EachBlock,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
) -> Result<String, Error> {
    let arm = arm_frame(emit.default_theme(), frame);
    let mut out = arm_header(index, emit);
    out.push_str("    let mut children = Vec::new();\n");
    let _ = writeln!(
        out,
        "    let collection = structure{index}_value(&mut component.borrow_mut())\n        .map_err(|error| error.reason())?;\n    let items = match collection {{"
    );
    out.push_str("        brul::Value::Array(items) => std::rc::Rc::unwrap_or_clone(items),\n");
    out.push_str(
        "        other => return Err(format!(\"each expects Array, got {}\", other.type_name())),\n    };\n",
    );
    out.push_str("    for (position, item) in items.into_iter().enumerate() {\n");
    out.push_str("        component.borrow_mut().push_build_scope(vec![\n");
    let _ = writeln!(
        out,
        "            brul::Variable {{ name: brul::VariableId::from_index({}), value: item.clone() }},",
        variable(block.item.as_str(), emit.names)
    );
    if let Some(index_name) = &block.index {
        let _ = writeln!(
            out,
            "            brul::Variable {{ name: brul::VariableId::from_index({}), value: brul::Value::Number(position as f64) }},",
            variable(index_name.as_str(), emit.names)
        );
    }
    out.push_str("        ]);\n");
    match &block.key {
        Some(_) => {
            let _ = writeln!(
                out,
                "        let key = structure{index}_key(&mut component.borrow_mut()).ok();"
            );
        }
        None => out.push_str("        let key = None;\n"),
    }
    out.push_str("        let start = children.len();\n");
    if !block.block.items.is_empty() {
        out.push_str("        children.extend([\n");
        let mut counter = 0;
        items(&mut out, &block.block, emit, arm, &mut counter)?;
        out.push_str("        ]);\n");
    }
    out.push_str("        let count = children.len() - start;\n");
    out.push_str("        component.borrow_mut().pop_build_scope();\n");
    out.push_str("        rows.push(brul::RowInfo { key, item, start, count });\n");
    out.push_str("    }\n");
    widget(&mut out, arm);
    Ok(out)
}

fn arm_frame<'a>(theme: Option<&'a str>, frame: Frame<'a>) -> Frame<'a> {
    Frame::root(theme, 3).inside(frame.direction(), frame.spacing())
}

fn variable(name: &str, names: &Names<'_>) -> u32 {
    names
        .get(name)
        .expect("each variable is collected into the name table")
        .index()
}

fn arm_header(index: usize, emit: &Emit<'_>) -> String {
    format!(
        "#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn structure{index}_rebuild(theme: Option<&str>, component: &brul::ComponentHandle) -> brul::Rebuilt {{
    let palette = {}palette(theme)?;
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let mut rows = Vec::new();
",
        emit.resource_prefix
    )
}

fn widget(out: &mut String, frame: Frame<'_>) {
    let direction = frame.direction();
    let spacing = frame.spacing();
    let _ = writeln!(
        out,
        "    let widget = brul_gui::Widget::view(brul_gui::Direction::{direction}, children)\n        .with_layout(brul_gui::LayoutStyle {{ spacing: {spacing}, ..Default::default() }});"
    );
    out.push_str("    Ok((brul::Built { widget, bindings, handlers, props }, rows))\n}\n");
}
