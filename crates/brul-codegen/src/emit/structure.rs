use std::collections::HashMap;
use std::fmt::Write;

use brul_syntax::error::Error;
use brul_syntax::markup::{EachBlock, IfBlock};

use super::Emit;
use super::Frame;
use super::expression::operation;
use super::{contains_slot, items, items_spliced};

use brul_script::Names;

#[derive(Default)]
pub struct StructureNames {
    counts: HashMap<String, usize>,
}

impl StructureNames {
    pub(super) fn next(&mut self, base: &str, kind: &str) -> String {
        let key = format!("{base}_{kind}");
        let count = self.counts.entry(key).or_insert(0);
        *count += 1;
        format!("{base}_{kind}{count}")
    }
}

pub(super) fn structure_if(
    out: &mut String,
    block: &IfBlock,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    let name = emit.structures.next(emit.base, "if");
    for (index, branch) in block.branches.iter().enumerate() {
        let function = format!("{name}_value{index}");
        emit.functions
            .push(value_function(&function, &branch.condition, emit.names));
    }
    let arm = rebuild_arm(&name, block, emit, frame)?;
    emit.functions.push(arm);
    inline(out, &name, &format!("{name}_value0"), false, frame, counter);
    Ok(())
}

pub(super) fn structure_each(
    out: &mut String,
    block: &EachBlock,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    let name = emit.structures.next(emit.base, "each");
    emit.functions.push(value_function(
        &format!("{name}_value"),
        &block.collection,
        emit.names,
    ));
    if let Some(key) = &block.key {
        emit.functions
            .push(key_function(&format!("{name}_key"), key, emit.names));
    }
    let arm = each_arm(&name, block, emit, frame)?;
    emit.functions.push(arm);
    inline(out, &name, &format!("{name}_value"), true, frame, counter);
    Ok(())
}

fn value_function(name: &str, value: &brul_syntax::ast::Expr, names: &Names<'_>) -> String {
    format!(
        "fn {name}(component: &mut brul::Component) -> Result<brul::Value, brul::Error> {{
    let value = {};
    Ok(value)
}}
",
        operation(value, names)
    )
}

fn key_function(name: &str, key: &brul_syntax::ast::Expr, names: &Names<'_>) -> String {
    format!(
        "fn {name}(component: &mut brul::Component) -> Result<brul::Value, brul::Error> {{
    let value = {};
    Ok(value)
}}
",
        operation(key, names)
    )
}

fn inline(
    out: &mut String,
    name: &str,
    value: &str,
    each: bool,
    frame: Frame<'_>,
    counter: &mut usize,
) {
    let pad = "    ".repeat(frame.depth());
    let inner = "    ".repeat(frame.depth() + 1);
    let widget = format!("widget{}", *counter);
    *counter += 1;
    let _ = writeln!(out, "{pad}{{");
    let _ = writeln!(
        out,
        "{inner}let (built, _rows) = {name}_rebuild(theme, component)?;"
    );
    let _ = writeln!(out, "{inner}bindings.extend(built.bindings);");
    let _ = writeln!(out, "{inner}handlers.extend(built.handlers);");
    let _ = writeln!(out, "{inner}props.extend(built.props);");
    let _ = writeln!(out, "{inner}let {widget} = built.widget;");
    let _ = writeln!(
        out,
        "{inner}brul::register_structure(component, {widget}.id(), {each}, {value}, {name}_rebuild);"
    );
    let _ = writeln!(out, "{inner}{widget}");
    let _ = write!(out, "{pad}}}");
}

fn rebuild_arm(
    name: &str,
    block: &IfBlock,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
) -> Result<String, Error> {
    let arm = arm_frame(emit.default_theme(), frame);
    let mut out = arm_header(name);
    out.push_str("    let children = {\n");
    let mut counter = 0;
    branch_chain(&mut out, name, block, emit, arm, &mut counter, 0)?;
    out.push_str("    };\n");
    widget(&mut out);
    Ok(out)
}

