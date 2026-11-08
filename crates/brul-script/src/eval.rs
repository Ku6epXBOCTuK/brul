use brul_syntax::ast::{AssignOp, BinaryOp, Expr, ExprKind, TemplatePart, UnaryOp};

use crate::component::Component;
use crate::error::Error;
use crate::value::Value;

pub fn eval(component: &mut Component, expression: &Expr) -> Result<Value, Error> {
    evaluate(component, expression).map_err(|error| error.at(expression.position))
}

fn evaluate(component: &mut Component, expression: &Expr) -> Result<Value, Error> {
    match &expression.kind {
        ExprKind::Number(value) => Ok(Value::Number(*value)),
        ExprKind::String(value) => Ok(Value::String(value.clone())),
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
            Ok(Value::Array(values))
        }
        ExprKind::Object(fields) => {
            let mut values = Vec::with_capacity(fields.len());
            for (name, value) in fields {
                values.push((name.clone(), eval(component, value)?));
            }
            Ok(Value::Object(values))
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
    match name.as_str() {
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
        Value::Number(number) => Ok(Value::String(number.to_string())),
        Value::Bool(value) => Ok(Value::String(value.to_string())),
        Value::Null => Ok(Value::String("null".to_owned())),
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
            Err(_) => Err(Error::InvalidNumber(text)),
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

fn read_field(target: &Value, name: &str) -> Result<Value, Error> {
    match target {
        Value::Object(fields) => fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| Error::MissingField(name.to_owned())),
        other => Err(expected(".", "Object", other)),
    }
}

fn assign_field(target: &mut Value, name: &str, value: Value) -> Result<(), Error> {
    match target {
        Value::Object(fields) => match fields.iter_mut().find(|(field, _)| field == name) {
            Some((_, field)) => {
                *field = value;
                Ok(())
            }
            None => Err(Error::MissingField(name.to_owned())),
        },
        other => Err(expected(".", "Object", other)),
    }
}

fn read_index(target: &Value, index: &Value) -> Result<Value, Error> {
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
                .find(|(field, _)| field == key)
                .map(|(_, value)| value.clone())
                .ok_or_else(|| Error::MissingField(key.clone()))
        }
        other => Err(expected("[]", "Array or Object", other)),
    }
}

fn assign_index(target: &mut Value, index: &Value, value: Value) -> Result<(), Error> {
    match target {
        Value::Array(items) => {
            let Value::Number(_) = index else {
                return Err(expected("[]", "Number", index));
            };
            let index = array_index(items.len(), index)?;
            items[index] = value;
            Ok(())
        }
        Value::Object(fields) => {
            let Value::String(key) = index else {
                return Err(expected("[]", "String", index));
            };
            match fields.iter_mut().find(|(field, _)| field == key) {
                Some((_, field)) => {
                    *field = value;
                    Ok(())
                }
                None => Err(Error::MissingField(key.clone())),
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
    let mut text = String::new();
    for part in parts {
        match part {
            TemplatePart::Text(chunk) => text.push_str(chunk),
            TemplatePart::Expr(expression) => match eval(component, expression)? {
                Value::String(chunk) => text.push_str(&chunk),
                Value::Number(number) => text.push_str(&number.to_string()),
                other => return Err(expected("${}", "String or Number", &other)),
            },
        }
    }
    Ok(Value::String(text))
}

fn eval_unary(component: &mut Component, op: UnaryOp, operand: &Expr) -> Result<Value, Error> {
    let value = eval(component, operand)?;
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

fn apply_binary(op: BinaryOp, left: Value, right: Value) -> Result<Value, Error> {
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
