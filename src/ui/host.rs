use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use brul_window::{WindowCommand, WindowKey};

use crate::{Error, Host, Value};

pub(crate) struct CommandHost {
    pub(crate) commands: Rc<RefCell<Vec<WindowCommand>>>,
    pub(crate) app: Option<Box<dyn Host>>,
    pub(crate) windows: Rc<HashMap<String, WindowKey>>,
    pub(crate) theme: Rc<RefCell<Option<String>>>,
}

impl Host for CommandHost {
    fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, Error> {
        tracing::debug!(name, arguments = arguments.len(), "host call");
        if name == "theme" {
            no_arguments(arguments, name)?;
            return Ok(match &*self.theme.borrow() {
                Some(theme) => Value::String(theme.clone().into()),
                None => Value::Null,
            });
        }
        let command = match name {
            "setTheme" => {
                let theme = one_argument(arguments, "setTheme")?;
                WindowCommand::SetTheme(theme)
            }
            "setAlwaysOnTop" => {
                let on_top = one_argument(arguments, "setAlwaysOnTop")?;
                WindowCommand::AlwaysOnTop(on_top)
            }
            "openWindow" => {
                let window: String = one_argument(arguments, "openWindow")?;
                let Some(key) = self.windows.get(window.as_str()) else {
                    return Err(Error::UnknownWindow(window));
                };
                WindowCommand::Open(*key)
            }
            "exit" | "closeWindow" => {
                no_arguments(arguments, name)?;
                WindowCommand::Close
            }
            "dragWindow" => {
                no_arguments(arguments, name)?;
                WindowCommand::Drag
            }
            "showWindow" => {
                no_arguments(arguments, name)?;
                WindowCommand::Show
            }
            "hideWindow" => {
                no_arguments(arguments, name)?;
                WindowCommand::Hide
            }
            "focusWindow" => {
                no_arguments(arguments, name)?;
                WindowCommand::Focus
            }
            _ => return self.delegate(name, arguments),
        };
        self.commands.borrow_mut().push(command);
        Ok(Value::Null)
    }

    fn every(&mut self, interval_ms: f64, function: &str) -> Result<(), Error> {
        match &mut self.app {
            Some(host) => host.every(interval_ms, function),
            None => Err(Error::Unsupported("every")),
        }
    }

    fn commands(&self) -> Vec<&'static str> {
        match &self.app {
            Some(app) => app.commands(),
            None => Vec::new(),
        }
    }
}

impl CommandHost {
    fn delegate(&mut self, name: &str, arguments: &[Value]) -> Result<Value, Error> {
        match &mut self.app {
            Some(host) => host.call(name, arguments),
            None => Err(Error::UnknownFunction(name.to_owned())),
        }
    }
}

fn no_arguments(arguments: &[Value], name: &str) -> Result<(), Error> {
    if arguments.is_empty() {
        Ok(())
    } else {
        Err(Error::Arity {
            name: name.to_owned(),
            expected: "no",
        })
    }
}

fn one_argument<T>(arguments: &[Value], name: &'static str) -> Result<T, Error>
where
    T: for<'a> TryFrom<&'a Value, Error = Error>,
{
    let [value] = arguments else {
        return Err(Error::Arity {
            name: name.to_owned(),
            expected: "one",
        });
    };
    T::try_from(value).map_err(|error| error.at_command(name))
}
