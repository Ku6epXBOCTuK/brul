use brul_reactive::Signal;

use crate::environment::Environment;
use crate::error::Error;
use crate::value::Value;

struct Property {
    name: String,
    value: Signal<Value>,
}

#[derive(Default)]
pub struct Component {
    environment: Environment,
    properties: Vec<Property>,
}

impl Component {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn declare(&mut self, name: &str, value: Value, constant: bool) -> Result<(), Error> {
        self.environment.declare(name, value, constant)
    }

    pub fn declare_property(&mut self, name: &str, value: Value) -> Result<(), Error> {
        if self.property(name).is_some() {
            return Err(Error::Redeclared(name.to_owned()));
        }
        self.properties.push(Property {
            name: name.to_owned(),
            value: Signal::new(value),
        });
        Ok(())
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
            return self.environment.assign(name, value);
        }
        if let Some(property) = self.property(name) {
            property.value.set(value);
            return Ok(());
        }
        Err(Error::Undefined(name.to_owned()))
    }

    fn property(&self, name: &str) -> Option<&Property> {
        self.properties
            .iter()
            .find(|property| property.name == name)
    }
}
