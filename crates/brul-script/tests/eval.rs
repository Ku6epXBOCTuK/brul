use brul_script::eval::{apply_binary, apply_unary, call, template};
use brul_script::{Component, Error, Value};
use brul_syntax::ast::{BinaryOp, UnaryOp};

fn binary(op: BinaryOp, left: Value, right: Value) -> Result<Value, Error> {
    apply_binary(op, left, right)
}

fn number(value: f64) -> Value {
    Value::Number(value)
}

fn text(value: &str) -> Value {
    Value::String(value.into())
}

#[test]
fn arithmetic_follows_the_operator() {
    assert_eq!(
        binary(BinaryOp::Add, number(1.0), number(2.0)),
        Ok(number(3.0))
    );
    assert_eq!(
        binary(BinaryOp::Sub, number(7.0), number(4.0)),
        Ok(number(3.0))
    );
    assert_eq!(
        binary(BinaryOp::Mul, number(2.0), number(3.0)),
        Ok(number(6.0))
    );
    assert_eq!(
        binary(BinaryOp::Div, number(6.0), number(2.0)),
        Ok(number(3.0))
    );
    assert_eq!(
        binary(BinaryOp::Rem, number(7.0), number(4.0)),
        Ok(number(3.0))
    );
}

#[test]
fn addition_concatenates_strings() {
    assert_eq!(binary(BinaryOp::Add, text("a"), text("b")), Ok(text("ab")));
}

#[test]
fn addition_rejects_mixed_types() {
    assert_eq!(
        binary(BinaryOp::Add, number(1.0), text("a")),
        Err(Error::Mismatch {
            operation: "+",
            left: "Number",
            right: "String",
        })
    );
}

#[test]
fn division_by_zero_is_an_error() {
    assert_eq!(
        binary(BinaryOp::Div, number(1.0), number(0.0)),
        Err(Error::DivisionByZero)
    );
    assert_eq!(
        binary(BinaryOp::Rem, number(1.0), number(0.0)),
        Err(Error::DivisionByZero)
    );
    assert_eq!(Error::DivisionByZero.to_string(), "division by zero");
}

#[test]
fn equality_is_strict() {
    assert_eq!(
        binary(BinaryOp::Eq, number(1.0), number(1.0)),
        Ok(Value::Bool(true))
    );
    assert_eq!(
        binary(BinaryOp::Eq, number(1.0), text("1")),
        Ok(Value::Bool(false))
    );
    assert_eq!(
        binary(BinaryOp::NotEq, text("a"), text("b")),
        Ok(Value::Bool(true))
    );
    assert_eq!(
        binary(BinaryOp::Eq, Value::Null, Value::Null),
        Ok(Value::Bool(true))
    );
}

#[test]
fn ordering_compares_numbers() {
    assert_eq!(
        binary(BinaryOp::Lt, number(2.0), number(3.0)),
        Ok(Value::Bool(true))
    );
    assert_eq!(
        binary(BinaryOp::Ge, number(3.0), number(3.0)),
        Ok(Value::Bool(true))
    );
    assert_eq!(
        binary(BinaryOp::Le, number(2.0), number(3.0)),
        Ok(Value::Bool(true))
    );
    assert_eq!(
        binary(BinaryOp::Lt, text("a"), text("b")),
        Err(Error::Mismatch {
            operation: "<",
            left: "String",
            right: "String",
        })
    );
}

#[test]
fn negation_requires_a_bool() {
    assert_eq!(
        apply_unary(UnaryOp::Not, Value::Bool(true)),
        Ok(Value::Bool(false))
    );
    assert_eq!(
        apply_unary(UnaryOp::Not, text("a")),
        Err(Error::Expected {
            operation: "!",
            expected: "Bool",
            found: "String",
        })
    );
}

#[test]
fn logic_requires_bools() {
    assert_eq!(
        binary(BinaryOp::And, Value::Bool(true), Value::Bool(false)),
        Ok(Value::Bool(false))
    );
    assert_eq!(
        binary(BinaryOp::Or, Value::Bool(false), Value::Bool(true)),
        Ok(Value::Bool(true))
    );
    assert_eq!(
        binary(BinaryOp::And, number(1.0), Value::Bool(true)),
        Err(Error::Mismatch {
            operation: "&&",
            left: "Number",
            right: "Bool",
        })
    );
}

#[test]
fn templates_concatenate_text_and_numbers() {
    assert_eq!(
        template(vec![text("x="), number(2.0), text("!")]),
        Ok("x=2!".to_owned())
    );
    assert_eq!(
        template(vec![Value::Bool(true)]),
        Err(Error::Expected {
            operation: "${}",
            expected: "String or Number",
            found: "Bool",
        })
    );
}

#[test]
fn undefined_names_and_functions_are_reported() {
    assert_eq!(
        Component::new().read("browsers"),
        Err(Error::Undefined("browsers".to_owned()))
    );
    assert_eq!(
        call(&mut Component::new(), "f", vec![]),
        Err(Error::UnknownFunction("f".to_owned()))
    );
}
