use brul_syntax::ast::{BinaryOp, Expr, UnaryOp};

use crate::error::Error;
use crate::value::Value;

pub fn eval(expression: &Expr) -> Result<Value, Error> {
    match expression {
        Expr::Number(value) => Ok(Value::Number(*value)),
        Expr::String(value) => Ok(Value::String(value.clone())),
        Expr::Bool(value) => Ok(Value::Bool(*value)),
        Expr::Null => Ok(Value::Null),
        Expr::Unary { op, operand } => eval_unary(*op, operand),
        Expr::Binary { op, left, right } => eval_binary(*op, left, right),
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => match eval(condition)? {
            Value::Bool(true) => eval(then_branch),
            Value::Bool(false) => eval(else_branch),
            other => Err(expected("?:", "Bool", &other)),
        },
        other => Err(Error::Unsupported(expression_name(other))),
    }
}

fn eval_unary(op: UnaryOp, operand: &Expr) -> Result<Value, Error> {
    let value = eval(operand)?;
    match (op, value) {
        (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
        (UnaryOp::Neg, Value::Number(value)) => Ok(Value::Number(-value)),
        (UnaryOp::Not, other) => Err(expected("!", "Bool", &other)),
        (UnaryOp::Neg, other) => Err(expected("-", "Number", &other)),
    }
}

fn eval_binary(op: BinaryOp, left: &Expr, right: &Expr) -> Result<Value, Error> {
    let left = eval(left)?;
    let right = eval(right)?;
    match op {
        BinaryOp::Add => match (left, right) {
            (Value::Number(left), Value::Number(right)) => Ok(Value::Number(left + right)),
            (Value::String(mut left), Value::String(right)) => {
                left.push_str(&right);
                Ok(Value::String(left))
            }
            (left, right) => Err(mismatch(op, &left, &right)),
        },
        BinaryOp::Sub => arithmetic(op, left, right, |left, right| left - right),
        BinaryOp::Mul => arithmetic(op, left, right, |left, right| left * right),
        BinaryOp::Div => arithmetic(op, left, right, |left, right| left / right),
        BinaryOp::Rem => arithmetic(op, left, right, |left, right| left % right),
        BinaryOp::Eq => Ok(Value::Bool(left == right)),
        BinaryOp::NotEq => Ok(Value::Bool(left != right)),
        BinaryOp::Lt => comparison(op, left, right, |left, right| left < right),
        BinaryOp::Gt => comparison(op, left, right, |left, right| left > right),
        BinaryOp::Le => comparison(op, left, right, |left, right| left <= right),
        BinaryOp::Ge => comparison(op, left, right, |left, right| left >= right),
        BinaryOp::And => logic(op, left, right, |left, right| left && right),
        BinaryOp::Or => logic(op, left, right, |left, right| left || right),
    }
}

fn arithmetic(
    op: BinaryOp,
    left: Value,
    right: Value,
    apply: impl FnOnce(f64, f64) -> f64,
) -> Result<Value, Error> {
    let (left, right) = number_pair(op, &left, &right)?;
    if matches!(op, BinaryOp::Div | BinaryOp::Rem) && right == 0.0 {
        return Err(Error::DivisionByZero);
    }
    Ok(Value::Number(apply(left, right)))
}

fn comparison(
    op: BinaryOp,
    left: Value,
    right: Value,
    apply: impl FnOnce(f64, f64) -> bool,
) -> Result<Value, Error> {
    let (left, right) = number_pair(op, &left, &right)?;
    Ok(Value::Bool(apply(left, right)))
}

fn logic(
    op: BinaryOp,
    left: Value,
    right: Value,
    apply: impl FnOnce(bool, bool) -> bool,
) -> Result<Value, Error> {
    match (&left, &right) {
        (Value::Bool(left), Value::Bool(right)) => Ok(Value::Bool(apply(*left, *right))),
        _ => Err(mismatch(op, &left, &right)),
    }
}

fn number_pair(op: BinaryOp, left: &Value, right: &Value) -> Result<(f64, f64), Error> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => Ok((*left, *right)),
        _ => Err(mismatch(op, left, right)),
    }
}

fn mismatch(op: BinaryOp, left: &Value, right: &Value) -> Error {
    Error::Mismatch {
        operation: op_name(op),
        left: left.type_name(),
        right: right.type_name(),
    }
}

fn expected(operation: &'static str, expected: &'static str, found: &Value) -> Error {
    Error::Expected {
        operation,
        expected,
        found: found.type_name(),
    }
}

fn op_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::Eq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Gt => ">",
        BinaryOp::Le => "<=",
        BinaryOp::Ge => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

fn expression_name(expression: &Expr) -> &'static str {
    match expression {
        Expr::Ident(_) => "identifier",
        Expr::Field { .. } => "field access",
        Expr::Index { .. } => "index access",
        Expr::Call { .. } => "call",
        Expr::Assign { .. } => "assignment",
        Expr::Array(_) => "array literal",
        Expr::Object(_) => "object literal",
        Expr::Template(_) => "template string",
        Expr::Number(_)
        | Expr::String(_)
        | Expr::Bool(_)
        | Expr::Null
        | Expr::Unary { .. }
        | Expr::Binary { .. }
        | Expr::Ternary { .. } => "expression",
    }
}
