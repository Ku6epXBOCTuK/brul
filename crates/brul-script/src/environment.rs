use crate::error::Error;
use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
struct Binding {
    name: String,
    value: Value,
    constant: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Environment {
    bindings: Vec<Binding>,
    frames: Vec<Vec<Binding>>,
}

impl Environment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_frame(&mut self) {
        self.frames.push(Vec::new());
    }

    pub fn pop_frame(&mut self) {
        self.frames.pop();
    }

    pub fn declare(&mut self, name: &str, value: Value, constant: bool) -> Result<(), Error> {
        let bindings = self.frames.last_mut().unwrap_or(&mut self.bindings);
        if bindings.iter().any(|binding| binding.name == name) {
            return Err(Error::Redeclared(name.to_owned()));
        }
        bindings.push(Binding {
            name: name.to_owned(),
            value,
            constant,
        });
        Ok(())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.frames
            .iter()
            .rev()
            .any(|frame| frame.iter().any(|binding| binding.name == name))
            || self.bindings.iter().any(|binding| binding.name == name)
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        for frame in self.frames.iter().rev() {
            if let Some(binding) = frame.iter().find(|binding| binding.name == name) {
                return Some(&binding.value);
            }
        }
        self.bindings
            .iter()
            .find(|binding| binding.name == name)
            .map(|binding| &binding.value)
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), Error> {
        for frame in self.frames.iter_mut().rev() {
            if let Some(binding) = frame.iter_mut().find(|binding| binding.name == name) {
                return binding.set(name, value);
            }
        }
        let binding = self
            .bindings
            .iter_mut()
            .find(|binding| binding.name == name)
            .ok_or_else(|| Error::Undefined(name.to_owned()))?;
        binding.set(name, value)
    }
}

impl Binding {
    fn set(&mut self, name: &str, value: Value) -> Result<(), Error> {
        if self.constant {
            return Err(Error::Constant(name.to_owned()));
        }
        self.value = value;
        Ok(())
    }
}
