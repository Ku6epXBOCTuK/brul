use super::StateId;
use super::component::{ComponentHandle, Instance};
use super::hooks::Hooks;

pub enum Capability<'a> {
    View {
        state: StateId,
        parent: ComponentHandle,
        instance: Instance<'a>,
    },
    Hooks(Hooks),
}

impl<'a> Capability<'a> {
    pub fn view(state: StateId, parent: ComponentHandle, instance: Instance<'a>) -> Self {
        Self::View {
            state,
            parent,
            instance,
        }
    }

    pub fn hooks(hooks: Hooks) -> Self {
        Self::Hooks(hooks)
    }
}
