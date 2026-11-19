use std::fmt::Write;

use brul_script::Names;
use brul_syntax::error::Error;
use brul_syntax::script::Statement;

use super::expression::operation;

pub(super) fn block(
    statements: &[Statement],
    names: &Names<'_>,
    out: &mut String,
) -> Result<(), Error> {
    for statement in statements {
        emit_statement(statement, names, out)?;
    }
    Ok(())
}

fn emit_statement(statement: &Statement, names: &Names<'_>, out: &mut String) -> Result<(), Error> {
    match statement {
        Statement::Let {
            name,
            value,
            constant,
        } => {
            let value = match value {
                Some(value) => operation(value, names)?,
                None => "brul::Value::Null".to_owned(),
            };
            let _ = writeln!(
                out,
                "    component.declare({name:?}, {value}, {constant})?;"
            );
        }
        Statement::Expr(expression) => {
            let _ = writeln!(out, "    {};", operation(expression, names)?);
        }
        Statement::Block(statements) => {
            out.push_str("    {\n");
            block(statements, names, out)?;
            out.push_str("    }\n");
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let value = operation(condition, names)?;
            let _ = writeln!(out, "    match {value} {{");
            let _ = writeln!(out, "        brul::Value::Bool(true) => {{");
            block(then_branch, names, out)?;
            out.push_str("        }\n");
            if let Some(branch) = else_branch {
                out.push_str("        brul::Value::Bool(false) => {\n");
                emit_statement(branch, names, out)?;
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
