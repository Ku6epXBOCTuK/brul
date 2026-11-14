use brul_syntax::ast::{BinaryOp, UnaryOp};

use crate::error::Error;
use crate::value::Value;

use super::errors::{expected, mismatch};

pub fn apply_unary(op: UnaryOp, value: Value) -> Result<Value, Error> {
    match (op, value) {
        (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
        (UnaryOp::Neg, Value::Number(value)) => Ok(Value::Number(-value)),
        (UnaryOp::Not, other) => Err(expected("!", "Bool", &other)),
        (UnaryOp::Neg, other) => Err(expected("-", "Number", &other)),
    }
}

pub fn apply_binary(op: BinaryOp, left: Value, right: Value) -> Result<Value, Error> {
    match op {
        BinaryOp::Add => add_or_concatenate(left, right),
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

fn add_or_concatenate(left: Value, right: Value) -> Result<Value, Error> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(left + right)),
        (Value::String(left), Value::String(right)) => {
            let mut text = String::with_capacity(left.len() + right.len());
            text.push_str(left.as_ref());
            text.push_str(right.as_ref());
            Ok(Value::String(text.into()))
        }
        (left, right) => Err(mismatch(BinaryOp::Add, &left, &right)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unary_and_binary_operations_match_the_interpreter() {
        assert_eq!(
            apply_binary(BinaryOp::Add, Value::Number(1.0), Value::Number(2.0)),
            Ok(Value::Number(3.0))
        );
        assert_eq!(
            apply_unary(UnaryOp::Neg, Value::Number(2.0)),
            Ok(Value::Number(-2.0))
        );
    }
}
