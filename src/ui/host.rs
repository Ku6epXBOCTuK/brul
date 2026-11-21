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
        if let Some(result) = self.framework_call(name, arguments) {
            return result;
        }
        self.delegate(name, arguments)
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

include!(concat!(env!("OUT_DIR"), "/lang_builtin_host.rs"));
