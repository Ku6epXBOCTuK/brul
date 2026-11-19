use std::fmt::Write;

use brul_script::Names;
use brul_syntax::error::Error;
use brul_syntax::script::Statement;

use super::expression::operation_in;
use super::scopes::Scopes;
use super::state::State;

pub(super) fn top_level(
    statements: &[Statement],
    names: &Names<'_>,
    state: &State,
    out: &mut String,
) -> Result<(), Error> {
    block_in(statements, names, state, out, &mut Scopes::new(), true)
}

pub(super) fn block_with_scopes(
    statements: &[Statement],
    names: &Names<'_>,
    state: &State,
    out: &mut String,
    scopes: &mut Scopes,
) -> Result<(), Error> {
    block_in(statements, names, state, out, scopes, false)
}

fn block_in(
    statements: &[Statement],
    names: &Names<'_>,
    state: &State,
    out: &mut String,
    scopes: &mut Scopes,
    top_level: bool,
) -> Result<(), Error> {
    for statement in statements {
        emit_statement(statement, names, state, out, scopes, top_level)?;
    }
    Ok(())
}

fn emit_statement(
    statement: &Statement,
    names: &Names<'_>,
    state: &State,
    out: &mut String,
    scopes: &mut Scopes,
    top_level: bool,
) -> Result<(), Error> {
    match statement {
        Statement::Let {
            name,
            ty,
            value,
            constant,
        } => {
            if top_level {
                let null = "brul::Value::Null".to_owned();
                let (method, value) = match value {
                    Some(value) => ("declare", operation_in(value, names, state, scopes)?),
                    None => ("declare_if_absent", null),
                };
                let _ = writeln!(
                    out,
                    "    component.{method}({name:?}, {value}, {constant})?;"
                );
                return Ok(());
            }
            let ident = scopes.declare(name, ty, *constant)?;
            let mutable = if *constant { "" } else { "mut " };
            let local_ty = Scopes::local_type(ty);
            let local = if matches!(ty, brul_syntax::types::Type::Null) {
                "()".to_owned()
            } else if Scopes::is_value(ty) {
                match value {
                    Some(value) => operation_in(value, names, state, scopes)?,
                    None => "brul::Value::Null".to_owned(),
                }
            } else {
                let value = match value {
                    Some(value) => operation_in(value, names, state, scopes)?,
                    None => "brul::Value::Null".to_owned(),
                };
                format!("(&{value}).try_into()?")
            };
            let _ = writeln!(out, "    let {mutable}{ident}: {local_ty} = {local};");
        }
        Statement::Expr(expression) => {
            let _ = writeln!(
                out,
                "    {};",
                operation_in(expression, names, state, scopes)?
            );
        }
        Statement::Block(statements) => {
            out.push_str("    {\n");
            scopes.push();
            block_in(statements, names, state, out, scopes, false)?;
            scopes.pop();
            out.push_str("    }\n");
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let value = operation_in(condition, names, state, scopes)?;
            let _ = writeln!(out, "    match {value} {{");
            let _ = writeln!(out, "        brul::Value::Bool(true) => {{");
            scopes.push();
            block_in(then_branch, names, state, out, scopes, false)?;
            scopes.pop();
            out.push_str("        }\n");
            if let Some(branch) = else_branch {
                out.push_str("        brul::Value::Bool(false) => {\n");
                scopes.push();
                emit_statement(branch, names, state, out, scopes, false)?;
                scopes.pop();
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
