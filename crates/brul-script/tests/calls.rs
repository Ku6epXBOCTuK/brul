use std::cell::RefCell;
use std::rc::Rc;

use brul_ident::{TimerId, TimerKind};
use brul_script::{Component, Error, Host, Value, eval};
use brul_syntax::parser::parse_expression;

type Calls = Rc<RefCell<Vec<(String, Vec<Value>)>>>;

#[derive(Clone, Default)]
struct FakeHost {
    calls: Calls,
}

impl Host for FakeHost {
    fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, Error> {
        self.calls
            .borrow_mut()
            .push((name.to_owned(), arguments.to_vec()));
        match name {
            "runBrowser" | "setTheme" | "exit" => Ok(Value::Null),
            "currentTheme" => Ok(Value::String("dark".into())),
            _ => Err(Error::UnknownFunction(name.to_owned())),
        }
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
    assert_eq!(evaluate_with(&host, "setTheme(\"dark\")"), Ok(Value::Null));
    assert_eq!(evaluate_with(&host, "exit()"), Ok(Value::Null));
    assert_eq!(
        host.calls.borrow().as_slice(),
        &[
            ("setTheme".to_owned(), vec![Value::String("dark".into())]),
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
fn every_registers_function_name_with_a_host() {
    let mut component = Component::new();
    component.set_host(FakeHost::default());
    assert_eq!(
        eval(
            &mut component,
            &parse_expression("every(3000, refresh)").unwrap()
        ),
        Ok(Value::Timer(TimerId::new(0)))
    );
    let timers = component.take_timers();
    assert_eq!(timers.len(), 1);
    assert_eq!(timers[0].id, TimerId::new(0));
    assert_eq!(timers[0].kind, TimerKind::Every);
    assert_eq!(timers[0].interval_ms, 3000.0);
    assert_eq!(timers[0].function.as_str(), "refresh");
}

#[test]
fn every_registers_function_name_without_a_host() {
    let mut component = Component::new();
    assert_eq!(
        eval(
            &mut component,
            &parse_expression("every(250, tick)").unwrap()
        ),
        Ok(Value::Timer(TimerId::new(0)))
    );
    let timers = component.take_timers();
    assert_eq!(timers.len(), 1);
    assert_eq!(timers[0].id, TimerId::new(0));
    assert_eq!(timers[0].kind, TimerKind::Every);
    assert_eq!(timers[0].interval_ms, 250.0);
    assert_eq!(timers[0].function.as_str(), "tick");
}

#[test]
fn every_requires_number_and_function_name() {
    assert_eq!(
        failure("every(\"3000\", refresh)"),
        Error::Argument {
            name: "every",
            expected: "Number",
            found: "String",
        }
    );
    assert_eq!(
        failure("every(3000, 42)"),
        Error::Argument {
            name: "every",
            expected: "function name",
            found: "expression",
        }
    );
    assert_eq!(
        failure("every(3000)"),
        Error::Arity {
            name: "every".to_owned(),
            expected: "two",
        }
    );
}

#[test]
fn after_registers_a_one_shot_timer() {
    let mut component = Component::new();
    assert_eq!(
        eval(
            &mut component,
            &parse_expression("after(100, tick)").unwrap()
        ),
        Ok(Value::Timer(TimerId::new(0)))
    );
    let timers = component.take_timers();
    assert_eq!(timers.len(), 1);
    assert_eq!(timers[0].id, TimerId::new(0));
    assert_eq!(timers[0].kind, TimerKind::Once);
    assert_eq!(timers[0].interval_ms, 100.0);
}

#[test]
fn cancel_queues_the_timer_id() {
    let mut component = Component::new();
    assert_eq!(
        eval(
            &mut component,
            &parse_expression("cancel(after(100, tick))").unwrap()
        ),
        Ok(Value::Null)
    );
    assert_eq!(component.take_cancels(), vec![TimerId::new(0)]);
    assert_eq!(component.take_timers().len(), 1);
}

#[test]
fn method_call_is_not_supported() {
    assert_eq!(failure("theme.apply()"), Error::Unsupported("method call"));
}
