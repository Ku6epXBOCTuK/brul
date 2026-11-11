use std::cell::RefCell;
use std::rc::Rc;

use brul_script::{Component, Error, Host, Value, eval};
use brul_syntax::parser::parse_expression;

type Calls = Rc<RefCell<Vec<(String, Vec<Value>)>>>;
type Intervals = Rc<RefCell<Vec<(f64, String)>>>;

#[derive(Clone, Default)]
struct FakeHost {
    calls: Calls,
    intervals: Intervals,
}

impl Host for FakeHost {
    fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, Error> {
        self.calls
            .borrow_mut()
            .push((name.to_owned(), arguments.to_vec()));
        match name {
            "runBrowser" | "set_theme" | "exit" => Ok(Value::Null),
            "currentTheme" => Ok(Value::String("dark".into())),
            _ => Err(Error::UnknownFunction(name.to_owned())),
        }
    }

    fn every(&mut self, interval_ms: f64, function: &str) -> Result<(), Error> {
        self.intervals
            .borrow_mut()
            .push((interval_ms, function.to_owned()));
        Ok(())
    }
}

fn evaluate(source: &str) -> Result<Value, Error> {
    eval(&mut Component::new(), &parse_expression(source).unwrap())
}

fn evaluate_with(host: &FakeHost, source: &str) -> Result<Value, Error> {
    let mut component = Component::new();
    component.set_host(host.clone());
    eval(&mut component, &parse_expression(source).unwrap())
}

fn failure(source: &str) -> Error {
    match evaluate(source).unwrap_err() {
        Error::At(_, error) => *error,
        error => error,
    }
}

fn failure_with(host: &FakeHost, source: &str) -> Error {
    match evaluate_with(host, source).unwrap_err() {
        Error::At(_, error) => *error,
        error => error,
    }
}

#[test]
fn string_converts_scalars_to_text() {
    assert_eq!(evaluate("String(42)"), Ok(Value::String("42".into())));
    assert_eq!(evaluate("String(true)"), Ok(Value::String("true".into())));
    assert_eq!(evaluate("String(null)"), Ok(Value::String("null".into())));
    assert_eq!(evaluate("String(\"hi\")"), Ok(Value::String("hi".into())));
}

#[test]
fn string_rejects_composites() {
    assert_eq!(
        failure("String([1])"),
        Error::Argument {
            name: "String",
            expected: "Number, String, Bool or Null",
            found: "Array",
        }
    );
}

#[test]
fn number_parses_strings() {
    assert_eq!(evaluate("Number(\"2.5\")"), Ok(Value::Number(2.5)));
    assert_eq!(evaluate("Number(3)"), Ok(Value::Number(3.0)));
}

#[test]
fn number_rejects_unparsable_text() {
    assert_eq!(
        failure("Number(\"abc\")"),
        Error::InvalidNumber("abc".to_owned())
    );
}

#[test]
fn number_rejects_non_scalars() {
    assert_eq!(
        failure("Number(true)"),
        Error::Argument {
            name: "Number",
            expected: "String or Number",
            found: "Bool",
        }
    );
}

#[test]
fn len_returns_array_length() {
    assert_eq!(evaluate("len([1, 2, 3])"), Ok(Value::Number(3.0)));
    assert_eq!(evaluate("len([])"), Ok(Value::Number(0.0)));
}

#[test]
fn len_rejects_non_array() {
    assert_eq!(
        failure("len(\"abc\")"),
        Error::Argument {
            name: "len",
            expected: "Array",
            found: "String",
        }
    );
}

#[test]
fn builtins_check_arity() {
    assert_eq!(
        failure("String()"),
        Error::Arity {
            name: "String".to_owned(),
            expected: "one",
        }
    );
    assert_eq!(
        failure("len([1], [2])"),
        Error::Arity {
            name: "len".to_owned(),
            expected: "one",
        }
    );
}

#[test]
fn host_function_receives_arguments_and_returns_value() {
    let host = FakeHost::default();
    assert_eq!(
        evaluate_with(&host, "runBrowser(2, \"dark\")"),
        Ok(Value::Null)
    );
    assert_eq!(
        host.calls.borrow().as_slice(),
        &[(
            "runBrowser".to_owned(),
            vec![Value::Number(2.0), Value::String("dark".into())]
        )]
    );
    assert_eq!(
        evaluate_with(&host, "currentTheme()"),
        Ok(Value::String("dark".into()))
    );
}

#[test]
fn side_effect_builtins_go_to_the_host() {
    let host = FakeHost::default();
    assert_eq!(evaluate_with(&host, "set_theme(\"dark\")"), Ok(Value::Null));
    assert_eq!(evaluate_with(&host, "exit()"), Ok(Value::Null));
    assert_eq!(
        host.calls.borrow().as_slice(),
        &[
            ("set_theme".to_owned(), vec![Value::String("dark".into())]),
            ("exit".to_owned(), vec![]),
        ]
    );
}

#[test]
fn unknown_function_without_host_is_reported() {
    assert_eq!(
        failure("currentTheme()"),
        Error::UnknownFunction("currentTheme".to_owned())
    );
}

#[test]
fn every_registers_function_name() {
    let host = FakeHost::default();
    assert_eq!(
        evaluate_with(&host, "every(3000, refresh)"),
        Ok(Value::Null)
    );
    assert_eq!(
        host.intervals.borrow().as_slice(),
        &[(3000.0, "refresh".into())]
    );
}

#[test]
fn every_requires_number_and_function_name() {
    let host = FakeHost::default();
    assert_eq!(
        failure_with(&host, "every(\"3000\", refresh)"),
        Error::Argument {
            name: "every",
            expected: "Number",
            found: "String",
        }
    );
    assert_eq!(
        failure_with(&host, "every(3000, 42)"),
        Error::Argument {
            name: "every",
            expected: "function name",
            found: "expression",
        }
    );
    assert_eq!(
        failure_with(&host, "every(3000)"),
        Error::Arity {
            name: "every".to_owned(),
            expected: "two",
        }
    );
}

#[test]
fn method_call_is_not_supported() {
    assert_eq!(failure("theme.apply()"), Error::Unsupported("method call"));
}
