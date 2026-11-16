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
    inline(out, &name, &format!("{name}_value0"), false, frame, counter);
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
    inline(out, &name, &format!("{name}_value"), true, frame, counter);
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

fn inline(
    out: &mut Out<'_>,
    name: &str,
    value: &str,
    each: bool,
    frame: Frame<'_>,
    counter: &mut usize,
) {
    let mut out = Out::at(out.raw(), frame.depth());
    let widget = format!("widget{}", *counter);
    *counter += 1;
    out.line(format_args!("{{"));
    out.indented(|out| {
        out.line(format_args!(
            "let (built, _rows) = {name}_rebuild(theme, component)?;"
        ));
        out.line(format_args!("bindings.extend(built.bindings);"));
        out.line(format_args!("handlers.extend(built.handlers);"));
        out.line(format_args!("props.extend(built.props);"));
        out.line(format_args!("let {widget} = built.widget;"));
        out.line(format_args!(
            "brul::register_structure(component, {widget}.id(), {each}, {value}, {name}_rebuild);"
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

fn branch_chain(
    out: &mut Out<'_>,
    name: &str,
    block: &IfBlock,
    emit: &mut Emit<'_>,
    frame: Frame<'_>,
    counter: &mut usize,
    index: usize,
) -> Result<(), Error> {
    let body = frame.deeper(index + 1);
    out.line(format_args!(
        "let condition = {name}_value{index}(&mut component.borrow_mut())"
    ));
    out.indented(|out| out.line(format_args!(".map_err(brul::ViewError::from)?;")));
    out.line(format_args!("let condition = match condition {{"));
    out.indented(|out| {
        out.line(format_args!("brul::Value::Bool(condition) => condition,"));
        out.line(format_args!(
            "other => return Err(brul::ViewError::expected(\"if condition\", \"Bool\", other.type_name())),"
        ));
    });
    out.line(format_args!("}};"));
    out.line(format_args!("if condition {{"));
    arm_body(out, &block.branches[index].block, emit, body, counter)?;
    out.line(format_args!("}} else {{"));
    out.indented(|out| -> Result<(), Error> {
        if index + 1 < block.branches.len() {
            branch_chain(out, name, block, emit, frame, counter, index + 1)?;
        } else if let Some(else_block) = &block.else_block {
            arm_body(out, else_block, emit, body, counter)?;
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
    let mut out = Out::at(out.raw(), frame.depth().saturating_sub(1));
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
        out.line(format_args!("let mut children = Vec::new();"));
        out.line(format_args!(
            "let collection = {name}_value(&mut component.borrow_mut())"
        ));
        out.indented(|out| out.line(format_args!(".map_err(brul::ViewError::from)?;")));
        out.line(format_args!("let items = match collection {{"));
        out.indented(|out| {
            out.line(format_args!(
                "brul::Value::Array(items) => std::rc::Rc::unwrap_or_clone(items),"
            ));
            out.line(format_args!(
                "other => return Err(brul::ViewError::expected(\"each\", \"Array\", other.type_name())),"
            ));
        });
        out.line(format_args!("}};"));
        if let Some(else_block) = &block.else_block {
            out.line(format_args!("if items.is_empty() {{"));
            out.indented(|out| -> Result<(), Error> {
                each_body(out, else_block, emit, arm)?;
                out.line(format_args!("rows.push(brul::RowInfo {{"));
                out.indented(|out| {
                    out.line(format_args!("key: None,"));
                    out.line(format_args!(
                        "item: brul::Value::Array(std::rc::Rc::new(Vec::new())),"
                    ));
                    out.line(format_args!("start: 0,"));
                    out.line(format_args!("count: children.len(),"));
                });
                out.line(format_args!("}});"));
                Ok(())
            })?;
            out.line(format_args!("}}"));
        }
        out.line(format_args!(
            "for (position, item) in items.into_iter().enumerate() {{"
        ));
        out.indented(|out| -> Result<(), Error> {
            out.line(format_args!("component.borrow_mut().push_build_scope(vec!["));
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
            match &block.key {
                Some(_) => out.line(format_args!(
                    "let key = {name}_key(&mut component.borrow_mut()).ok();"
                )),
                None => out.line(format_args!("let key = None;")),
            }
            out.line(format_args!("let start = children.len();"));
            each_body(out, &block.block, emit, arm)?;
            out.line(format_args!("let count = children.len() - start;"));
            out.line(format_args!("component.borrow_mut().pop_build_scope();"));
            out.line(format_args!(
                "rows.push(brul::RowInfo {{ key, item, start, count }});"
            ));
            Ok(())
        })?;
        out.line(format_args!("}}"));
        Ok(())
    })?;
    widget(&mut out);
    Ok(text)
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
    Frame::root(theme, 3).inside(frame.direction(), frame.gap())
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
        "fn {name}_rebuild(theme: Option<&str>, component: &brul::ComponentHandle) -> brul::Rebuilt {{"
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
