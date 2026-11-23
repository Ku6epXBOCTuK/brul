use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use brul_gui::WidgetId;

use super::ComponentHandle;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StateId(u64);

impl StateId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub const fn for_widget(self, widget: WidgetId) -> Self {
        let id = (widget.get() as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        Self((self.0 ^ id) & 0x001f_ffff_ffff_ffff)
    }
}

#[derive(Default)]
pub struct States {
    values: HashMap<StateId, Box<dyn Any>>,
    owners: HashMap<StateId, ComponentHandle>,
}

impl States {
    pub fn get<T: 'static>(
        &mut self,
        id: StateId,
        owner: &ComponentHandle,
        make: impl FnOnce() -> T,
    ) -> Rc<T> {
        self.owners.insert(id, Rc::clone(owner));
        let value = self
            .values
            .entry(id)
            .or_insert_with(|| Box::new(Rc::new(make())));
        value
            .downcast_ref::<Rc<T>>()
            .cloned()
            .unwrap_or_else(|| panic!("state type mismatch for {id:?}"))
    }

    pub fn get_cloned<T: 'static>(&self, id: StateId) -> Option<Rc<T>> {
        self.values
            .get(&id)
            .and_then(|value| value.downcast_ref::<Rc<T>>())
            .cloned()
    }

    pub fn owner(&self, id: StateId) -> Option<ComponentHandle> {
        self.owners.get(&id).cloned()
    }

    pub fn remove(&mut self, id: StateId) {
        self.values.remove(&id);
        self.owners.remove(&id);
    }
}

pub type SharedStates = Rc<RefCell<States>>;
