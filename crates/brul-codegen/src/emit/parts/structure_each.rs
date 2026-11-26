use brul_syntax::error::Error;
use brul_syntax::markup::{Block, EachBlock};

use super::Emit;
use super::Frame;
use super::Out;
use super::{contains_slot, items, items_spliced};

use super::structure::{bind_value, each_scope};

pub(super) fn each_arm(
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

pub(super) fn each_collection(out: &mut Out<'_>, name: &str) {
    out.line(format_args!("let mut children = Vec::new();"));
    bind_value(out, "collection", &format!("{name}_value"));
    out.line(format_args!("let items = collection;"));
}

pub(super) fn each_else(
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
            out.line(format_args!("item: brul::Dyn::array(vec![]).into(),"));
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

pub(super) fn each_row(
    out: &mut Out<'_>,
    name: &str,
    block: &EachBlock,
    emit: &mut Emit<'_>,
    arm: Frame<'_>,
) -> Result<(), Error> {
    let info = arm
        .scope
        .and_then(|index| emit.scope_def(index))
        .cloned()
        .expect("each scope definition");
    each_scope(out, &info, emit)?;
    each_key(out, name, block);
    out.line(format_args!("let start = children.len();"));
    each_take(out);
    each_body(out, &block.block, emit, arm)?;
    out.line(format_args!("let count = children.len() - start;"));
    out.line(format_args!(
        "component.borrow_mut().set_scope(previous_scope);"
    ));
    out.line(format_args!(
        "rows.push(brul::RowInfo {{ key, item, start, count, reuse: None }});"
    ));
    Ok(())
}

pub(super) fn each_key(out: &mut Out<'_>, name: &str, block: &EachBlock) {
    if block.key.is_some() {
        out.line(format_args!(
            "let key = {name}_key(&mut component.borrow_mut()).map(|key| brul::RowKey::Explicit(key.into())).unwrap_or(brul::RowKey::Index(position));"
        ));
    } else {
        out.line(format_args!("let key = brul::RowKey::Index(position);"));
    }
}

pub(super) fn each_take(out: &mut Out<'_>) {
    out.line(format_args!(
        "if let Some(other) = matcher.take(&key, &item, position) {{"
    ));
    out.indented(|out| {
        out.line(format_args!(
            "rows.push(brul::RowInfo {{ key, item, start, count: 0, reuse: Some(other) }});"
        ));
        out.line(format_args!(
            "component.borrow_mut().set_scope(previous_scope);"
        ));
        out.line(format_args!("continue;"));
    });
    out.line(format_args!("}}"));
}

pub(super) fn each_body(
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

pub(super) fn arm_frame<'a>(theme: Option<&'a str>, frame: Frame<'a>) -> Frame<'a> {
    let mut arm = Frame::root(theme).inside(frame.direction(), frame.row_gap(), frame.column_gap());
    arm.scope = frame.scope;
    arm
}

pub(super) fn arm_header(out: &mut Out<'_>, name: &str) {
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

pub(super) fn widget(out: &mut Out<'_>) {
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
