use std::cell::RefCell;
use std::rc::Rc;

use brul_script::eval::call;
use brul_script::{Component, Error, Host, TimerId, TimerKind, Value};

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

fn call_builtin(name: &str, arguments: Vec<Value>) -> Result<Value, Error> {
    call(&mut Component::new(), name, arguments)
}

fn call_with(host: &FakeHost, name: &str, arguments: Vec<Value>) -> Result<Value, Error> {
    let mut component = Component::new();
    component.set_host(host.clone());
    call(&mut component, name, arguments)
}

#[test]
fn string_converts_scalars_to_text() {
    assert_eq!(
        call_builtin("String", vec![Value::Number(42.0)]),
        Ok(Value::String("42".into()))
    );
    assert_eq!(
        call_builtin("String", vec![Value::Bool(true)]),
        Ok(Value::String("true".into()))
    );
    assert_eq!(
        call_builtin("String", vec![Value::Null]),
        Ok(Value::String("null".into()))
    );
    assert_eq!(
        call_builtin("String", vec![Value::String("hi".into())]),
        Ok(Value::String("hi".into()))
    );
}

#[test]
fn string_rejects_composites() {
    assert_eq!(
        call_builtin(
            "String",
            vec![Value::Array(vec![Value::Number(1.0)].into())]
        ),
        Err(Error::Argument {
            name: "String",
            expected: "Number, String, Bool or Null",
            found: "Array",
        })
    );
}

#[test]
fn number_parses_strings() {
    assert_eq!(
        call_builtin("Number", vec![Value::String("2.5".into())]),
        Ok(Value::Number(2.5))
    );
    assert_eq!(
        call_builtin("Number", vec![Value::Number(3.0)]),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn number_rejects_unparsable_text() {
    assert_eq!(
        call_builtin("Number", vec![Value::String("abc".into())]),
        Err(Error::InvalidNumber("abc".to_owned()))
    );
}

#[test]
fn number_rejects_non_scalars() {
    assert_eq!(
        call_builtin("Number", vec![Value::Bool(true)]),
        Err(Error::Argument {
            name: "Number",
            expected: "String or Number",
            found: "Bool",
        })
    );
}

#[test]
fn len_returns_array_length() {
    assert_eq!(
        call_builtin(
            "len",
            vec![Value::Array(
                vec![Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)].into()
            )]
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        call_builtin("len", vec![Value::Array(vec![].into())]),
        Ok(Value::Number(0.0))
    );
}

#[test]
fn len_rejects_non_array() {
    assert_eq!(
        call_builtin("len", vec![Value::String("abc".into())]),
        Err(Error::Argument {
            name: "len",
            expected: "Array",
            found: "String",
        })
    );
}

#[test]
fn builtins_check_arity() {
    assert_eq!(
        call_builtin("String", vec![]),
        Err(Error::Arity {
            name: "String".to_owned(),
            expected: "one",
        })
    );
    assert_eq!(
        call_builtin(
            "len",
            vec![Value::Array(vec![].into()), Value::Array(vec![].into()),]
        ),
        Err(Error::Arity {
            name: "len".to_owned(),
            expected: "one",
        })
    );
}

#[test]
fn host_function_receives_arguments_and_returns_value() {
    let host = FakeHost::default();
    assert_eq!(
        call_with(
            &host,
            "runBrowser",
            vec![Value::Number(2.0), Value::String("dark".into())]
        ),
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
        call_with(&host, "currentTheme", vec![]),
        Ok(Value::String("dark".into()))
    );
}

#[test]
fn side_effect_builtins_go_to_the_host() {
    let host = FakeHost::default();
    assert_eq!(
        call_with(&host, "setTheme", vec![Value::String("dark".into())]),
        Ok(Value::Null)
    );
    assert_eq!(call_with(&host, "exit", vec![]), Ok(Value::Null));
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
        call_builtin("currentTheme", vec![]),
        Err(Error::UnknownFunction("currentTheme".to_owned()))
    );
}

#[test]
fn every_registers_function_name_with_a_host() {
    let mut component = Component::new();
    component.set_host(FakeHost::default());
    assert_eq!(component.every(3000.0, "refresh"), Ok(TimerId::new(0)));
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
    assert_eq!(component.every(250.0, "tick"), Ok(TimerId::new(0)));
    let timers = component.take_timers();
    assert_eq!(timers.len(), 1);
    assert_eq!(timers[0].id, TimerId::new(0));
    assert_eq!(timers[0].kind, TimerKind::Every);
    assert_eq!(timers[0].interval_ms, 250.0);
    assert_eq!(timers[0].function.as_str(), "tick");
}

#[test]
fn after_registers_a_one_shot_timer() {
    let mut component = Component::new();
    assert_eq!(component.after(100.0, "tick"), Ok(TimerId::new(0)));
    let timers = component.take_timers();
    assert_eq!(timers.len(), 1);
    assert_eq!(timers[0].id, TimerId::new(0));
    assert_eq!(timers[0].kind, TimerKind::Once);
    assert_eq!(timers[0].interval_ms, 100.0);
}

#[test]
fn cancel_queues_the_timer_id() {
    let mut component = Component::new();
    let id = component.after(100.0, "tick").unwrap();
    assert_eq!(component.cancel(id), Ok(()));
    assert_eq!(component.take_cancels(), vec![TimerId::new(0)]);
    assert_eq!(component.take_timers().len(), 1);
}