fn branch_chain(
    out: &mut String,
    name: &str,
    block: &IfBlock,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
    index: usize,
) -> Result<(), Error> {
    let pad = "    ".repeat(2 + index);
    let body = frame.deeper(index + 1);
    let _ = writeln!(
        out,
        "{pad}let condition = {name}_value{index}(&mut component.borrow_mut())\n{pad}    .map_err(brul::ViewError::from)?;\n{pad}let condition = match condition {{\n{pad}    brul::Value::Bool(condition) => condition,\n{pad}    other => return Err(brul::ViewError::expected(\"if condition\", \"Bool\", other.type_name())),\n{pad}}};\n{pad}if condition {{"
    );
    arm_body(out, &block.branches[index].block, emit, body, counter)?;
    let _ = writeln!(out, "{pad}}} else {{");
    if index + 1 < block.branches.len() {
        branch_chain(out, name, block, emit, frame, counter, index + 1)?;
    } else if let Some(else_block) = &block.else_block {
        arm_body(out, else_block, emit, body, counter)?;
    } else {
        let _ = writeln!(out, "{}vec![]", "    ".repeat(3 + index));
    }
    let _ = writeln!(out, "{pad}}}");
    Ok(())
}

fn arm_body(
    out: &mut String,
    block: &brul_syntax::markup::Block,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    let pad = "    ".repeat(frame.depth().saturating_sub(1));
    let inner = "    ".repeat(frame.depth());
    if contains_slot(block) {
        let _ = writeln!(out, "{pad}{{");
        let _ = writeln!(out, "{inner}let mut children = Vec::new();");
        items_spliced(out, block, emit, frame, counter)?;
        let _ = writeln!(out, "{inner}children");
        let _ = writeln!(out, "{pad}}}");
    } else {
        let _ = writeln!(out, "{pad}vec![");
        items(out, block, emit, frame, counter)?;
        let _ = writeln!(out, "{pad}]");
    }
    Ok(())
}

fn each_arm(
    name: &str,
    block: &EachBlock,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
) -> Result<String, Error> {
    let arm = arm_frame(emit.default_theme(), frame);
    let mut out = arm_header(name);
    out.push_str("    let mut children = Vec::new();\n");
    let _ = writeln!(
        out,
        "    let collection = {name}_value(&mut component.borrow_mut())\n        .map_err(brul::ViewError::from)?;\n    let items = match collection {{"
    );
    out.push_str("        brul::Value::Array(items) => std::rc::Rc::unwrap_or_clone(items),\n");
    out.push_str(
        "        other => return Err(brul::ViewError::expected(\"each\", \"Array\", other.type_name())),\n    };\n",
    );
    if let Some(else_block) = &block.else_block {
        out.push_str("    if items.is_empty() {\n");
        each_body(&mut out, else_block, emit, arm)?;
        out.push_str("        rows.push(brul::RowInfo {\n");
        out.push_str("            key: None,\n");
        out.push_str("            item: brul::Value::Array(std::rc::Rc::new(Vec::new())),\n");
        out.push_str("            start: 0,\n");
        out.push_str("            count: children.len(),\n");
        out.push_str("        });\n");
        out.push_str("    }\n");
    }
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
                "        let key = {name}_key(&mut component.borrow_mut()).ok();"
            );
        }
        None => out.push_str("        let key = None;\n"),
    }
    out.push_str("        let start = children.len();\n");
    each_body(&mut out, &block.block, emit, arm)?;
    out.push_str("        let count = children.len() - start;\n");
    out.push_str("        component.borrow_mut().pop_build_scope();\n");
    out.push_str("        rows.push(brul::RowInfo { key, item, start, count });\n");
    out.push_str("    }\n");
    widget(&mut out);
    Ok(out)
}

fn each_body(
    out: &mut String,
    block: &brul_syntax::markup::Block,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
) -> Result<(), Error> {
    let pad = "    ".repeat(frame.depth().saturating_sub(1));
    let mut counter = 0;
    if contains_slot(block) {
        items_spliced(out, block, emit, frame, &mut counter)?;
    } else if !block.items.is_empty() {
        let _ = writeln!(out, "{pad}children.extend([");
        items(out, block, emit, frame, &mut counter)?;
        let _ = writeln!(out, "{pad}]);");
    }
    Ok(())
}

fn arm_frame<'a>(theme: Option<&'a str>, frame: Frame<'a>) -> Frame<'a> {
    Frame::root(theme, 3).inside(frame.direction(), frame.gap())
}

fn variable(name: &str, names: &Names<'_>) -> u32 {
    names
        .get(name)
        .expect("each variable is collected into the name table")
        .index()
}

fn arm_header(name: &str) -> String {
    format!(
        "#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn {name}_rebuild(theme: Option<&str>, component: &brul::ComponentHandle) -> brul::Rebuilt {{
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let mut rows = Vec::new();
"
    )
}

fn widget(out: &mut String) {
    out.push_str("    let widget = brul::gui::Widget::structure(children);\n");
    out.push_str("    Ok((brul::Built { widget, bindings, handlers, props }, rows))\n}\n");
}
