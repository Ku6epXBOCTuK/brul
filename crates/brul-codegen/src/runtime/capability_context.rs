use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use brul_gui::{ClipboardData, EventTarget, RawUiEvent, UiEvent, WidgetId, WidgetTree};
use brul_script::Value;

use super::component::ComponentHandle;
use super::event_sink;
use super::hooks::Element;
use super::state::{SharedStates, StateId};
use super::{Clipboard, EventSink, Services};

pub struct CapabilityContext {
    tree: Rc<RefCell<WidgetTree>>,
    states: SharedStates,
    event_sink: Option<Arc<dyn EventSink>>,
    clipboard: Option<Rc<dyn Clipboard>>,
    target: EventTarget,
    state: StateId,
    component: ComponentHandle,
    handled: Option<Rc<Cell<bool>>>,
}

impl CapabilityContext {
    pub fn new(
        services: Services,
        tree: Rc<RefCell<WidgetTree>>,
        target: EventTarget,
        state: StateId,
        component: ComponentHandle,
        handled: Option<Rc<Cell<bool>>>,
    ) -> Self {
        let Services {
            states,
            event_sink,
            clipboard,
        } = services;
        Self {
            tree,
            states,
            event_sink,
            clipboard,
            target,
            state,
            component,
            handled,
        }
    }

    pub fn component(&self) -> &ComponentHandle {
        &self.component
    }

    pub fn state_id(&self) -> StateId {
        self.state
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

    pub fn handled(&self) -> bool {
        self.handled.as_ref().is_some_and(|flag| flag.get())
    }

    pub fn element(&self) -> Element<'_> {
        self.try_element().expect("capability widget")
    }

    pub fn try_element(&self) -> Option<Element<'_>> {
        let id = self.target.widget()?;
        Some(Element::new(id, &self.tree))
    }

    pub fn element_for(&self, id: WidgetId) -> Option<Element<'_>> {
        Some(Element::new(id, &self.tree))
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<WidgetId> {
        self.tree.borrow().hit_active(x, y).ok().flatten()
    }

    pub fn focused(&self) -> Option<WidgetId> {
        self.tree.borrow().focused_id()
    }

    pub fn focus(&self, id: Option<WidgetId>) -> bool {
        self.tree.borrow_mut().set_focused(id)
    }

    pub fn enter_modal_scope(&self) {
        self.tree.borrow_mut().enter_modal_scope();
    }

    pub fn focusables(&self) -> Vec<WidgetId> {
        self.tree.borrow().focusables()
    }

    pub fn path_to(&self, id: WidgetId) -> Vec<WidgetId> {
        self.tree.borrow().path_to_id(id)
    }

    pub fn ancestors(&self, id: WidgetId) -> Vec<WidgetId> {
        let mut path = self.path_to(id);
        path.pop();
        path
    }

    pub fn modal_scope(&self) -> Option<WidgetId> {
        self.tree.borrow().modal_scope()
    }

    pub fn within_scope(&self, scope: WidgetId, id: WidgetId) -> bool {
        self.tree.borrow().within_scope(scope, id)
    }

    pub fn nearest_scrollable(&self, id: WidgetId) -> Option<WidgetId> {
        let path = self.path_to(id);
        let tree = self.tree.borrow();
        let scope = tree.modal_scope();
        path.into_iter().rev().find(|candidate| {
            tree.scrollable(*candidate)
                && scope.is_none_or(|scope| tree.within_scope(scope, *candidate))
        })
    }

    pub fn get_state<T: 'static>(&self) -> Option<Rc<RefCell<T>>> {
        self.states
            .borrow()
            .get_cloned::<RefCell<T>>(self.state_id())
    }

    pub fn set_state<T: 'static>(&self, value: T) {
        self.states
            .borrow_mut()
            .get::<RefCell<T>>(self.state_id(), &self.component, || RefCell::new(value));
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

    pub fn emit_raw_event(&self, event: RawUiEvent) {
        super::event::push_raw_event(event);
    }
}
