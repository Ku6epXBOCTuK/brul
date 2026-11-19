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

    pub fn declare(&mut self, id: VariableId, value: Value, constant: bool) -> Result<(), Error> {
        let name = self.name(id);
        let bindings = &mut self.bindings;
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
        self.find(id).is_some()
    }

    pub fn get(&self, id: VariableId) -> Option<&Value> {
        self.find(id)
    }

    pub fn assign(&mut self, id: VariableId, value: Value) -> Result<(), Error> {
        let name = self.name(id);
        let binding = self
            .bindings
            .iter_mut()
            .find(|binding| binding.id == id)
            .ok_or_else(|| Error::Undefined(name.to_owned()))?;
        binding.set(name, value)
    }

    fn find(&self, id: VariableId) -> Option<&Value> {
        self.bindings
            .iter()
            .find(|binding| binding.id == id)
            .map(|binding| &binding.value)
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
