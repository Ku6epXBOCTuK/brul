use std::collections::HashMap;
use std::fmt::Write;

use brul_syntax::TypeName;
use brul_syntax::error::Error;
use brul_syntax::markup::{Block, EachBlock, IfBlock};
use brul_syntax::types::Type;

use super::Emit;
use super::Frame;
use super::Out;
use super::PropTypes;

use super::expression::{operation, typed};
use super::scopes::{ScopeField, ScopeInfo, Scopes};
use super::state::State;
use super::{contains_slot, items, items_spliced};

use brul_script::Names;

#[derive(Clone, Copy)]
enum ValueKind {
    Bool,
    Items,
    Inferred,
}

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
        check_condition(&branch.condition, emit.names, emit.state, emit.prop_types)?;
        let function = format!("{name}_value{index}");
        emit.functions.push(value_function(
            &function,
            &branch.condition,
            ValueKind::Bool,
            emit.names,
            emit.state,
            emit.prop_types,
        )?);
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
    check_collection(&block.collection, emit.names, emit.state, emit.prop_types)?;
    emit.functions.push(value_function(
        &format!("{name}_value"),
        &block.collection,
        ValueKind::Items,
        emit.names,
        emit.state,
        emit.prop_types,
    )?);
    if let Some(key) = &block.key {
        emit.functions.push(value_function(
            &format!("{name}_key"),
            key,
            ValueKind::Inferred,
            emit.names,
            emit.state,
            emit.prop_types,
        )?);
    }
    let mut fields = vec![ScopeField {
        name: block.item.clone(),
        field: "item".to_owned(),
        ty: emit.item_type(&block.collection),
    }];
    if let Some(index) = &block.index {
        fields.push(ScopeField {
            name: index.clone(),
            field: "index".to_owned(),
            ty: brul_syntax::types::Type::Number,
        });
    }
    if let Some(outer) = frame.scope.and_then(|index| emit.scope_def(index)) {
        for field in &outer.fields {
            fields.push(ScopeField {
                name: field.name.clone(),
                field: format!("outer_{}", field.field),
                ty: field.ty.clone(),
            });
        }
    }
    let info = ScopeInfo {
        struct_name: format!("{name}_scope"),
        outer: frame.scope,
        fields,
    };
    emit.functions.push(scope_struct(&info));
    let scope = emit.add_scope(info);
    let mut body_frame = frame;
    body_frame.scope = Some(scope);
    let arm = each_arm(&name, block, emit, body_frame)?;
    emit.functions.push(arm);
    inline(out, &name, &format!("{name}_value"), true, counter);
    Ok(())
}

fn check_condition(
    condition: &brul_syntax::ast::Expr,
    names: &Names<'_>,
    state: &State,
    props: &PropTypes,
) -> Result<(), Error> {
    if let Some((_, ty)) = typed(condition, names, state, props, &Scopes::new())?
        && ty != Type::Bool
    {
        return Err(crate::error_at(
            condition.position,
            format!("if condition expects Bool, found {}", ty.type_name()),
        ));
    }
    Ok(())
}

fn check_collection(
    collection: &brul_syntax::ast::Expr,
    names: &Names<'_>,
    state: &State,
    props: &PropTypes,
) -> Result<(), Error> {
    if let Some((_, ty)) = typed(collection, names, state, props, &Scopes::new())?
        && !matches!(ty, Type::Array(_) | Type::Object)
    {
        return Err(crate::error_at(
            collection.position,
            format!("each expects Array or Object, found {}", ty.type_name()),
        ));
    }
    Ok(())
}

fn value_function(
    name: &str,
    value: &brul_syntax::ast::Expr,
    kind: ValueKind,
    names: &Names<'_>,
    state: &super::state::State,
    props: &super::PropTypes,
) -> Result<String, Error> {
    let inferred = typed(value, names, state, props, &Scopes::new())?;
    let (return_type, body) = match kind {
        ValueKind::Bool => match inferred {
            Some((code, Type::Bool)) => ("bool", code),
            _ => (
                "bool",
                format!("(&{}).try_into()?", operation(value, names, state, props)?),
            ),
        },
        ValueKind::Items => (
            "brul::Items",
            format!(
                "brul::items(&({}))",
                match inferred {
                    Some((code, _)) => code,
                    None => operation(value, names, state, props)?,
                }
            ),
        ),
        ValueKind::Inferred => match inferred {
            Some((code, ty)) => (Scopes::local_type(&ty), code),
            None => ("brul::Dyn", operation(value, names, state, props)?),
        },
    };
    let mut text = String::new();
    let mut out = Out::new(&mut text);
    out.line(format_args!(
        "fn {name}(component: &mut brul::Component) -> Result<{return_type}, brul::Error> {{"
    ));
    out.indented(|out| out.line(format_args!("Ok({body})")));
    out.line(format_args!("}}"));
    Ok(text)
}

fn scope_struct(info: &ScopeInfo) -> String {
    let mut text = String::new();
    let _ = writeln!(
        text,
        "\n#[derive(Clone)]\n#[allow(dead_code)]\nstruct {} {{",
        info.struct_name
    );
    for field in &info.fields {
        let _ = writeln!(
            text,
            "    {}: {},",
            field.field,
            Scopes::local_type(&field.ty)
        );
    }
    text.push_str("}\n");
    text
}

fn scope_default(ty: &brul_syntax::types::Type) -> &'static str {
    match ty {
        brul_syntax::types::Type::Number => "0.0",
        brul_syntax::types::Type::String => "String::new()",
        brul_syntax::types::Type::Bool => "false",
        brul_syntax::types::Type::Null => "()",
        _ => "brul::Dyn::null()",
    }
}

fn each_scope(out: &mut Out<'_>, info: &ScopeInfo, emit: &Emit<'_>) -> Result<(), Error> {
    out.line(format_args!(
        "let previous_scope = component.borrow().scope().clone();"
    ));
    let mut values = Vec::new();
    for (index, field) in info.fields.iter().enumerate() {
        let value = if index == 0 {
            "item.clone()".to_owned()
        } else if field.field == "index" {
            "position as f64".to_owned()
        } else if let Some(outer) = info
            .outer
            .and_then(|index| emit.scope_def(index))
            .map(|info| info.struct_name.as_str())
        {
            let source = field.field.strip_prefix("outer_").unwrap_or(&field.field);
            let default = scope_default(&field.ty);
            format!(
                "component.borrow().scope().get::<{outer}>().map(|value| value.{source}.clone()).unwrap_or_else(|| {default})"
            )
        } else {
            scope_default(&field.ty).to_owned()
        };
        values.push(format!("{}: {value}", field.field));
    }
    out.line(format_args!(
        "component.borrow_mut().set_scope(brul::Scope::each({} {{",
        info.struct_name
    ));
    out.indented(|out| {
        for value in values {
            out.line(format_args!("{value},"));
        }
        Ok::<(), Error>(())
    })?;
    out.line(format_args!("}}));"));
    Ok(())
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
    out.line(format_args!("let items = collection;"));
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

fn each_row(
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

fn each_key(out: &mut Out<'_>, name: &str, block: &EachBlock) {
    if block.key.is_some() {
        out.line(format_args!(
            "let key = {name}_key(&mut component.borrow_mut()).map(|key| brul::RowKey::Explicit(key.into())).unwrap_or(brul::RowKey::Index(position));"
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
        out.line(format_args!(
            "component.borrow_mut().set_scope(previous_scope);"
        ));
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
    let mut arm = Frame::root(theme).inside(frame.direction(), frame.gap());
    arm.scope = frame.scope;
    arm
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
