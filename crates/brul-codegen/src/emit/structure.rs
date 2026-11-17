use std::collections::HashMap;

use brul_syntax::error::Error;
use brul_syntax::markup::{Block, EachBlock, IfBlock};

use super::Emit;
use super::Frame;
use super::Out;
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
    out: &mut Out<'_>,
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
    inline(out, &name, &format!("{name}_value0"), false, counter);
    Ok(())
}

pub(super) fn structure_each(
    out: &mut Out<'_>,
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
            .push(value_function(&format!("{name}_key"), key, emit.names));
    }
    let arm = each_arm(&name, block, emit, frame)?;
    emit.functions.push(arm);
    inline(out, &name, &format!("{name}_value"), true, counter);
    Ok(())
}

fn value_function(name: &str, value: &brul_syntax::ast::Expr, names: &Names<'_>) -> String {
    let mut text = String::new();
    let mut out = Out::new(&mut text);
    out.line(format_args!(
        "fn {name}(component: &mut brul::Component) -> Result<brul::Value, brul::Error> {{"
    ));
    out.indented(|out| {
        out.line(format_args!("let value = {};", operation(value, names)));
        out.line(format_args!("Ok(value)"));
    });
    out.line(format_args!("}}"));
    text
}

fn inline(out: &mut Out<'_>, name: &str, value: &str, each: bool, counter: &mut usize) {
    let widget = format!("widget{}", *counter);
    *counter += 1;
    out.line(format_args!("{{"));
    out.indented(|out| {
        out.line(format_args!(
            "let (built, _rows) = {name}_rebuild(theme, build, component, &[])?;"
        ));
        out.line(format_args!("bindings.extend(built.bindings);"));
        out.line(format_args!("handlers.extend(built.handlers);"));
        out.line(format_args!("props.extend(built.props);"));
        out.line(format_args!("let {widget} = built.widget;"));
        out.line(format_args!(
            "brul::register_structure(build, component, {widget}.id(), {each}, {value}, {name}_rebuild);"
        ));
        out.line(format_args!("{widget}"));
    });
    out.start(format_args!("}}"));
}

fn rebuild_arm(
    name: &str,
    block: &IfBlock,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
) -> Result<String, Error> {
    let arm = arm_frame(emit.default_theme(), frame);
    let mut text = String::new();
    let mut out = Out::new(&mut text);
    arm_header(&mut out, name);
    out.indented(|out| -> Result<(), Error> {
        out.line(format_args!("let children = {{"));
        let mut counter = 0;
        out.indented(|out| branch_chain(out, name, block, emit, arm, &mut counter, 0))?;
        out.line(format_args!("}};"));
        Ok(())
    })?;
    widget(&mut out);
    Ok(text)
}

fn bind_value(out: &mut Out<'_>, binding: &str, call: &str) {
    out.line(format_args!(
        "let {binding} = {call}(&mut component.borrow_mut())"
    ));
    out.indented(|out| out.line(format_args!(".map_err(brul::ViewError::from)?;")));
}

fn unwrap_value(
    out: &mut Out<'_>,
    source: &str,
    binding: &str,
    variant: &str,
    expected: &str,
    transform: &str,
) {
    out.line(format_args!("let {binding} = match {source} {{"));
    out.indented(|out| {
        out.line(format_args!(
            "brul::Value::{variant}({binding}) => {transform},"
        ));
        out.line(format_args!(
            "other => return Err(brul::ViewError::expected(\"{expected}\", \"{variant}\", other.type_name())),"
        ));
    });
    out.line(format_args!("}};"));
}

fn branch_chain(
    out: &mut Out<'_>,
    name: &str,
    block: &IfBlock,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
    index: usize,
) -> Result<(), Error> {
    bind_value(out, "condition", &format!("{name}_value{index}"));
    unwrap_value(
        out,
        "condition",
        "condition",
        "Bool",
        "if condition",
        "condition",
    );
    out.line(format_args!("if condition {{"));
    out.indented(|out| arm_body(out, &block.branches[index].block, emit, frame, counter))?;
    out.line(format_args!("}} else {{"));
    out.indented(|out| -> Result<(), Error> {
        if index + 1 < block.branches.len() {
            branch_chain(out, name, block, emit, frame, counter, index + 1)?;
        } else if let Some(else_block) = &block.else_block {
            arm_body(out, else_block, emit, frame, counter)?;
        } else {
            out.line(format_args!("vec![]"));
        }
        Ok(())
    })?;
    out.line(format_args!("}}"));
    Ok(())
}

