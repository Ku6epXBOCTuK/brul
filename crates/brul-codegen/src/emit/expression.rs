use std::fmt::Write;

use brul_script::Names;
use brul_syntax::ast::{AssignOp, Expr, ExprKind, TemplatePart};
use brul_syntax::error::Error;

use super::names;
use super::state::State;

pub fn closure(value: &Expr, names: &Names<'_>, state: &State) -> Result<String, Error> {
    Ok(format!(
        "|component: &mut brul::Component| -> Result<brul::Value, brul::Error> {{ let value = {}; Ok(value) }}",
        operation(value, names, state)?
    ))
}

pub fn operation(expression: &Expr, names: &Names<'_>, state: &State) -> Result<String, Error> {
    Ok(match &expression.kind {
        ExprKind::Number(value) => format!("brul::Value::Number({value:?})"),
        ExprKind::String(text) => format!("brul::Value::String({text:?}.into())"),
        ExprKind::Bool(value) => format!("brul::Value::Bool({value})"),
        ExprKind::Null => "brul::Value::Null".to_owned(),
        ExprKind::Ident(name) => match state.read(name) {
            Some(read) => read,
            None => format!(
                "component.read_id({})?",
                names::variable_expr(name, expression.position, names)?
            ),
        },
        ExprKind::Field { target, name } => format!(
            "brul::eval::read_field(&({}), {name:?})?",
            operation(target, names, state)?
        ),
        ExprKind::Index { target, index } => format!(
            "brul::eval::read_index(&({}), &({}))?",
            operation(target, names, state)?,
            operation(index, names, state)?
        ),
        ExprKind::Array(items) => format!(
            "brul::Value::Array(vec![{}].into())",
            operations(items, names, state)?
        ),
        ExprKind::Object(fields) => {
            let fields = fields
                .iter()
                .map(|(name, value)| {
                    Ok(format!(
                        "({name:?}.to_owned(), {})",
                        operation(value, names, state)?
                    ))
                })
                .collect::<Result<Vec<_>, Error>>()?
                .join(", ");
            format!("brul::Value::Object(vec![{fields}].into())")
        }
        ExprKind::Template(parts) => {
            let parts = parts
                .iter()
                .map(|part| match part {
                    TemplatePart::Text(text) => Ok(format!("brul::Value::String({text:?}.into())")),
                    TemplatePart::Expr(expression) => operation(expression, names, state),
                })
                .collect::<Result<Vec<_>, Error>>()?
                .join(", ");
            format!("brul::Value::String(brul::eval::template(vec![{parts}])?)")
        }
        ExprKind::Call { target, args } => call(target, args, names, state)?,
        ExprKind::Unary { op, operand } => format!(
            "brul::eval::apply_unary(brul::ast::UnaryOp::{op:?}, {})?",
            operation(operand, names, state)?
        ),
        ExprKind::Binary { op, left, right } => format!(
            "brul::eval::apply_binary(brul::ast::BinaryOp::{op:?}, {}, {})?",
            operation(left, names, state)?,
            operation(right, names, state)?
        ),
        ExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "match {} {{ brul::Value::Bool(true) => {}, brul::Value::Bool(false) => {}, other => return Err(brul::Error::Expected {{ operation: \"?:\", expected: \"Bool\", found: other.type_name() }}), }}",
            operation(condition, names, state)?,
            operation(then_branch, names, state)?,
            operation(else_branch, names, state)?
        ),
        ExprKind::Assign { target, op, value } => assignment(target, *op, value, names, state)?,
    })
}

fn operations(expressions: &[Expr], names: &Names<'_>, state: &State) -> Result<String, Error> {
    Ok(expressions
        .iter()
        .map(|expression| operation(expression, names, state))
        .collect::<Result<Vec<_>, Error>>()?
        .join(", "))
}

fn call(
    target: &Expr,
    arguments: &[Expr],
    names: &Names<'_>,
    state: &State,
) -> Result<String, Error> {
    let ExprKind::Ident(name) = &target.kind else {
        return Ok("{ return Err(brul::Error::Unsupported(\"method call\")) }".to_owned());
    };
    if name == "every" {
        return every(arguments, names, state);
    }
    Ok(format!(
        "{{ let arguments = vec![{}]; brul::eval::call(component, {name:?}, arguments)? }}",
        operations(arguments, names, state)?
    ))
}

fn every(arguments: &[Expr], names: &Names<'_>, state: &State) -> Result<String, Error> {
    let [interval, function] = arguments else {
        return Ok(
            "{ return Err(brul::Error::Arity { name: \"every\".to_owned(), expected: \"two\" }) }"
                .to_owned(),
        );
    };
    let ExprKind::Ident(function) = &function.kind else {
        return Ok(format!(
            "{{ return Err(brul::Error::Argument {{ name: \"every\", expected: \"function name\", found: {:?} }}) }}",
            function.kind.name()
        ));
    };
    Ok(format!(
        "match {} {{ brul::Value::Number(interval) => {{ component.every(interval, {function:?})?; brul::Value::Null }}, other => return Err(brul::Error::Argument {{ name: \"every\", expected: \"Number\", found: other.type_name() }}), }}",
        operation(interval, names, state)?
    ))
}

fn assignment(
    target: &Expr,
    op: AssignOp,
    value: &Expr,
    names: &Names<'_>,
    state: &State,
) -> Result<String, Error> {
    let value = match op {
        AssignOp::Set => operation(value, names, state)?,
        AssignOp::Add => format!(
            "brul::eval::apply_binary(brul::ast::BinaryOp::Add, {}, {})?",
            operation(target, names, state)?,
            operation(value, names, state)?
        ),
        AssignOp::Sub => format!(
            "brul::eval::apply_binary(brul::ast::BinaryOp::Sub, {}, {})?",
            operation(target, names, state)?,
            operation(value, names, state)?
        ),
    };
    let mut out = format!("{{ let value = {value};\n");
    assign_to(&mut out, target, "value", 0, names, state)?;
    out.push_str("\nvalue }");
    Ok(out)
}

fn assign_to(
    out: &mut String,
    target: &Expr,
    value: &str,
    depth: usize,
    names: &Names<'_>,
    state: &State,
) -> Result<(), Error> {
    match &target.kind {
        ExprKind::Ident(name) => {
            if let Some(write) = state.write(name, value) {
                let _ = writeln!(out, "    {write}");
                return Ok(());
            }
            let _ = writeln!(
                out,
                "component.assign_id({}, {value}.clone())?;",
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
                operation(field, names, state)?
            );
            let _ = writeln!(
                out,
                "brul::eval::assign_field(&mut {container}, {name:?}, {value}.clone())?;"
            );
            assign_to(out, field, &container, depth + 1, names, state)
        }
        ExprKind::Index {
            target: container_expression,
            index,
        } => {
            let position = format!("index{depth}");
            let container = format!("container{depth}");
            let _ = writeln!(out, "let {position} = {};", operation(index, names, state)?);
            let _ = writeln!(
                out,
                "let mut {container} = {};",
                operation(container_expression, names, state)?
            );
            let _ = writeln!(
                out,
                "brul::eval::assign_index(&mut {container}, &{position}, {value}.clone())?;"
            );
            assign_to(
                out,
                container_expression,
                &container,
                depth + 1,
                names,
                state,
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
