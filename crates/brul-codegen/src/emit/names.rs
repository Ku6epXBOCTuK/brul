use brul_script::Names;
use brul_syntax::ast::{Expr, ExprKind, TemplatePart};
use brul_syntax::markup::{Arg, Block, BlockItem, Element, Member, Prop, Window};
use brul_syntax::script::{Function, Program, Statement};

use crate::{Definition, Source};

pub(super) fn table<'a>(source: &'a Source, definitions: &'a [Definition]) -> Names<'a> {
    let mut names = Names::new();
    window(&source.window, &mut names);
    if let Some(program) = &source.program {
        script(program, &mut names);
    }
    for definition in definitions {
        members(&definition.component.members, &mut names);
        if let Some(program) = &definition.program {
            script(program, &mut names);
        }
    }
    names
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

fn window<'a>(window: &'a Window, names: &mut Names<'a>) {
    members(&window.members, names);
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
                expression(&block.condition, names);
                block_node(&block.then_block, names);
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
            }
            BlockItem::Expr(expr) => expression(expr, names),
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
        names.intern(param);
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
    }
}

fn expression<'a>(expr: &'a Expr, names: &mut Names<'a>) {
    match &expr.kind {
        ExprKind::Ident(name) => {
            names.intern(name);
        }
        ExprKind::Field { target, .. } => expression(target, names),
        ExprKind::Index { target, index } => {
            expression(target, names);
            expression(index, names);
        }
        ExprKind::Call { target, args } => call(target, args, names),
        ExprKind::Unary { operand, .. } => expression(operand, names),
        ExprKind::Binary { left, right, .. } => {
            expression(left, names);
            expression(right, names);
        }
        ExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            expression(condition, names);
            expression(then_branch, names);
            expression(else_branch, names);
        }
        ExprKind::Assign { target, value, .. } => {
            expression(target, names);
            expression(value, names);
        }
        ExprKind::Array(items) => {
            for item in items {
                expression(item, names);
            }
        }
        ExprKind::Object(fields) => {
            for (_, value) in fields {
                expression(value, names);
            }
        }
        ExprKind::Template(parts) => {
            for part in parts {
                if let TemplatePart::Expr(expr) = part {
                    expression(expr, names);
                }
            }
        }
        ExprKind::Number(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::Null => {}
    }
}

fn call<'a>(target: &'a Expr, args: &'a [Expr], names: &mut Names<'a>) {
    if let ExprKind::Ident(name) = &target.kind {
        let every = name == "every";
        for (index, argument) in args.iter().enumerate() {
            if every && index == 1 {
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
