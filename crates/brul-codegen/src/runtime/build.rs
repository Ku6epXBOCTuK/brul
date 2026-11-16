use std::rc::Rc;

use super::slot::Slots;
use super::structure::Structure;

#[derive(Default)]
pub struct Build {
    slots: Option<Rc<Slots>>,
    structures: Vec<Structure>,
}

impl Build {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_slots(slots: Option<Rc<Slots>>) -> Self {
        Self {
            slots,
            structures: Vec::new(),
        }
    }

    pub fn slots(&self) -> Option<Rc<Slots>> {
        self.slots.clone()
    }

    pub(crate) fn replace_slots(&mut self, slots: Option<Rc<Slots>>) -> Option<Rc<Slots>> {
        std::mem::replace(&mut self.slots, slots)
    }

    pub fn register(&mut self, structure: Structure) {
        self.structures.push(structure);
    }

    pub fn take_structures(&mut self) -> Vec<Structure> {
        std::mem::take(&mut self.structures)
    }
}
