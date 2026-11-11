use crate::error::Error;
use crate::names::VariableId;
use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
struct Binding {
    id: VariableId,
    value: Value,
    constant: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Environment {
    names: &'static [&'static str],
    bindings: Vec<Binding>,
    frames: Vec<Vec<Binding>>,
}

impl Environment {
    pub fn new(names: &'static [&'static str]) -> Self {
        Self {
            names,
            ..Self::default()
        }
    }

    pub fn set_names(&mut self, names: &'static [&'static str]) {
        self.names = names;
    }

    pub fn push_frame(&mut self) {
        self.frames.push(Vec::new());
    }

    pub fn pop_frame(&mut self) {
        self.frames.pop();
    }

    pub fn declare(&mut self, id: VariableId, value: Value, constant: bool) -> Result<(), Error> {
        let name = self.name(id);
        let bindings = self.frames.last_mut().unwrap_or(&mut self.bindings);
        if bindings.iter().any(|binding| binding.id == id) {
            return Err(Error::Redeclared(name.to_owned()));
        }
        bindings.push(Binding {
            id,
            value,
            constant,
        });
        Ok(())
    }

    pub fn contains(&self, id: VariableId) -> bool {
        self.frames
            .iter()
            .rev()
            .any(|frame| frame.iter().any(|binding| binding.id == id))
            || self.bindings.iter().any(|binding| binding.id == id)
    }

    pub fn get(&self, id: VariableId) -> Option<&Value> {
        for frame in self.frames.iter().rev() {
            if let Some(binding) = frame.iter().find(|binding| binding.id == id) {
                return Some(&binding.value);
            }
        }
        self.bindings
            .iter()
            .find(|binding| binding.id == id)
            .map(|binding| &binding.value)
    }

    pub fn assign(&mut self, id: VariableId, value: Value) -> Result<(), Error> {
        let name = self.name(id);
        for frame in self.frames.iter_mut().rev() {
            if let Some(binding) = frame.iter_mut().find(|binding| binding.id == id) {
                return binding.set(name, value);
            }
        }
        let binding = self
            .bindings
            .iter_mut()
            .find(|binding| binding.id == id)
            .ok_or_else(|| Error::Undefined(name.to_owned()))?;
        binding.set(name, value)
    }

    fn name(&self, id: VariableId) -> &'static str {
        self.names
            .get(id.index() as usize)
            .copied()
            .unwrap_or("<unknown>")
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
