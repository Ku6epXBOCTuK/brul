use std::rc::Rc;

use brul_syntax::ast::{AssignOp, BinaryOp, Expr, ExprKind, TemplatePart, UnaryOp};

use crate::component::Component;
use crate::error::Error;
use crate::value::Value;

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

pub fn call(component: &mut Component, name: &str, values: Vec<Value>) -> Result<Value, Error> {
    match name {
        "String" => call_string(values),
        "Number" => call_number(values),
        "len" => call_len(values),
        _ if component.has_function(name) => component.call_function(name, values),
        _ => component.call(name, &values),
    }
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

fn call_string(arguments: Vec<Value>) -> Result<Value, Error> {
    let value = single_argument("String", arguments)?;
    match value {
        Value::String(text) => Ok(Value::String(text)),
        Value::Number(number) => Ok(Value::String(number.to_string().into())),
        Value::Bool(value) => Ok(Value::String(value.to_string().into())),
        Value::Null => Ok(Value::String("null".into())),
        other => Err(argument_error(
            "String",
            "Number, String, Bool or Null",
            &other,
        )),
    }
}

fn call_number(arguments: Vec<Value>) -> Result<Value, Error> {
    let value = single_argument("Number", arguments)?;
    match value {
        Value::Number(number) => Ok(Value::Number(number)),
        Value::String(text) => match text.parse() {
            Ok(number) => Ok(Value::Number(number)),
            Err(_) => Err(Error::InvalidNumber(text.to_string())),
        },
        other => Err(argument_error("Number", "String or Number", &other)),
    }
}

fn call_len(arguments: Vec<Value>) -> Result<Value, Error> {
    let value = single_argument("len", arguments)?;
    match value {
        Value::Array(items) => Ok(Value::Number(items.len() as f64)),
        other => Err(argument_error("len", "Array", &other)),
    }
}

fn single_argument(name: &'static str, mut arguments: Vec<Value>) -> Result<Value, Error> {
    if arguments.len() != 1 {
        return Err(Error::Arity {
            name: name.to_owned(),
            expected: "one",
        });
    }
    Ok(arguments.remove(0))
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

pub fn read_field(target: &Value, name: &str) -> Result<Value, Error> {
    match target {
        Value::Object(fields) => fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| Error::MissingField(name.to_owned())),
        other => Err(expected(".", "Object", other)),
    }
}

pub fn assign_field(target: &mut Value, name: &str, value: Value) -> Result<(), Error> {
    match target {
        Value::Object(fields) => match Rc::make_mut(fields)
            .iter_mut()
            .find(|(field, _)| field == name)
        {
            Some((_, field)) => {
                *field = value;
                Ok(())
            }
            None => Err(Error::MissingField(name.to_owned())),
        },
        other => Err(expected(".", "Object", other)),
    }
}

pub fn read_index(target: &Value, index: &Value) -> Result<Value, Error> {
    match target {
        Value::Array(items) => {
            let Value::Number(_) = index else {
                return Err(expected("[]", "Number", index));
            };
            let index = array_index(items.len(), index)?;
            Ok(items[index].clone())
        }
        Value::Object(fields) => {
            let Value::String(key) = index else {
                return Err(expected("[]", "String", index));
            };
            fields
                .iter()
                .find(|(field, _)| field.as_str() == key.as_ref())
                .map(|(_, value)| value.clone())
                .ok_or_else(|| Error::MissingField(key.to_string()))
        }
        other => Err(expected("[]", "Array or Object", other)),
    }
}

pub fn assign_index(target: &mut Value, index: &Value, value: Value) -> Result<(), Error> {
    match target {
        Value::Array(items) => {
            let Value::Number(_) = index else {
                return Err(expected("[]", "Number", index));
            };
            let index = array_index(items.len(), index)?;
            Rc::make_mut(items)[index] = value;
            Ok(())
        }
        Value::Object(fields) => {
            let Value::String(key) = index else {
                return Err(expected("[]", "String", index));
            };
            match Rc::make_mut(fields)
                .iter_mut()
                .find(|(field, _)| field.as_str() == key.as_ref())
            {
                Some((_, field)) => {
                    *field = value;
                    Ok(())
                }
                None => Err(Error::MissingField(key.to_string())),
            }
        }
        other => Err(expected("[]", "Array or Object", other)),
    }
}

fn array_index(length: usize, index: &Value) -> Result<usize, Error> {
    let Value::Number(index) = index else {
        return Err(Error::IndexNotInteger);
    };
    if !index.is_finite() || *index < 0.0 || index.fract() != 0.0 {
        return Err(Error::IndexNotInteger);
    }
    let index = *index as usize;
    if index >= length {
        return Err(Error::OutOfBounds { index, length });
    }
    Ok(index)
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

pub fn template(values: Vec<Value>) -> Result<String, Error> {
    let mut text = String::new();
    for value in values {
        match value {
            Value::String(chunk) => text.push_str(chunk.as_ref()),
            Value::Number(number) => text.push_str(&number.to_string()),
            other => return Err(expected("${}", "String or Number", &other)),
        }
    }
    Ok(text)
}

fn eval_unary(component: &mut Component, op: UnaryOp, operand: &Expr) -> Result<Value, Error> {
    apply_unary(op, eval(component, operand)?)
}

pub fn apply_unary(op: UnaryOp, value: Value) -> Result<Value, Error> {
    match (op, value) {
        (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
        (UnaryOp::Neg, Value::Number(value)) => Ok(Value::Number(-value)),
        (UnaryOp::Not, other) => Err(expected("!", "Bool", &other)),
        (UnaryOp::Neg, other) => Err(expected("-", "Number", &other)),
    }
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

fn argument_error(name: &'static str, expected: &'static str, found: &Value) -> Error {
    Error::Argument {
        name,
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
    fn the_public_operations_match_the_interpreter() {
        assert_eq!(
            read_field(
                &Value::Object(vec![("a".to_owned(), Value::Number(1.0))].into()),
                "a"
            ),
            Ok(Value::Number(1.0))
        );
        assert_eq!(
            read_index(
                &Value::Array(vec![Value::Bool(true)].into()),
                &Value::Number(0.0)
            ),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            apply_binary(BinaryOp::Add, Value::Number(1.0), Value::Number(2.0)),
            Ok(Value::Number(3.0))
        );
        assert_eq!(
            apply_unary(UnaryOp::Neg, Value::Number(2.0)),
            Ok(Value::Number(-2.0))
        );
        assert_eq!(
            template(vec![Value::String("a".into()), Value::Number(2.0)]),
            Ok("a2".to_owned())
        );
        let mut object = Value::Object(vec![("a".to_owned(), Value::Null)].into());
        assert_eq!(assign_field(&mut object, "a", Value::Bool(true)), Ok(()));
        assert_eq!(read_field(&object, "a"), Ok(Value::Bool(true)));
    }

    #[test]
    fn op_name_spells_every_operator() {
        let named = [
            (BinaryOp::Add, "+"),
            (BinaryOp::Sub, "-"),
            (BinaryOp::Mul, "*"),
            (BinaryOp::Div, "/"),
            (BinaryOp::Rem, "%"),
            (BinaryOp::Eq, "=="),
            (BinaryOp::NotEq, "!="),
            (BinaryOp::Lt, "<"),
            (BinaryOp::Gt, ">"),
            (BinaryOp::Le, "<="),
            (BinaryOp::Ge, ">="),
            (BinaryOp::And, "&&"),
            (BinaryOp::Or, "||"),
        ];
        for (op, name) in named {
            assert_eq!(op_name(op), name);
        }
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
