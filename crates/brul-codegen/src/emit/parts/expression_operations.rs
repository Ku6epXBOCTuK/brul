use std::collections::HashMap;
use std::fmt::Write;

use brul_script::Names;

use brul_syntax::ast::{AssignOp, Expr, ExprKind, TemplatePart};
use brul_syntax::error::Error;

use brul_syntax::types::Type;

use super::names;
use super::rust_string;
use super::scopes::Scopes;
use super::state::State;

use super::expression::{typed, wrap_typed};

pub(super) fn operation_in(
    expression: &Expr,
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<String, Error> {
    if let Some((code, ty)) = typed(expression, names, state, props, scopes)? {
        return Ok(wrap_typed(code, &ty));
    }
    Ok(match &expression.kind {
        ExprKind::Number(value) => format!("brul::Dyn::num({value:?})"),
        ExprKind::String(text) => format!("brul::Dyn::text({})", rust_string(text)),
        ExprKind::Bool(value) => format!("brul::Dyn::flag({value})"),
        ExprKind::Null => "brul::Dyn::null()".to_owned(),
        ExprKind::Ident(name) => {
            if let Some(read) = scopes.read(name) {
                read
            } else if let Some(read) = state.read(name) {
                read
            } else {
                format!(
                    "brul::Dyn::from(component.read_id({})?)",
                    names::variable_expr(name, expression.position, names)?
                )
            }
        }
        ExprKind::Field { target, name } => format!(
            "({}).field({name:?})?",
            operation_in(target, names, state, props, scopes)?
        ),
        ExprKind::Index { target, index } => format!(
            "({}).index(&({}))?",
            operation_in(target, names, state, props, scopes)?,
            operation_in(index, names, state, props, scopes)?
        ),
        ExprKind::Array(items) => format!(
            "brul::Dyn::array(vec![{}])",
            operations(items, names, state, props, scopes)?
        ),
        ExprKind::Object(fields) => {
            let fields = fields
                .iter()
                .map(|(name, value)| {
                    Ok(format!(
                        "({name:?}.to_owned(), {})",
                        operation_in(value, names, state, props, scopes)?
                    ))
                })
                .collect::<Result<Vec<_>, Error>>()?
                .join(", ");
            format!("brul::Dyn::object(vec![{fields}])")
        }
        ExprKind::Template(parts) => {
            let parts = parts
                .iter()
                .map(|part| match part {
                    TemplatePart::Text(text) => {
                        Ok(format!("brul::Dyn::text({})", rust_string(text)))
                    }
                    TemplatePart::Expr(expression) => {
                        operation_in(expression, names, state, props, scopes)
                    }
                })
                .collect::<Result<Vec<_>, Error>>()?
                .join(", ");
            format!("brul::Dyn::template(vec![{parts}])?")
        }
        ExprKind::Call { target, args } => call(target, args, names, state, props, scopes)?,
        ExprKind::Unary { op, operand } => format!(
            "brul::Dyn::unary(brul::ast::UnaryOp::{op:?}, &({}))?",
            operation_in(operand, names, state, props, scopes)?
        ),
        ExprKind::Binary { op, left, right } => format!(
            "brul::Dyn::binary(brul::ast::BinaryOp::{op:?}, &({}), &({}))?",
            operation_in(left, names, state, props, scopes)?,
            operation_in(right, names, state, props, scopes)?
        ),
        ExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "if ({}).boolean()? {{ {} }} else {{ {} }}",
            operation_in(condition, names, state, props, scopes)?,
            operation_in(then_branch, names, state, props, scopes)?,
            operation_in(else_branch, names, state, props, scopes)?
        ),
        ExprKind::Assign { target, op, value } => {
            assignment(target, *op, value, names, state, props, scopes)?
        }
    })
}

fn operations(
    expressions: &[Expr],
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<String, Error> {
    Ok(expressions
        .iter()
        .map(|expression| operation_in(expression, names, state, props, scopes))
        .collect::<Result<Vec<_>, Error>>()?
        .join(", "))
}

fn call(
    target: &Expr,
    arguments: &[Expr],
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<String, Error> {
    let ExprKind::Ident(name) = &target.kind else {
        return Ok("{ return Err(brul::Error::Unsupported(\"method call\")) }".to_owned());
    };
    Ok(format!(
        "{{ let arguments = vec![{}]; brul::Dyn::call(component, {name:?}, arguments)? }}",
        operations(arguments, names, state, props, scopes)?
    ))
}

fn assignment(
    target: &Expr,
    op: AssignOp,
    value: &Expr,
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<String, Error> {
    let value = match op {
        AssignOp::Set => operation_in(value, names, state, props, scopes)?,
        AssignOp::Add => format!(
            "brul::Dyn::binary(brul::ast::BinaryOp::Add, &({}), &({}))?",
            operation_in(target, names, state, props, scopes)?,
            operation_in(value, names, state, props, scopes)?
        ),
        AssignOp::Sub => format!(
            "brul::Dyn::binary(brul::ast::BinaryOp::Sub, &({}), &({}))?",
            operation_in(target, names, state, props, scopes)?,
            operation_in(value, names, state, props, scopes)?
        ),
    };
    let mut out = format!("{{ let value = {value};\n");
    assign_to(&mut out, target, "value", 0, names, state, props, scopes)?;
    out.push_str("\nvalue }");
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn assign_to(
    out: &mut String,
    target: &Expr,
    value: &str,
    depth: usize,
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<(), Error> {
    match &target.kind {
        ExprKind::Ident(name) => {
            if let Some(write) = scopes.write(name, value)? {
                let _ = writeln!(out, "    {write}");
                return Ok(());
            }
            if let Some(write) = state.write(name, value) {
                let _ = writeln!(out, "    {write}");
                return Ok(());
            }
            let _ = writeln!(
                out,
                "component.assign_id({}, ({value}.clone()).into())?;",
                names::variable_expr(name, target.position, names)?
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
                operation_in(field, names, state, props, scopes)?
            );
            let _ = writeln!(out, "{container}.assign_field({name:?}, &{value})?;");
            assign_to(
                out,
                field,
                &container,
                depth + 1,
                names,
                state,
                props,
                scopes,
            )
        }
        ExprKind::Index {
            target: container_expression,
            index,
        } => {
            let position = format!("index{depth}");
            let container = format!("container{depth}");
            let _ = writeln!(
                out,
                "let {position} = {};",
                operation_in(index, names, state, props, scopes)?
            );
            let _ = writeln!(
                out,
                "let mut {container} = {};",
                operation_in(container_expression, names, state, props, scopes)?
            );
            let _ = writeln!(out, "{container}.assign_index(&{position}, &{value})?;");
            assign_to(
                out,
                container_expression,
                &container,
                depth + 1,
                names,
                state,
                props,
                scopes,
            )
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
