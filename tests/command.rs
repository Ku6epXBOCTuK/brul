use brul::{Error, Fields, Host, Object, Value, command, host};

#[derive(Debug, PartialEq)]
enum Action {
    Hide,
    Show,
}

impl TryFrom<&Value> for Action {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value.as_text() {
            Some("hide") => Ok(Self::Hide),
            Some("show") => Ok(Self::Show),
            _ => Err(Error::conversion("'hide' or 'show'", value)),
        }
    }
}

#[derive(Debug, PartialEq)]
struct Draft {
    name: String,
    port: usize,
}

impl TryFrom<&Value> for Draft {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let mut fields = Fields::new(value)?;
        Ok(Self {
            name: fields.take("name")?,
            port: fields.take("port")?,
        })
    }
}

struct TestHost {
    calls: usize,
}

#[command]
fn act(
    host: &mut TestHost,
    action: Action,
    draft: Draft,
    label: &str,
    ratio: f64,
    flag: bool,
) -> Result<Value, Error> {
    host.calls += 1;
    let action = match action {
        Action::Hide => "hide",
        Action::Show => "show",
    };
    Ok(Value::from(format!(
        "{action} {} {} {label} {ratio} {flag}",
        draft.name, draft.port
    )))
}

host! { TestHost: act }

fn draft(port: Option<Value>) -> Value {
    let mut object = Object::builder().field("name", Value::from("garden"));
    if let Some(port) = port {
        object = object.field("port", port);
    }
    Value::Object(object.into())
}

fn arguments(action: Value, port: Option<Value>) -> Vec<Value> {
    vec![
        action,
        draft(port),
        Value::from("tag"),
        Value::from(2.5),
        Value::from(true),
    ]
}

fn call(arguments: &[Value]) -> Result<Value, Error> {
    TestHost { calls: 0 }.call("act", arguments)
}

#[test]
fn typed_arguments_reach_the_command() {
    let value = call(&arguments(Value::from("hide"), Some(Value::from(8080.0)))).expect("command");

    assert_eq!(
        value,
        Value::from("hide garden 8080 tag 2.5 true".to_owned())
    );
}

#[test]
fn an_enum_argument_takes_its_script_value() {
    let value = call(&arguments(Value::from("show"), Some(Value::from(1.0)))).expect("command");

    assert!(matches!(value, Value::String(text) if text.starts_with("show garden 1 ")));
}

#[test]
fn a_wrong_argument_names_the_command() {
    assert_eq!(
        call(&arguments(Value::from(1.0), Some(Value::from(1.0)))),
        Err(Error::Argument {
            name: "act",
            expected: "'hide' or 'show'",
            found: "Number",
        })
    );
}

#[test]
fn a_missing_field_names_the_field() {
    assert_eq!(
        call(&arguments(Value::from("hide"), None)),
        Err(Error::MissingField("port".to_owned()))
    );
}

#[test]
fn the_generated_host_lists_its_commands() {
    assert_eq!(TestHost { calls: 0 }.commands(), vec!["act"]);
}

#[test]
fn a_script_call_to_the_command_resolves() {
    let program = brul_syntax::script::parse(
        "let outcome: Object = act(\"hide\", { name: \"garden\", port: 1 });",
    )
    .expect("script");

    brul_syntax::script::validate(&program, &TestHost { calls: 0 }.commands())
        .expect("the command is known");
}

#[test]
fn a_short_argument_list_is_an_arity_error() {
    assert_eq!(
        call(&arguments(Value::from("hide"), None)[..2]),
        Err(Error::Arity {
            name: "act".to_owned(),
            expected: "the declared number of",
        })
    );
}
