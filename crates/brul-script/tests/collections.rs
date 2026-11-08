use brul_script::{Component, Error, Value, eval};
use brul_syntax::parser::parse_expression;

fn evaluate(source: &str) -> Result<Value, Error> {
    eval(&mut Component::new(), &parse_expression(source).unwrap())
}

fn object(fields: &[(&str, Value)]) -> Value {
    Value::Object(
        fields
            .iter()
            .map(|(name, value)| ((*name).to_owned(), value.clone()))
            .collect(),
    )
}

#[test]
fn array_literal_evaluates_elements() {
    assert_eq!(
        evaluate("[1, 2 + 3, \"x\"]"),
        Ok(Value::Array(vec![
            Value::Number(1.0),
            Value::Number(5.0),
            Value::String("x".to_owned()),
        ]))
    );
}

#[test]
fn object_literal_evaluates_fields() {
    assert_eq!(
        evaluate("{ name: \"Chrome\", flag: true }"),
        Ok(object(&[
            ("name", Value::String("Chrome".to_owned())),
            ("flag", Value::Bool(true)),
        ]))
    );
}

#[test]
fn array_index_reads_element() {
    assert_eq!(evaluate("[10, 20, 30][1]"), Ok(Value::Number(20.0)));
}

#[test]
fn fields_and_indexes_chain() {
    assert_eq!(
        evaluate("{ items: [{ name: \"a\" }] }.items[0].name"),
        Ok(Value::String("a".to_owned()))
    );
}

#[test]
fn object_reads_string_key() {
    let mut component = Component::new();
    component
        .declare(
            "browser",
            object(&[("profile-dir", Value::String("/tmp".to_owned()))]),
            false,
        )
        .unwrap();

    assert_eq!(
        eval(
            &mut component,
            &parse_expression("browser[\"profile-dir\"]").unwrap()
        ),
        Ok(Value::String("/tmp".to_owned()))
    );
}

#[test]
fn missing_field_is_an_error() {
    assert_eq!(
        evaluate("{ a: 1 }.b"),
        Err(Error::MissingField("b".to_owned()))
    );
}

#[test]
fn array_index_is_checked() {
    assert_eq!(
        evaluate("[1][3]"),
        Err(Error::OutOfBounds {
            index: 3,
            length: 1
        })
    );
    assert_eq!(evaluate("[1][0.5]"), Err(Error::IndexNotInteger));
}

#[test]
fn field_and_index_require_the_right_target() {
    assert_eq!(
        evaluate("\"x\".name"),
        Err(Error::Expected {
            operation: ".",
            expected: "Object",
            found: "String",
        })
    );
    assert_eq!(
        evaluate("1[0]"),
        Err(Error::Expected {
            operation: "[]",
            expected: "Array or Object",
            found: "Number",
        })
    );
}

#[test]
fn template_substitutes_text_and_numbers() {
    assert_eq!(
        evaluate("`n=${1 + 2}`"),
        Ok(Value::String("n=3".to_owned()))
    );
    assert_eq!(
        evaluate("`a${\"b\"}c`"),
        Ok(Value::String("abc".to_owned()))
    );
}

#[test]
fn template_rejects_other_types() {
    assert_eq!(
        evaluate("`${true}`"),
        Err(Error::Expected {
            operation: "${}",
            expected: "String or Number",
            found: "Bool",
        })
    );
}

#[test]
fn assignment_updates_array_elements() {
    let mut component = Component::new();
    component
        .declare(
            "values",
            Value::Array(vec![
                Value::Number(1.0),
                Value::Number(2.0),
                Value::Number(3.0),
            ]),
            false,
        )
        .unwrap();

    assert_eq!(
        eval(&mut component, &parse_expression("values[1] = 5").unwrap()),
        Ok(Value::Number(5.0))
    );
    eval(&mut component, &parse_expression("values[0] += 4").unwrap()).unwrap();

    assert_eq!(
        component.read("values"),
        Ok(Value::Array(vec![
            Value::Number(5.0),
            Value::Number(5.0),
            Value::Number(3.0),
        ]))
    );
}

#[test]
fn assignment_updates_object_fields() {
    let mut component = Component::new();
    component
        .declare(
            "settings",
            object(&[
                ("flag", Value::Bool(true)),
                ("always-ask", Value::Bool(true)),
            ]),
            false,
        )
        .unwrap();

    eval(
        &mut component,
        &parse_expression("settings.flag = false").unwrap(),
    )
    .unwrap();
    eval(
        &mut component,
        &parse_expression("settings[\"always-ask\"] = false").unwrap(),
    )
    .unwrap();

    assert_eq!(
        component.read("settings"),
        Ok(object(&[
            ("flag", Value::Bool(false)),
            ("always-ask", Value::Bool(false)),
        ]))
    );
}

#[test]
fn assignment_to_missing_field_is_an_error() {
    let mut component = Component::new();
    component
        .declare("settings", object(&[("a", Value::Number(1.0))]), false)
        .unwrap();

    assert_eq!(
        eval(&mut component, &parse_expression("settings.b = 2").unwrap()),
        Err(Error::MissingField("b".to_owned()))
    );
}
