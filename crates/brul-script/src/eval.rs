use brul_syntax::ast::{AssignOp, BinaryOp, Expr, ExprKind, TemplatePart, UnaryOp};

use crate::component::Component;
use crate::error::Error;
use crate::value::Value;

mod access;
mod call;
mod errors;
mod operators;
mod template;

pub use access::{assign_field, assign_index, read_field, read_index};
pub use call::call;
pub use operators::{apply_binary, apply_unary};
pub use template::template;

use errors::{argument_error, expected};

pub fn eval(component: &mut Component, expression: &Expr) -> Result<Value, Error> {
    eval_expression(component, expression).map_err(|error| error.at(expression.position))
}

fn eval_expression(component: &mut Component, expression: &Expr) -> Result<Value, Error> {
    match &expression.kind {
        ExprKind::Number(value) => Ok(Value::Number(*value)),
        ExprKind::String(value) => Ok(Value::String(value.as_str().into())),
        ExprKind::Bool(value) => Ok(Value::Bool(*value)),
        ExprKind::Null => Ok(Value::Null),
        ExprKind::Ident(name) => component.read(name),
        ExprKind::Field { target, name } => read_field(&eval(component, target)?, name),
        ExprKind::Index { target, index } => {
            let target = eval(component, target)?;
            let index = eval(component, index)?;
            read_index(&target, &index)
        }
        ExprKind::Array(items) => {
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                values.push(eval(component, item)?);
            }
            Ok(Value::Array(values.into()))
        }
        ExprKind::Object(fields) => {
            let mut values = Vec::with_capacity(fields.len());
            for (name, value) in fields {
                values.push((name.clone(), eval(component, value)?));
            }
            Ok(Value::Object(values.into()))
        }
        ExprKind::Template(parts) => eval_template(component, parts),
        ExprKind::Call { target, args } => eval_call(component, target, args),
        ExprKind::Assign { target, op, value } => eval_assign(component, target, *op, value),
        ExprKind::Unary { op, operand } => eval_unary(component, *op, operand),
        ExprKind::Binary { op, left, right } => eval_binary(component, *op, left, right),
        ExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => match eval(component, condition)? {
            Value::Bool(true) => eval(component, then_branch),
            Value::Bool(false) => eval(component, else_branch),
            other => Err(expected("?:", "Bool", &other)),
        },
    }
}

fn eval_call(component: &mut Component, target: &Expr, arguments: &[Expr]) -> Result<Value, Error> {
    let ExprKind::Ident(name) = &target.kind else {
        return Err(Error::Unsupported("method call"));
    };
    if name == "every" {
        return call_every(component, arguments);
    }
    let mut values = Vec::with_capacity(arguments.len());
    for argument in arguments {
        values.push(eval(component, argument)?);
    }
    call(component, name, values)
}

fn call_every(component: &mut Component, arguments: &[Expr]) -> Result<Value, Error> {
    let [interval, function] = arguments else {
        return Err(Error::Arity {
            name: "every".to_owned(),
            expected: "two",
        });
    };
    let interval = eval(component, interval)?;
    let Value::Number(interval) = interval else {
        return Err(argument_error("every", "Number", &interval));
    };
    let ExprKind::Ident(function) = &function.kind else {
        return Err(Error::Argument {
            name: "every",
            expected: "function name",
            found: expression_name(function),
        });
    };
    component.every(interval, function)?;
    Ok(Value::Null)
}

fn eval_assign(
    component: &mut Component,
    target: &Expr,
    op: AssignOp,
    value: &Expr,
) -> Result<Value, Error> {
    let value = match op {
        AssignOp::Set => eval(component, value)?,
        AssignOp::Add => {
            let left = eval(component, target)?;
            apply_binary(BinaryOp::Add, left, eval(component, value)?)?
        }
        AssignOp::Sub => {
            let left = eval(component, target)?;
            apply_binary(BinaryOp::Sub, left, eval(component, value)?)?
        }
    };
    assign_to(component, target, value.clone())?;
    Ok(value)
}

