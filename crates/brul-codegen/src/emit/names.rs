use brul_script::{Names, VariableId};
use brul_syntax::ast::{Expr, ExprKind, Position};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Block, BlockItem, Element, Member, Prop};
use brul_syntax::script::{Function, Program, Statement};

use crate::{Definition, Source, error_at};

pub(super) fn table<'a>(source: &'a Source, definitions: &'a [Definition]) -> Names<'a> {
    let mut names = Names::new();
    if let Some(declaration) = &source.declaration {
        members(&declaration.members, &mut names);
    }
    if let Some(program) = &source.program {
        script(program, &mut names);
    }
    for definition in definitions {
        members(&definition.declaration.members, &mut names);
        if let Some(program) = &definition.program {
            script(program, &mut names);
        }
    }
    names
}

pub(super) fn declared(members: &[Member]) -> Vec<&str> {
    members
        .iter()
        .filter_map(|member| match member {
            Member::Prop(prop) => Some(prop.name.as_str()),
            _ => None,
        })
        .collect()
}

pub(super) fn declared_events(members: &[Member]) -> Vec<&str> {
    members
        .iter()
        .filter_map(|member| match member {
            Member::Event(event) => Some(event.name.as_str()),
            _ => None,
        })
        .collect()
}

pub(super) fn output(names: &Names<'_>) -> String {
    let names = names
        .names()
        .iter()
        .map(|name| format!("{name:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("\npub fn names() -> &'static [&'static str] {{\n    &[{names}]\n}}\n")
}

pub(super) fn variable(
    name: &str,
    position: Position,
    names: &Names<'_>,
) -> Result<VariableId, Error> {
    names.get(name).ok_or_else(|| {
        error_at(
            position,
            format!("identifier '{name}' is not collected into the name table"),
        )
    })
}

pub(super) fn variable_expr(
    name: &str,
    position: Position,
    names: &Names<'_>,
) -> Result<String, Error> {
    Ok(format!(
        "brul::VariableId::from_index({})",
        variable(name, position, names)?.index()
    ))
}

fn members<'a>(members: &'a [Member], names: &mut Names<'a>) {
    for member in members {
        match member {
            Member::Prop(prop) => prop_node(prop, names),
            Member::Event(_) => {}
            Member::Element(element) => element_node(element, names),
        }
    }
}

fn prop_node<'a>(prop: &'a Prop, names: &mut Names<'a>) {
    names.intern(&prop.name);
    if let Some(default) = &prop.default {
        expression(default, names);
    }
}

fn element_node<'a>(element: &'a Element, names: &mut Names<'a>) {
    for arg in &element.args {
        if let Arg::Named { value, .. } = arg {
            expression(value, names);
        }
    }
    block_node(&element.block, names);
}

fn block_node<'a>(block: &'a Block, names: &mut Names<'a>) {
    for item in &block.items {
        match item {
            BlockItem::Element(element) => element_node(element, names),
            BlockItem::If(block) => {
                for branch in &block.branches {
                    expression(&branch.condition, names);
                    block_node(&branch.block, names);
                }
                if let Some(else_block) = &block.else_block {
                    block_node(else_block, names);
                }
            }
            BlockItem::Each(each) => {
                expression(&each.collection, names);
                names.intern(&each.item);
                if let Some(index) = &each.index {
                    names.intern(index);
                }
                if let Some(key) = &each.key {
                    expression(key, names);
                }
                block_node(&each.block, names);
                if let Some(else_block) = &each.else_block {
                    block_node(else_block, names);
                }
            }
            BlockItem::Expr(expr) => expression(expr, names),
            BlockItem::Slot(slot) => {
                if let Some(block) = &slot.block {
                    block_node(block, names);
                }
            }
        }
    }
}

fn script<'a>(program: &'a Program, names: &mut Names<'a>) {
    for function in &program.functions {
        function_node(function, names);
    }
    statements(&program.statements, names);
}

fn function_node<'a>(function: &'a Function, names: &mut Names<'a>) {
    for param in &function.params {
        names.intern(&param.name);
    }
    statements(&function.body, names);
}

fn statements<'a>(statements: &'a [Statement], names: &mut Names<'a>) {
    for statement in statements {
        statement_node(statement, names);
    }
}

fn statement_node<'a>(statement: &'a Statement, names: &mut Names<'a>) {
    match statement {
        Statement::Let { name, value, .. } => {
            names.intern(name);
            if let Some(value) = value {
                expression(value, names);
            }
        }
        Statement::Expr(expr) => expression(expr, names),
        Statement::Block(body) => statements(body, names),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expression(condition, names);
            statements(then_branch, names);
            if let Some(else_branch) = else_branch {
                statement_node(else_branch, names);
            }
        }
        Statement::Return { value, .. } => {
            if let Some(value) = value {
                expression(value, names);
            }
        }
    }
}

pub(super) fn expression<'a>(expr: &'a Expr, names: &mut Names<'a>) {
    match &expr.kind {
        ExprKind::Ident(name) => {
            names.intern(name);
        }
        ExprKind::Call { target, args } => call(target, args, names),
        _ => {
            for child in expr.kind.children() {
                expression(child, names);
            }
        }
    }
}

fn call<'a>(target: &'a Expr, args: &'a [Expr], names: &mut Names<'a>) {
    if let ExprKind::Ident(name) = &target.kind {
        let timer = name == "every" || name == "after";
        for (index, argument) in args.iter().enumerate() {
            if timer && index == 1 {
                continue;
            }
            expression(argument, names);
        }
    } else {
        expression(target, names);
        for argument in args {
            expression(argument, names);
        }
    }
}

#[cfg(test)]
mod tests {
    use brul_syntax::ast::Position;

    use super::variable;

    #[test]
    fn variable_reports_missing_identifier_at_position() {
        let names = brul_script::Names::new();
        let error = variable("item", Position { line: 7, column: 3 }, &names).unwrap_err();
        assert_eq!(
            error.message(),
            "identifier 'item' is not collected into the name table"
        );
        assert_eq!((error.line(), error.column()), (7, 3));
    }
}
