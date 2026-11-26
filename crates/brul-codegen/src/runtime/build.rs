use std::rc::Rc;
use std::sync::Arc;

use super::component::ComponentHandle;
use super::hooks::Hooks;
use super::slot::Slots;
use super::state::{SharedStates, StateId};
use super::structure::Structure;
use super::{Built, Capability, Clipboard, EventSink, ViewError};

#[derive(Clone, Default)]
pub struct Services {
    pub states: SharedStates,
    pub event_sink: Option<Arc<dyn EventSink>>,
    pub clipboard: Option<Rc<dyn Clipboard>>,
}

pub struct BuildParts {
    pub structures: Vec<Structure>,
    pub components: Vec<ComponentHandle>,
    pub capabilities: Vec<Hooks>,
}

pub struct Build {
    slots: Option<Rc<Slots>>,
    states: SharedStates,
    event_sink: Option<Arc<dyn EventSink>>,
    structures: Vec<Structure>,
    components: Vec<ComponentHandle>,
    capabilities: Vec<Hooks>,
    state: Option<StateId>,
}

impl Build {
    pub fn new(services: Services) -> Self {
        Self::with_slots(None, services)
    }

    pub fn with_slots(slots: Option<Rc<Slots>>, services: Services) -> Self {
        let Services {
            states, event_sink, ..
        } = services;
        Self {
            slots,
            states,
            event_sink,
            structures: Vec::new(),
            components: Vec::new(),
            capabilities: Vec::new(),
            state: None,
        }
    }

    pub fn event_sink(&self) -> Option<Arc<dyn EventSink>> {
        self.event_sink.clone()
    }

    pub fn state_id(&self) -> StateId {
        self.state.expect("view capability state")
    }

    pub fn capability(&mut self, capability: Capability<'_>) -> Result<Option<Built>, ViewError> {
        match capability {
            Capability::View {
                state,
                parent,
                instance,
            } => {
                let previous = self.state.replace(state);
                let built = super::component::instance(self, &parent, instance);
                self.state = previous;
                built.map(Some)
            }
            Capability::Hooks(hooks) => {
                self.capabilities.push(hooks);
                Ok(None)
            }
        }
    }

    pub fn state<T: 'static>(
        &self,
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

    pub fn register_component(&mut self, component: ComponentHandle) {
        self.components.push(component);
    }

    pub fn finish(self) -> BuildParts {
        BuildParts {
            structures: self.structures,
            components: self.components,
            capabilities: self.capabilities,
        }
    }
}