fn arm_body(
    out: &mut Out<'_>,
    block: &Block,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
) -> Result<(), Error> {
    if contains_slot(block) {
        out.line(format_args!("{{"));
        out.indented(|out| -> Result<(), Error> {
            out.line(format_args!("let mut children = Vec::new();"));
            items_spliced(out, block, emit, frame, counter)?;
            out.line(format_args!("children"));
            Ok(())
        })?;
        out.line(format_args!("}}"));
    } else {
        out.line(format_args!("vec!["));
        out.indented(|out| items(out, block, emit, frame, counter))?;
        out.line(format_args!("]"));
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
    let mut text = String::new();
    let mut out = Out::new(&mut text);
    arm_header(&mut out, name);
    out.indented(|out| -> Result<(), Error> {
        each_collection(out, name);
        out.line(format_args!(
            "let mut matcher = brul::RowMatcher::new(old_rows);"
        ));
        each_else(out, block, emit, arm)?;
        out.line(format_args!(
            "for (position, item) in items.into_iter().enumerate() {{"
        ));
        out.indented(|out| each_row(out, name, block, emit, arm))?;
        out.line(format_args!("}}"));
        Ok(())
    })?;
    widget(&mut out);
    Ok(text)
}

fn each_collection(out: &mut Out<'_>, name: &str) {
    out.line(format_args!("let mut children = Vec::new();"));
    bind_value(out, "collection", &format!("{name}_value"));
    unwrap_value(
        out,
        "collection",
        "items",
        "Array",
        "each",
        "std::sync::Arc::unwrap_or_clone(items)",
    );
}

fn each_else(
    out: &mut Out<'_>,
    block: &EachBlock,
    emit: &mut Emit<'_>,
    arm: Frame<'_>,
) -> Result<(), Error> {
    let Some(else_block) = &block.else_block else {
        return Ok(());
    };
    out.line(format_args!("if items.is_empty() {{"));
    out.indented(|out| -> Result<(), Error> {
        each_body(out, else_block, emit, arm)?;
        out.line(format_args!("rows.push(brul::RowInfo {{"));
        out.indented(|out| {
            out.line(format_args!("key: brul::RowKey::Index(0),"));
            out.line(format_args!(
                "item: brul::Value::Array(std::sync::Arc::new(Vec::new())),"
            ));
            out.line(format_args!("start: 0,"));
            out.line(format_args!("count: children.len(),"));
            out.line(format_args!("reuse: None,"));
        });
        out.line(format_args!("}});"));
        Ok(())
    })?;
    out.line(format_args!("}}"));
    Ok(())
}

fn each_row(
    out: &mut Out<'_>,
    name: &str,
    block: &EachBlock,
    emit: &mut Emit<'_>,
    arm: Frame<'_>,
) -> Result<(), Error> {
    each_scope(out, block, emit);
    each_key(out, name, block);
    out.line(format_args!("let start = children.len();"));
    each_take(out);
    each_body(out, &block.block, emit, arm)?;
    out.line(format_args!("let count = children.len() - start;"));
    out.line(format_args!("component.borrow_mut().pop_build_scope();"));
    out.line(format_args!(
        "rows.push(brul::RowInfo {{ key, item, start, count, reuse: None }});"
    ));
    Ok(())
}

fn each_scope(out: &mut Out<'_>, block: &EachBlock, emit: &mut Emit<'_>) {
    out.line(format_args!(
        "component.borrow_mut().push_build_scope(vec!["
    ));
    out.indented(|out| {
        out.line(format_args!(
            "brul::Variable {{ name: brul::VariableId::from_index({}), value: item.clone() }},",
            variable(block.item.as_str(), emit.names)
        ));
        if let Some(index_name) = &block.index {
            out.line(format_args!(
                "brul::Variable {{ name: brul::VariableId::from_index({}), value: brul::Value::Number(position as f64) }},",
                variable(index_name.as_str(), emit.names)
            ));
        }
    });
    out.line(format_args!("]);"));
}

fn each_key(out: &mut Out<'_>, name: &str, block: &EachBlock) {
    if block.key.is_some() {
        out.line(format_args!(
            "let key = {name}_key(&mut component.borrow_mut()).map(brul::RowKey::Explicit).unwrap_or(brul::RowKey::Index(position));"
        ));
    } else {
        out.line(format_args!("let key = brul::RowKey::Index(position);"));
    }
}

fn each_take(out: &mut Out<'_>) {
    out.line(format_args!(
        "if let Some(other) = matcher.take(&key, &item, position) {{"
    ));
    out.indented(|out| {
        out.line(format_args!(
            "rows.push(brul::RowInfo {{ key, item, start, count: 0, reuse: Some(other) }});"
        ));
        out.line(format_args!("component.borrow_mut().pop_build_scope();"));
        out.line(format_args!("continue;"));
    });
    out.line(format_args!("}}"));
}

fn each_body(
    out: &mut Out<'_>,
    block: &Block,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
) -> Result<(), Error> {
    let mut counter = 0;
    if contains_slot(block) {
        out.indented(|out| items_spliced(out, block, emit, frame, &mut counter))?;
    } else if !block.items.is_empty() {
        out.line(format_args!("children.extend(["));
        out.indented(|out| items(out, block, emit, frame, &mut counter))?;
        out.line(format_args!("]);"));
    }
    Ok(())
}

fn arm_frame<'a>(theme: Option<&'a str>, frame: Frame<'a>) -> Frame<'a> {
    Frame::root(theme).inside(frame.direction(), frame.gap())
}

fn variable(name: &str, names: &Names<'_>) -> u32 {
    names
        .get(name)
        .expect("each variable is collected into the name table")
        .index()
}

fn arm_header(out: &mut Out<'_>, name: &str) {
    out.line(format_args!(
        "#[allow(unused_mut, unused_variables, clippy::let_and_return)]"
    ));
    out.line(format_args!(
        "fn {name}_rebuild(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle, old_rows: &[brul::RowInfo]) -> brul::Rebuilt {{"
    ));
    out.indented(|out| {
        for field in ["bindings", "handlers", "props", "rows"] {
            out.line(format_args!("let mut {field} = Vec::new();"));
        }
    });
}

fn widget(out: &mut Out<'_>) {
    out.indented(|out| {
        out.line(format_args!(
            "let widget = brul::gui::Widget::structure(children);"
        ));
        out.line(format_args!(
            "Ok((brul::Built {{ widget, bindings, handlers, props }}, rows))"
        ));
    });
    out.line(format_args!("}}"));
}
