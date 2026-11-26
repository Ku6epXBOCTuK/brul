use std::collections::HashMap;
use std::fmt::Write;

use brul_syntax::TypeName;
use brul_syntax::error::Error;
use brul_syntax::markup::{Block, EachBlock, IfBlock};
use brul_syntax::types::Type;

use super::Emit;
use super::ExprContext;
use super::Frame;
use super::Out;

use super::expression::{operation_in, typed};
use super::scopes::{ScopeField, ScopeInfo, Scopes};
use super::{contains_slot, items, items_spliced};

use super::structure_each::{arm_frame, arm_header, each_arm, widget};

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
    let name = emit.structures.next(emit.context.base, "if");
    for (index, branch) in block.branches.iter().enumerate() {
        check_condition(&branch.condition, &emit.expr_context())?;
        let function = format!("{name}_value{index}");
        emit.functions.push(value_function(
            &function,
            &branch.condition,
            ValueKind::Bool,
            &emit.expr_context(),
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
    let name = emit.structures.next(emit.context.base, "each");
    check_collection(&block.collection, &emit.expr_context())?;
    emit.functions.push(value_function(
        &format!("{name}_value"),
        &block.collection,
        ValueKind::Items,
        &emit.expr_context(),
    )?);
    if let Some(key) = &block.key {
        emit.functions.push(value_function(
            &format!("{name}_key"),
            key,
            ValueKind::Inferred,
            &emit.expr_context(),
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
    context: &ExprContext<'_>,
) -> Result<(), Error> {
    if let Some((_, ty)) = typed(condition, &context.with_scopes(Scopes::new()))?
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
    context: &ExprContext<'_>,
) -> Result<(), Error> {
    if let Some((_, ty)) = typed(collection, &context.with_scopes(Scopes::new()))?
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
    context: &ExprContext<'_>,
) -> Result<String, Error> {
    let scoped = context.with_scopes(Scopes::new());
    let inferred = typed(value, &scoped)?;
    let (return_type, body) = match kind {
        ValueKind::Bool => match inferred {
            Some((code, Type::Bool)) => ("bool", code),
            _ => (
                "bool",
                format!("(&{}).try_into()?", operation_in(value, &scoped)?),
            ),
        },
        ValueKind::Items => (
            "brul::Items",
            format!(
                "brul::items(&({}))",
                match inferred {
                    Some((code, _)) => code,
                    None => operation_in(value, &scoped)?,
                }
            ),
        ),
        ValueKind::Inferred => match inferred {
            Some((code, ty)) => (Scopes::local_type(&ty), code),
            None => ("brul::Dyn", operation_in(value, &scoped)?),
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
    ty.scope_default()
}

pub(super) fn each_scope(
    out: &mut Out<'_>,
    info: &ScopeInfo,
    emit: &Emit<'_>,
) -> Result<(), Error> {
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

pub(super) fn bind_value(out: &mut Out<'_>, binding: &str, call: &str) {
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
