use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use brul_gui::{ClipboardData, UiEvent, WidgetId, WidgetTree};
use brul_script::Value;

use super::component::ComponentHandle;
use super::event_sink;
use super::hooks::{Element, Hooks};
use super::slot::Slots;
use super::state::{SharedStates, StateId};
use super::structure::Structure;
use super::{Built, Capability, ViewError};

#[derive(Default)]
pub struct Build {
    slots: Option<Rc<Slots>>,
    structures: Vec<Structure>,
    components: Vec<ComponentHandle>,
    capabilities: Vec<Hooks>,
    states: SharedStates,
    event_sink: Option<Arc<dyn super::EventSink>>,
    clipboard: Option<Rc<dyn super::Clipboard>>,
    tree: Option<Rc<RefCell<WidgetTree>>>,
    widget: Option<WidgetId>,
    state: Option<StateId>,
    component: Option<ComponentHandle>,
}

impl Build {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_slots(slots: Option<Rc<Slots>>) -> Self {
        Self {
            slots,
            ..Self::default()
        }
    }

    pub fn set_states(&mut self, states: SharedStates) {
        self.states = states;
    }

    pub fn take_states(&self) -> SharedStates {
        Rc::clone(&self.states)
    }

    pub fn set_event_sink(&mut self, sink: Option<Arc<dyn super::EventSink>>) {
        self.event_sink = sink;
    }

    pub fn event_sink(&self) -> Option<Arc<dyn super::EventSink>> {
        self.event_sink.clone()
    }

    pub fn set_clipboard_bridge(&mut self, clipboard: Option<Rc<dyn super::Clipboard>>) {
        self.clipboard = clipboard;
    }

    pub fn clipboard(&self) -> Option<ClipboardData> {
        self.clipboard
            .as_ref()
            .and_then(|clipboard| clipboard.read())
    }

    pub fn set_clipboard(&self, data: &ClipboardData) {
        if let Some(clipboard) = &self.clipboard {
            clipboard.write(data);
        }
    }

    pub fn set_tree(&mut self, tree: Rc<RefCell<WidgetTree>>) {
        self.tree = Some(tree);
    }

    pub fn set_target(&mut self, widget: WidgetId, state: StateId, component: ComponentHandle) {
        self.widget = Some(widget);
        self.state = Some(state);
        self.component = Some(component);
    }

    pub fn component(&self) -> &ComponentHandle {
        self.component.as_ref().expect("capability component")
    }

    pub fn state_id(&self) -> StateId {
        self.state.expect("capability state")
    }

    pub fn element(&self) -> Element<'_> {
        Element::new(
            self.widget.expect("capability widget"),
            self.tree.as_ref().expect("capability widget tree"),
        )
    }

    pub fn focused(&self) -> Option<WidgetId> {
        self.tree
            .as_ref()
            .and_then(|tree| tree.borrow().focused_id())
    }

    pub fn focus(&self, id: Option<WidgetId>) -> bool {
        self.tree
            .as_ref()
            .is_some_and(|tree| tree.borrow_mut().set_focused(id))
    }

    pub fn focusables(&self) -> Vec<WidgetId> {
        self.tree
            .as_ref()
            .map_or_else(Vec::new, |tree| tree.borrow().focusables())
    }

    pub fn path_to(&self, id: WidgetId) -> Vec<WidgetId> {
        self.tree
            .as_ref()
            .map_or_else(Vec::new, |tree| tree.borrow().path_to_id(id))
    }

    pub fn ancestors(&self, id: WidgetId) -> Vec<WidgetId> {
        let mut path = self.path_to(id);
        path.pop();
        path
    }

    pub fn modal_scope(&self) -> Option<WidgetId> {
        self.tree
            .as_ref()
            .and_then(|tree| tree.borrow().modal_scope())
    }

    pub fn within_scope(&self, scope: WidgetId, id: WidgetId) -> bool {
        self.tree
            .as_ref()
            .is_some_and(|tree| tree.borrow().within_scope(scope, id))
    }

    pub fn get_state<T: 'static>(&self) -> Option<Rc<RefCell<T>>> {
        self.states
            .borrow()
            .get_cloned::<RefCell<T>>(self.state_id())
    }

    pub fn set_state<T: 'static>(&mut self, value: T) {
        let state = self.state_id();
        self.states.borrow_mut().get::<RefCell<T>>(
            state,
            self.component.as_ref().expect("capability component"),
            || RefCell::new(value),
        );
    }

    pub fn remove_state(&self) {
        self.states.borrow_mut().remove(self.state_id());
    }

    pub fn emit(&self, event: &str, arguments: Vec<Value>) {
        event_sink::emit(&self.event_sink, self.state_id(), event, arguments);
    }

    pub fn emit_event(&self, event: UiEvent) {
        super::event::push_ui_event(event);
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

    pub fn take_capabilities(&mut self) -> Vec<Hooks> {
        std::mem::take(&mut self.capabilities)
    }
}
