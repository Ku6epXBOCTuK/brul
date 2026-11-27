use std::fmt::Write;

use brul_syntax::ast::{AssignOp, Expr, ExprKind, TemplatePart};
use brul_syntax::error::Error;

use super::names;
use super::rust_string;

use super::ExprContext;
use super::expression::{typed, wrap_typed};

pub(super) fn operation_in(expression: &Expr, context: &ExprContext<'_>) -> Result<String, Error> {
    if let Some((code, ty)) = typed(expression, context)? {
        return Ok(wrap_typed(code, &ty));
    }
    Ok(match &expression.kind {
        ExprKind::Number(value) => format!("brul::Dyn::num({value:?})"),
        ExprKind::String(text) => format!("brul::Dyn::text({})", rust_string(text)),
        ExprKind::Bool(value) => format!("brul::Dyn::flag({value})"),
        ExprKind::Null => "brul::Dyn::null()".to_owned(),
        ExprKind::Ident(name) => {
            if let Some(read) = context.scopes.read(name) {
                read
            } else if let Some(read) = context.state.read(name) {
                read
            } else {
                format!(
                    "brul::Dyn::from(component.read_id({})?)",
                    names::variable_expr(name, expression.position, context.names)?
                )
            }
        }
        ExprKind::Field { target, name } => {
            format!("({}).field({name:?})?", operation_in(target, context)?)
        }
        ExprKind::Index { target, index } => format!(
            "({}).index(&({}))?",
            operation_in(target, context)?,
            operation_in(index, context)?
        ),
        ExprKind::Array(items) => {
            format!("brul::Dyn::array(vec![{}])", operations(items, context)?)
        }
        ExprKind::Object(fields) => {
            let fields = fields
                .iter()
                .map(|(name, value)| {
                    Ok(format!(
                        ".field({name:?}, {})",
                        operation_in(value, context)?
                    ))
                })
                .collect::<Result<Vec<_>, Error>>()?
                .concat();
            format!("brul::Dyn::object(brul::Object::builder(){fields})")
        }
        ExprKind::Template(parts) => {
            let parts = parts
                .iter()
                .map(|part| match part {
                    TemplatePart::Text(text) => {
                        Ok(format!("brul::Dyn::text({})", rust_string(text)))
                    }
                    TemplatePart::Expr(expression) => operation_in(expression, context),
                })
                .collect::<Result<Vec<_>, Error>>()?
                .join(", ");
            format!("brul::Dyn::template(vec![{parts}])?")
        }
        ExprKind::Call { target, args } => call(target, args, context)?,
        ExprKind::Unary { op, operand } => format!(
            "brul::Dyn::unary(brul::ast::UnaryOp::{op:?}, &({}))?",
            operation_in(operand, context)?
        ),
        ExprKind::Binary { op, left, right } => format!(
            "brul::Dyn::binary(brul::ast::BinaryOp::{op:?}, &({}), &({}))?",
            operation_in(left, context)?,
            operation_in(right, context)?
        ),
        ExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "if ({}).boolean()? {{ {} }} else {{ {} }}",
            operation_in(condition, context)?,
            operation_in(then_branch, context)?,
            operation_in(else_branch, context)?
        ),
        ExprKind::Assign { target, op, value } => assignment(target, *op, value, context)?,
    })
}

fn operations(expressions: &[Expr], context: &ExprContext<'_>) -> Result<String, Error> {
    Ok(expressions
        .iter()
        .map(|expression| operation_in(expression, context))
        .collect::<Result<Vec<_>, Error>>()?
        .join(", "))
}

fn call(target: &Expr, arguments: &[Expr], context: &ExprContext<'_>) -> Result<String, Error> {
    let ExprKind::Ident(name) = &target.kind else {
        return Ok("{ return Err(brul::Error::Unsupported(\"method call\")) }".to_owned());
    };
    Ok(format!(
        "{{ let arguments = vec![{}]; component.call_dynamic({name:?}, arguments)? }}",
        operations(arguments, context)?
    ))
}

fn assignment(
    target: &Expr,
    op: AssignOp,
    value: &Expr,
    context: &ExprContext<'_>,
) -> Result<String, Error> {
    let value = match op {
        AssignOp::Set => operation_in(value, context)?,
        AssignOp::Add => format!(
            "brul::Dyn::binary(brul::ast::BinaryOp::Add, &({}), &({}))?",
            operation_in(target, context)?,
            operation_in(value, context)?
        ),
        AssignOp::Sub => format!(
            "brul::Dyn::binary(brul::ast::BinaryOp::Sub, &({}), &({}))?",
            operation_in(target, context)?,
            operation_in(value, context)?
        ),
    };
    let mut out = format!("{{ let value = {value};\n");
    assign_to(&mut out, target, "value", 0, context)?;
    out.push_str("\nvalue }");
    Ok(out)
}

fn assign_to(
    out: &mut String,
    target: &Expr,
    value: &str,
    depth: usize,
    context: &ExprContext<'_>,
) -> Result<(), Error> {
    match &target.kind {
        ExprKind::Ident(name) => {
            if let Some(write) = context.scopes.write(name, value)? {
                let _ = writeln!(out, "    {write}");
                return Ok(());
            }
            if let Some(write) = context.state.write(name, value) {
                let _ = writeln!(out, "    {write}");
                return Ok(());
            }
            let _ = writeln!(
                out,
                "component.assign_id({}, ({value}.clone()).into())?;",
                names::variable_expr(name, target.position, context.names)?
            );
            Ok(())
        }
        ExprKind::Field {
            target: field,
            name,
        } => {
            let container = format!("container{depth}");
            let _ = writeln!(
                out,
                "let mut {container} = {};",
                operation_in(field, context)?
            );
            let _ = writeln!(out, "{container}.assign_field({name:?}, &{value})?;");
            assign_to(out, field, &container, depth + 1, context)
        }
        ExprKind::Index {
            target: container_expression,
            index,
        } => {
            let position = format!("index{depth}");
            let container = format!("container{depth}");
            let _ = writeln!(out, "let {position} = {};", operation_in(index, context)?);
            let _ = writeln!(
                out,
                "let mut {container} = {};",
                operation_in(container_expression, context)?
            );
            let _ = writeln!(out, "{container}.assign_index(&{position}, &{value})?;");
            assign_to(out, container_expression, &container, depth + 1, context)
        }
        other => {
            let _ = write!(
                out,
                "return Err(brul::Error::Unsupported({:?}));",
                other.name()
            );
            Ok(())
        }
    }
}
