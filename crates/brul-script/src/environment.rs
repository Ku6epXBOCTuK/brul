use std::rc::Rc;

use crate::component::Variable;
use crate::error::Error;
use crate::names::VariableId;
use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
struct Binding {
    id: VariableId,
    value: Value,
    constant: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum Frame {
    Bindings(Vec<Binding>),
    Scope {
        scopes: Rc<[Vec<Variable>]>,
        index: usize,
    },
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Environment {
    names: &'static [&'static str],
    bindings: Vec<Binding>,
    frames: Vec<Frame>,
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
        self.frames.push(Frame::Bindings(Vec::new()));
    }

    pub fn push_scope(&mut self, scopes: &Rc<[Vec<Variable>]>, index: usize) {
        self.frames.push(Frame::Scope {
            scopes: Rc::clone(scopes),
            index,
        });
    }

    pub fn pop_frame(&mut self) {
        self.frames.pop();
    }

    pub fn declare(&mut self, id: VariableId, value: Value, constant: bool) -> Result<(), Error> {
        let name = self.name(id);
        let bindings = match self.frames.last_mut() {
            Some(Frame::Bindings(bindings)) => bindings,
            _ => &mut self.bindings,
        };
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
        for frame in self.frames.iter_mut().rev() {
            let bindings = match frame {
                Frame::Bindings(bindings) => bindings,
                Frame::Scope { scopes, index }
                    if scopes[*index].iter().any(|variable| variable.name == id) =>
                {
                    return Err(Error::Constant(name.to_owned()));
                }
                Frame::Scope { .. } => continue,
            };
            if let Some(binding) = bindings.iter_mut().find(|binding| binding.id == id) {
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

    fn find(&self, id: VariableId) -> Option<&Value> {
        for frame in self.frames.iter().rev() {
            match frame {
                Frame::Bindings(bindings) => {
                    if let Some(binding) = bindings.iter().find(|binding| binding.id == id) {
                        return Some(&binding.value);
                    }
                }
                Frame::Scope { scopes, index } => {
                    if let Some(variable) =
                        scopes[*index].iter().find(|variable| variable.name == id)
                    {
                        return Some(&variable.value);
                    }
                }
            }
        }
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