fn assign_to(component: &mut Component, target: &Expr, value: Value) -> Result<(), Error> {
    match target {
        Expr {
            kind: ExprKind::Ident(name),
            ..
        } => component.assign(name, value),
        Expr {
            kind: ExprKind::Field { target, name },
            ..
        } => {
            let mut container = eval(component, target)?;
            assign_field(&mut container, name, value)?;
            assign_to(component, target, container)
        }
        Expr {
            kind: ExprKind::Index { target, index },
            ..
        } => {
            let index = eval(component, index)?;
            let mut container = eval(component, target)?;
            assign_index(&mut container, &index, value)?;
            assign_to(component, target, container)
        }
        other => Err(Error::Unsupported(expression_name(other))),
    }
}

fn eval_template(component: &mut Component, parts: &[TemplatePart]) -> Result<Value, Error> {
    let mut values = Vec::with_capacity(parts.len());
    for part in parts {
        values.push(match part {
            TemplatePart::Text(chunk) => Value::String(chunk.as_str().into()),
            TemplatePart::Expr(expression) => eval(component, expression)?,
        });
    }
    Ok(Value::String(template(values)?.into()))
}

fn eval_unary(component: &mut Component, op: UnaryOp, operand: &Expr) -> Result<Value, Error> {
    apply_unary(op, eval(component, operand)?)
}

fn eval_binary(
    component: &mut Component,
    op: BinaryOp,
    left: &Expr,
    right: &Expr,
) -> Result<Value, Error> {
    let left = eval(component, left)?;
    let right = eval(component, right)?;
    apply_binary(op, left, right)
}

fn expression_name(expression: &Expr) -> &'static str {
    match &expression.kind {
        ExprKind::Ident(_) => "identifier",
        ExprKind::Field { .. } => "field access",
        ExprKind::Index { .. } => "index access",
        ExprKind::Call { .. } => "call",
        ExprKind::Assign { .. } => "assignment",
        ExprKind::Array(_) => "array literal",
        ExprKind::Object(_) => "object literal",
        ExprKind::Template(_) => "template string",
        ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Bool(_)
        | ExprKind::Null
        | ExprKind::Unary { .. }
        | ExprKind::Binary { .. }
        | ExprKind::Ternary { .. } => "expression",
    }
}

#[cfg(test)]
mod tests {
    use brul_syntax::ast::Position;

    use super::*;

    const POSITION: Position = Position { line: 1, column: 1 };

    fn ident() -> Expr {
        Expr::new(ExprKind::Ident("x".to_owned()), POSITION)
    }

    #[test]
    fn expression_name_spells_every_kind() {
        let target = || Box::new(ident());
        let named = [
            (ExprKind::Ident("x".to_owned()), "identifier"),
            (
                ExprKind::Field {
                    target: target(),
                    name: "x".to_owned(),
                },
                "field access",
            ),
            (
                ExprKind::Index {
                    target: target(),
                    index: target(),
                },
                "index access",
            ),
            (
                ExprKind::Call {
                    target: target(),
                    args: Vec::new(),
                },
                "call",
            ),
            (
                ExprKind::Assign {
                    target: target(),
                    op: AssignOp::Set,
                    value: target(),
                },
                "assignment",
            ),
            (ExprKind::Array(Vec::new()), "array literal"),
            (ExprKind::Object(Vec::new()), "object literal"),
            (ExprKind::Template(Vec::new()), "template string"),
            (ExprKind::Number(1.0), "expression"),
            (ExprKind::String(String::new()), "expression"),
            (ExprKind::Bool(true), "expression"),
            (ExprKind::Null, "expression"),
            (
                ExprKind::Unary {
                    op: UnaryOp::Not,
                    operand: target(),
                },
                "expression",
            ),
            (
                ExprKind::Binary {
                    op: BinaryOp::Add,
                    left: target(),
                    right: target(),
                },
                "expression",
            ),
            (
                ExprKind::Ternary {
                    condition: target(),
                    then_branch: target(),
                    else_branch: target(),
                },
                "expression",
            ),
        ];
        for (kind, name) in named {
            assert_eq!(expression_name(&Expr::new(kind, POSITION)), name);
        }
    }
}
