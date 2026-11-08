use std::fmt;

use crate::value::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Undefined(String),
    Redeclared(String),
    Constant(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Undefined(name) => write!(f, "variable '{name}' is not defined"),
            Error::Redeclared(name) => write!(f, "variable '{name}' is already declared"),
            Error::Constant(name) => write!(f, "cannot assign to constant '{name}'"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, PartialEq)]
struct Binding {
    name: String,
    value: Value,
    constant: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Environment {
    bindings: Vec<Binding>,
}

impl Environment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn declare(&mut self, name: &str, value: Value, constant: bool) -> Result<(), Error> {
        if self.bindings.iter().any(|binding| binding.name == name) {
            return Err(Error::Redeclared(name.to_owned()));
        }
        self.bindings.push(Binding {
            name: name.to_owned(),
            value,
            constant,
        });
        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<&Value, Error> {
        self.bindings
            .iter()
            .find(|binding| binding.name == name)
            .map(|binding| &binding.value)
            .ok_or_else(|| Error::Undefined(name.to_owned()))
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), Error> {
        let binding = self
            .bindings
            .iter_mut()
            .find(|binding| binding.name == name)
            .ok_or_else(|| Error::Undefined(name.to_owned()))?;
        if binding.constant {
            return Err(Error::Constant(name.to_owned()));
        }
        binding.value = value;
        Ok(())
    }
}
