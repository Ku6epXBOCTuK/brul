use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariableId(u32);

impl VariableId {
    pub const fn from_index(index: u32) -> Self {
        Self(index)
    }

    pub const fn index(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Names<'a> {
    order: Vec<&'a str>,
    index: HashMap<&'a str, VariableId>,
}

impl<'a> Names<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, name: &'a str) -> VariableId {
        if let Some(id) = self.index.get(name) {
            return *id;
        }
        let id = VariableId(self.order.len() as u32);
        self.order.push(name);
        self.index.insert(name, id);
        id
    }

    pub fn get(&self, name: &str) -> Option<VariableId> {
        self.index.get(name).copied()
    }

    pub fn name(&self, id: VariableId) -> Option<&'a str> {
        self.order.get(id.index() as usize).copied()
    }

    pub fn names(&self) -> &[&'a str] {
        &self.order
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}
