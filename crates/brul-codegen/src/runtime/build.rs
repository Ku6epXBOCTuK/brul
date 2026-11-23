use std::cell::RefCell;
use std::rc::Rc;

use super::component::ComponentHandle;
use super::slot::Slots;
use super::state::{SharedStates, StateId, States};
use super::structure::Structure;

#[derive(Default)]
pub struct Build {
    slots: Option<Rc<Slots>>,
    structures: Vec<Structure>,
    components: Vec<ComponentHandle>,
    states: SharedStates,
}

impl Build {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_slots(slots: Option<Rc<Slots>>) -> Self {
        Self {
            slots,
            structures: Vec::new(),
            components: Vec::new(),
            states: Rc::new(RefCell::new(States::default())),
        }
    }

    pub fn set_states(&mut self, states: SharedStates) {
        self.states = states;
    }

    pub fn take_states(&self) -> SharedStates {
        Rc::clone(&self.states)
    }

    pub fn state<T: 'static>(
        &mut self,
        id: StateId,
        owner: &ComponentHandle,
        make: impl FnOnce() -> T,
    ) -> Rc<T> {
        self.states.borrow_mut().get(id, owner, make)
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

    pub fn register_component(&mut self, component: ComponentHandle) {
        self.components.push(component);
    }

    pub fn take_components(&mut self) -> Vec<ComponentHandle> {
        std::mem::take(&mut self.components)
    }
}
