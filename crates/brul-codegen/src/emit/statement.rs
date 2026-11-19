use std::fmt::Write;

use brul_script::Names;
use brul_syntax::error::Error;
use brul_syntax::script::Statement;

use super::expression::operation;
use super::state::State;

pub(super) fn block(
    statements: &[Statement],
    names: &Names<'_>,
    state: &State,
    out: &mut String,
) -> Result<(), Error> {
    block_with(statements, names, state, out, false)
}

pub(super) fn top_level(
    statements: &[Statement],
    names: &Names<'_>,
    state: &State,
    out: &mut String,
) -> Result<(), Error> {
    block_with(statements, names, state, out, true)
}

fn block_with(
    statements: &[Statement],
    names: &Names<'_>,
    state: &State,
    out: &mut String,
    top_level: bool,
) -> Result<(), Error> {
    for statement in statements {
        emit_statement(statement, names, state, out, top_level)?;
    }
    Ok(())
}

fn emit_statement(
    statement: &Statement,
    names: &Names<'_>,
    state: &State,
    out: &mut String,
    top_level: bool,
) -> Result<(), Error> {
    match statement {
        Statement::Let {
            name,
            value,
            constant,
            ..
        } => {
            let null = "brul::Value::Null".to_owned();
            let (method, value) = match value {
                Some(value) => ("declare", operation(value, names, state)?),
                None if top_level => ("declare_if_absent", null),
                None => ("declare", null),
            };
            let _ = writeln!(
                out,
                "    component.{method}({name:?}, {value}, {constant})?;"
            );
        }
        Statement::Expr(expression) => {
            let _ = writeln!(out, "    {};", operation(expression, names, state)?);
        }
        Statement::Block(statements) => {
            out.push_str("    {\n");
            block_with(statements, names, state, out, false)?;
            out.push_str("    }\n");
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let value = operation(condition, names, state)?;
            let _ = writeln!(out, "    match {value} {{");
            let _ = writeln!(out, "        brul::Value::Bool(true) => {{");
            block_with(then_branch, names, state, out, false)?;
            out.push_str("        }\n");
            if let Some(branch) = else_branch {
                out.push_str("        brul::Value::Bool(false) => {\n");
                emit_statement(branch, names, state, out, false)?;
                out.push_str("        }\n");
            }
            let _ = writeln!(
                out,
                "        other => return Err(brul::Error::Expected {{ operation: \"if\", expected: \"Bool\", found: other.type_name() }}.at(brul::ast::Position {{ line: {}, column: {} }})),",
                condition.position.line, condition.position.column
            );
            out.push_str("    }\n");
        }
    }
    Ok(())
}
