use std::fmt::Write;

use brul_syntax::script::{Function, Program, Statement};

use super::expression;

pub(super) fn program(name: &str, program: Option<&Program>) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "\npub fn {name}() -> Option<brul::script::Program> {{");
    match program {
        None => out.push_str("    None\n"),
        Some(program) => {
            out.push_str("    Some(brul::script::Program {\n        functions: vec![\n");
            for function in &program.functions {
                let _ = writeln!(out, "            {},", self::function(function));
            }
            out.push_str("        ],\n        statements: vec![\n");
            for statement in &program.statements {
                let _ = writeln!(out, "            {},", self::statement(statement));
            }
            out.push_str("        ],\n    })\n");
        }
    }
    out.push_str("}\n");
    out
}

fn function(function: &Function) -> String {
    let params: Vec<String> = function
        .params
        .iter()
        .map(|name| format!("{name:?}.to_owned()"))
        .collect();
    format!(
        "brul::script::Function {{ name: {:?}.to_owned(), params: vec![{}], body: {} }}",
        function.name,
        params.join(", "),
        statements(&function.body)
    )
}

fn statement(statement: &Statement) -> String {
    match statement {
        Statement::Let {
            constant,
            name,
            value,
        } => {
            let value = value.as_ref().map_or_else(
                || "None".to_owned(),
                |value| format!("Some({})", expression(value)),
            );
            format!(
                "brul::script::Statement::Let {{ constant: {constant}, name: {name:?}.to_owned(), value: {value} }}"
            )
        }
        Statement::Expr(value) => {
            format!("brul::script::Statement::Expr({})", expression(value))
        }
        Statement::Block(body) => {
            format!("brul::script::Statement::Block({})", statements(body))
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let else_branch = else_branch.as_ref().map_or_else(
                || "None".to_owned(),
                |branch| format!("Some(Box::new({}))", self::statement(branch)),
            );
            format!(
                "brul::script::Statement::If {{ condition: {}, then_branch: {}, else_branch: {} }}",
                expression(condition),
                statements(then_branch),
                else_branch
            )
        }
    }
}

fn statements(body: &[Statement]) -> String {
    let items: Vec<String> = body.iter().map(statement).collect();
    format!("vec![{}]", items.join(", "))
}
