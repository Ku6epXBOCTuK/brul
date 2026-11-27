use brul_script::eval::{assign_field, assign_index, read_field, read_index, template as render};
use brul_script::{Component, Error, Object, Value};

const NAMES: &[&str] = &["browser", "values", "settings"];

fn component() -> Component {
    let mut component = Component::new();
    component.set_names(NAMES);
    component
}

fn object(fields: &[(&'static str, Value)]) -> Value {
    let mut builder = Object::builder();
    for (name, value) in fields {
        builder = builder.field(name, value.clone());
    }
    Value::Object(builder.into())
}

fn array(values: &[Value]) -> Value {
    Value::Array(values.to_vec().into())
}

fn number(value: f64) -> Value {
    Value::Number(value)
}

#[test]
fn array_literal_holds_its_elements() {
    let value = array(&[number(1.0), number(5.0), Value::String("x".into())]);
    assert_eq!(read_index(&value, &number(0.0)), Ok(number(1.0)));
    assert_eq!(read_index(&value, &number(1.0)), Ok(number(5.0)));
    assert_eq!(
        read_index(&value, &number(2.0)),
        Ok(Value::String("x".into()))
    );
}

#[test]
fn object_literal_holds_its_fields() {
    let value = object(&[
        ("name", Value::String("Chrome".into())),
        ("flag", Value::Bool(true)),
    ]);
    assert_eq!(
        read_field(&value, "name"),
        Ok(Value::String("Chrome".into()))
    );
    assert_eq!(read_field(&value, "flag"), Ok(Value::Bool(true)));
}

#[test]
fn array_index_reads_element() {
    assert_eq!(
        read_index(
            &array(&[number(10.0), number(20.0), number(30.0)]),
            &number(1.0)
        ),
        Ok(number(20.0))
    );
}

#[test]
fn fields_and_indexes_chain() {
    let value = object(&[(
        "items",
        array(&[object(&[("name", Value::String("a".into()))])]),
    )]);
    let items = read_field(&value, "items").unwrap();
    let first = read_index(&items, &number(0.0)).unwrap();
    assert_eq!(read_field(&first, "name"), Ok(Value::String("a".into())));
}

#[test]
fn object_reads_string_key() {
    let value = object(&[("profile-dir", Value::String("/tmp".into()))]);
    assert_eq!(
        read_index(&value, &Value::String("profile-dir".into())),
        Ok(Value::String("/tmp".into()))
    );
}

#[test]
fn missing_field_is_an_error() {
    assert_eq!(
        read_field(&object(&[("a", number(1.0))]), "b"),
        Err(Error::MissingField("b".to_owned()))
    );
}

#[test]
fn array_index_is_checked() {
    assert_eq!(
        read_index(&array(&[number(1.0)]), &number(3.0)),
        Err(Error::OutOfBounds {
            index: 3,
            length: 1
        })
    );
    assert_eq!(
        read_index(&array(&[number(1.0)]), &number(0.5)),
        Err(Error::IndexNotInteger)
    );
}

#[test]
fn field_and_index_require_the_right_target() {
    assert_eq!(
        read_field(&Value::String("x".into()), "name"),
        Err(Error::Expected {
            operation: ".",
            expected: "Object",
            found: "String",
        })
    );
    assert_eq!(
        read_index(&number(1.0), &number(0.0)),
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
        render(vec![Value::String("n=".into()), number(3.0)]),
        Ok("n=3".to_owned())
    );
    assert_eq!(
        render(vec![
            Value::String("a".into()),
            Value::String("b".into()),
            Value::String("c".into()),
        ]),
        Ok("abc".to_owned())
    );
}

#[test]
fn template_rejects_other_types() {
    assert_eq!(
        render(vec![Value::Bool(true)]),
        Err(Error::Expected {
            operation: "${}",
            expected: "String or Number",
            found: "Bool",
        })
    );
}

#[test]
fn assignment_updates_array_elements() {
    let mut component = component();
    component
        .declare(
            "values",
            array(&[number(1.0), number(2.0), number(3.0)]),
            false,
        )
        .unwrap();

    let mut values = component.read("values").unwrap();
    assign_index(&mut values, &number(1.0), number(5.0)).unwrap();
    component.assign("values", values).unwrap();

    let mut values = component.read("values").unwrap();
    let Value::Number(current) = read_index(&values, &number(0.0)).unwrap() else {
        panic!("element is a number");
    };
    assign_index(&mut values, &number(0.0), number(current + 4.0)).unwrap();
    component.assign("values", values).unwrap();

    assert_eq!(
        component.read("values"),
        Ok(array(&[number(5.0), number(5.0), number(3.0)]))
    );
}

#[test]
fn assignment_updates_object_fields() {
    let mut component = component();
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

    let mut settings = component.read("settings").unwrap();
    assign_field(&mut settings, "flag", Value::Bool(false)).unwrap();
    assign_index(
        &mut settings,
        &Value::String("always-ask".into()),
        Value::Bool(false),
    )
    .unwrap();
    component.assign("settings", settings).unwrap();

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
    let mut component = component();
    component
        .declare("settings", object(&[("a", number(1.0))]), false)
        .unwrap();

    let mut settings = component.read("settings").unwrap();
    assert_eq!(
        assign_field(&mut settings, "b", number(2.0)),
        Err(Error::MissingField("b".to_owned()))
    );
}
