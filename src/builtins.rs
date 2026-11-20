use brul_script::{Component, Error, HostCommand, TypeName, Value};

pub fn string(value: impl Into<Value>) -> Result<String, Error> {
    match value.into() {
        Value::String(text) => Ok(text.to_string()),
        Value::Number(number) => Ok(number.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Null => Ok("null".to_owned()),
        other => Err(Error::Argument {
            name: "String",
            expected: "Number, String, Bool or Null",
            found: other.type_name(),
        }),
    }
}

pub fn number(value: impl Into<Value>) -> Result<f64, Error> {
    match value.into() {
        Value::Number(number) => Ok(number),
        Value::String(text) => text
            .parse()
            .map_err(|_| Error::InvalidNumber(text.to_string())),
        other => Err(Error::Argument {
            name: "Number",
            expected: "String or Number",
            found: other.type_name(),
        }),
    }
}

pub fn len(value: impl Into<Value>) -> Result<f64, Error> {
    match value.into() {
        Value::Array(items) => Ok(items.len() as f64),
        other => Err(Error::Argument {
            name: "len",
            expected: "Array",
            found: other.type_name(),
        }),
    }
}

pub struct Theme;

impl HostCommand for Theme {
    type Arguments = ();
    type Output = String;

    const NAME: &'static str = "theme";

    fn invoke(
        component: &mut Component,
        _arguments: Self::Arguments,
    ) -> Result<Self::Output, Error> {
        let value = component.call("theme", &[])?;
        String::try_from(&value)
    }
}

macro_rules! unit_builtin {
    ($marker:ident, $name:literal) => {
        pub struct $marker;

        impl HostCommand for $marker {
            type Arguments = ();
            type Output = ();

            const NAME: &'static str = $name;

            fn invoke(
                component: &mut Component,
                _arguments: Self::Arguments,
            ) -> Result<Self::Output, Error> {
                let _ = component.call($name, &[])?;
                Ok(())
            }
        }
    };
}

macro_rules! value_builtin {
    ($marker:ident, $name:literal, $argument:ty) => {
        pub struct $marker;

        impl HostCommand for $marker {
            type Arguments = ($argument,);
            type Output = ();

            const NAME: &'static str = $name;

            fn invoke(
                component: &mut Component,
                arguments: Self::Arguments,
            ) -> Result<Self::Output, Error> {
                let _ = component.call($name, &[Value::from(arguments.0)])?;
                Ok(())
            }
        }
    };
}

value_builtin!(SetTheme, "setTheme", String);
value_builtin!(SetAlwaysOnTop, "setAlwaysOnTop", bool);
value_builtin!(OpenWindow, "openWindow", String);
unit_builtin!(Exit, "exit");
unit_builtin!(CloseWindow, "closeWindow");
unit_builtin!(DragWindow, "dragWindow");
unit_builtin!(ShowWindow, "showWindow");
unit_builtin!(HideWindow, "hideWindow");
unit_builtin!(FocusWindow, "focusWindow");
