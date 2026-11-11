use brul_script::{Component, Error, Value, eval};
use brul_syntax::parser::parse_expression;

fn evaluate(source: &str) -> Result<Value, Error> {
    eval(&mut Component::new(), &parse_expression(source).unwrap())
}

fn failure(source: &str) -> Error {
    match evaluate(source).unwrap_err() {
        Error::At(_, error) => *error,
        error => error,
    }
}

#[test]
fn literals_evaluate_to_their_values() {
    assert_eq!(evaluate("2.5"), Ok(Value::Number(2.5)));
    assert_eq!(evaluate("\"hi\""), Ok(Value::String("hi".into())));
    assert_eq!(evaluate("true"), Ok(Value::Bool(true)));
    assert_eq!(evaluate("false"), Ok(Value::Bool(false)));
    assert_eq!(evaluate("null"), Ok(Value::Null));
}

#[test]
fn arithmetic_follows_precedence() {
    assert_eq!(evaluate("1 + 2 * 3"), Ok(Value::Number(7.0)));
    assert_eq!(evaluate("(1 + 2) * 3"), Ok(Value::Number(9.0)));
    assert_eq!(evaluate("7 % 4"), Ok(Value::Number(3.0)));
    assert_eq!(evaluate("-(1 + 2)"), Ok(Value::Number(-3.0)));
}

#[test]
fn addition_concatenates_strings() {
    assert_eq!(evaluate("\"a\" + \"b\""), Ok(Value::String("ab".into())));
}

#[test]
fn addition_rejects_mixed_types() {
    assert_eq!(
        failure("1 + \"a\""),
        Error::Mismatch {
            operation: "+",
            left: "Number",
            right: "String",
        }
    );
}

#[test]
fn division_by_zero_is_an_error() {
    assert_eq!(failure("1 / 0"), Error::DivisionByZero);
    assert_eq!(failure("1 % 0"), Error::DivisionByZero);
    assert_eq!(Error::DivisionByZero.to_string(), "division by zero");
}

#[test]
fn equality_is_strict() {
    assert_eq!(evaluate("1 == 1"), Ok(Value::Bool(true)));
    assert_eq!(evaluate("1 == \"1\""), Ok(Value::Bool(false)));
    assert_eq!(evaluate("\"a\" != \"b\""), Ok(Value::Bool(true)));
    assert_eq!(evaluate("null == null"), Ok(Value::Bool(true)));
}

#[test]
fn ordering_compares_numbers() {
    assert_eq!(evaluate("2 < 3"), Ok(Value::Bool(true)));
    assert_eq!(evaluate("3 >= 3"), Ok(Value::Bool(true)));
    assert_eq!(evaluate("2 <= 3"), Ok(Value::Bool(true)));
    assert_eq!(
        failure("\"a\" < \"b\""),
        Error::Mismatch {
            operation: "<",
            left: "String",
            right: "String",
        }
    );
}

#[test]
fn negation_requires_a_bool() {
    assert_eq!(evaluate("!true"), Ok(Value::Bool(false)));
    assert_eq!(
        failure("!\"a\""),
        Error::Expected {
            operation: "!",
            expected: "Bool",
            found: "String",
        }
    );
}

#[test]
fn logic_requires_bools() {
    assert_eq!(evaluate("true && false"), Ok(Value::Bool(false)));
    assert_eq!(evaluate("false || true"), Ok(Value::Bool(true)));
    assert_eq!(
        failure("1 && true"),
        Error::Mismatch {
            operation: "&&",
            left: "Number",
            right: "Bool",
        }
    );
}

#[test]
fn ternary_picks_a_branch_by_bool() {
    assert_eq!(evaluate("true ? 1 : 2"), Ok(Value::Number(1.0)));
    assert_eq!(evaluate("false ? 1 : 2"), Ok(Value::Number(2.0)));
    assert_eq!(
        evaluate("1 < 2 ? \"a\" : \"b\""),
        Ok(Value::String("a".into()))
    );
    assert_eq!(
        failure("1 ? 1 : 2"),
        Error::Expected {
            operation: "?:",
            expected: "Bool",
            found: "Number",
        }
    );
}

#[test]
fn unsupported_expressions_report_their_kind() {
    assert_eq!(failure("browsers"), Error::Undefined("browsers".to_owned()));
    assert_eq!(failure("f(1)"), Error::UnknownFunction("f".to_owned()));
}
