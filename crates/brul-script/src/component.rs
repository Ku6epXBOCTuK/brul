use brul_reactive::Signal;
use brul_syntax::script::Function;

use crate::environment::Environment;
use crate::error::Error;
use crate::execute::execute;
use crate::host::Host;
use crate::value::Value;

struct Property {
    name: String,
    value: Signal<Value>,
}

pub struct Component {
    environment: Environment,
    properties: Vec<Property>,
    functions: Vec<Function>,
    host: Option<Box<dyn Host>>,
    revision: Signal<u64>,
}

impl Default for Component {
    fn default() -> Self {
        Self {
            environment: Environment::default(),
            properties: Vec::new(),
            functions: Vec::new(),
            host: None,
            revision: Signal::new(0),
        }
    }
}

impl Component {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_host(&mut self, host: impl Host + 'static) {
        self.host = Some(Box::new(host));
    }

    pub fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, Error> {
        match &mut self.host {
            Some(host) => host.call(name, arguments),
            None => Err(Error::UnknownFunction(name.to_owned())),
        }
    }

    pub fn every(&mut self, interval_ms: f64, function: &str) -> Result<(), Error> {
        match &mut self.host {
            Some(host) => host.every(interval_ms, function),
            None => Err(Error::UnknownFunction("every".to_owned())),
        }
    }

    pub fn define_function(&mut self, function: Function) -> Result<(), Error> {
        if self.has_function(&function.name) {
            return Err(Error::FunctionRedeclared(function.name));
        }
        self.functions.push(function);
        Ok(())
    }

    pub fn has_function(&self, name: &str) -> bool {
        self.functions.iter().any(|function| function.name == name)
    }

    pub fn call_function(&mut self, name: &str, arguments: Vec<Value>) -> Result<Value, Error> {
        let Some(function) = self
            .functions
            .iter()
            .find(|function| function.name == name)
            .cloned()
        else {
            return Err(Error::UnknownFunction(name.to_owned()));
        };
        if function.params.len() != arguments.len() {
            return Err(Error::FunctionArity {
                name: function.name,
                expected: function.params.len(),
                found: arguments.len(),
            });
        }
        self.environment.push_frame();
        for (param, argument) in function.params.iter().zip(arguments) {
            self.environment.declare(param, argument, false)?;
        }
        let result = execute(self, &function.body);
        self.environment.pop_frame();
        result.map(|()| Value::Null)
    }

    pub fn declare(&mut self, name: &str, value: Value, constant: bool) -> Result<(), Error> {
        self.environment.declare(name, value, constant)?;
        self.changed();
        Ok(())
    }

    pub fn declare_property(&mut self, name: &str, value: Value) -> Result<(), Error> {
        if self.property(name).is_some() {
            return Err(Error::Redeclared(name.to_owned()));
        }
        self.properties.push(Property {
            name: name.to_owned(),
            value: Signal::new(value),
        });
        self.changed();
        Ok(())
    }

    pub fn track(&self) {
        let _ = self.revision.get();
    }

    pub fn read(&self, name: &str) -> Result<Value, Error> {
        if let Ok(value) = self.environment.get(name) {
            return Ok(value.clone());
        }
        if let Some(property) = self.property(name) {
            return Ok(property.value.get());
        }
        Err(Error::Undefined(name.to_owned()))
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), Error> {
        if self.environment.contains(name) {
            self.environment.assign(name, value)?;
            self.changed();
            return Ok(());
        }
        if let Some(property) = self.property(name) {
            property.value.set(value);
            self.changed();
            return Ok(());
        }
        Err(Error::Undefined(name.to_owned()))
    }

    fn changed(&self) {
        self.revision.set(self.revision.get() + 1);
    }

    fn property(&self, name: &str) -> Option<&Property> {
        self.properties
            .iter()
            .find(|property| property.name == name)
    }
}
