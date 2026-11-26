use std::fmt::Write;

use brul_syntax::TypeName;
use brul_syntax::error::Error;
use brul_syntax::script::Statement;
use brul_syntax::types::Type;

use super::ExprContext;
use super::expression::{convert, operation_in, operation_statement, typed};
use super::scopes::Scopes;

pub(super) fn top_level(
    statements: &[Statement],
    out: &mut String,
    context: &mut ExprContext<'_>,
) -> Result<(), Error> {
    block_in(statements, out, context, true)
}

pub(super) fn block_with_scopes(
    statements: &[Statement],
    out: &mut String,
    context: &mut ExprContext<'_>,
) -> Result<(), Error> {
    block_in(statements, out, context, false)
}

fn block_in(
    statements: &[Statement],
    out: &mut String,
    context: &mut ExprContext<'_>,
    top_level: bool,
) -> Result<(), Error> {
    for statement in statements {
        emit_statement(statement, out, context, top_level)?;
    }
    Ok(())
}

fn emit_statement(
    statement: &Statement,
    out: &mut String,
    context: &mut ExprContext<'_>,
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
                let null = "brul::Dyn::null().into()".to_owned();
                let (method, value) = match value {
                    Some(value) => (
                        "declare",
                        format!("({}).into()", operation_in(value, context)?),
                    ),
                    None => ("declare_if_absent", null),
                };
                let _ = writeln!(
                    out,
                    "    component.{method}({name:?}, {value}, {constant})?;"
                );
                return Ok(());
            }
            let ident = context.scopes.declare(name, ty, *constant)?;
            let mutable = if *constant { "" } else { "mut " };
            let local_ty = Scopes::local_type(ty);
            let local = if matches!(ty, brul_syntax::types::Type::Null) {
                "()".to_owned()
            } else if ty.is_value() {
                match value {
                    Some(value) => operation_in(value, context)?,
                    None => "brul::Dyn::null()".to_owned(),
                }
            } else {
                let value = match value {
                    Some(value) => operation_in(value, context)?,
                    None => "brul::Dyn::null()".to_owned(),
                };
                format!("(&{value}).try_into()?")
            };
            let _ = writeln!(out, "    let {mutable}{ident}: {local_ty} = {local};");
        }
        Statement::Expr(expression) => {
            let _ = writeln!(out, "    {};", operation_statement(expression, context)?);
        }
        Statement::Block(statements) => {
            out.push_str("    {\n");
            context.scopes.push();
            block_in(statements, out, context, false)?;
            context.scopes.pop();
            out.push_str("    }\n");
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            match typed(condition, context)? {
                Some((code, ty)) => {
                    if ty != Type::Bool {
                        return Err(crate::error_at(
                            condition.position,
                            format!("if condition expects Bool, found {}", ty.type_name()),
                        ));
                    }
                    let _ = writeln!(out, "    if {code} {{");
                }
                None => {
                    let code = operation_in(condition, context)?;
                    let _ = writeln!(out, "    if ({code}).boolean()? {{");
                }
            }
            context.scopes.push();
            block_in(then_branch, out, context, false)?;
            context.scopes.pop();
            if let Some(branch) = else_branch {
                out.push_str("    } else {\n");
                context.scopes.push();
                emit_statement(branch, out, context, false)?;
                context.scopes.pop();
            }
            out.push_str("    }\n");
        }
        Statement::Return { value, position } => {
            let return_type = context.scopes.return_type().clone();
            let Some(value) = value else {
                if !matches!(return_type, Type::Void | Type::Null) {
                    return Err(crate::error_at(
                        *position,
                        format!(
                            "return requires a value of type {}",
                            return_type.type_name()
                        ),
                    ));
                }
                let _ = writeln!(out, "    return Ok(());");
                return Ok(());
            };
            if return_type == Type::Void {
                return Err(crate::error_at(
                    value.position,
                    "a Void function cannot return a value",
                ));
            }
            let code = if let Some((code, ty)) = typed(value, context)? {
                if ty != return_type {
                    return Err(crate::error_at(
                        value.position,
                        format!(
                            "return expects {}, found {}",
                            return_type.type_name(),
                            ty.type_name()
                        ),
                    ));
                }
                code
            } else {
                let code = operation_in(value, context)?;
                if return_type == Type::Null {
                    format!("({code}).unit()?")
                } else {
                    convert(&code, &return_type)
                }
            };
            let _ = writeln!(out, "    return Ok({code});");
        }
    }
    Ok(())
}
