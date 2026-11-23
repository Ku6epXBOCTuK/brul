use std::cell::RefCell;
use std::rc::Rc;

use brul_event::EventSet;
use brul_gui::{UiEvent, WidgetId, WidgetTree};
use brul_script::{Component, Dyn, Error as ScriptError};

use super::{Build, ComponentHandle, StateId, ViewError};

pub type Param = fn(&mut Component) -> Result<Dyn, ScriptError>;
pub type Apply = fn(&mut Build, StateId, &[Dyn]) -> Result<(), ViewError>;
pub type EventHook = fn(&mut Build, StateId, &UiEvent) -> bool;
pub type CleanupHook = fn(&mut Build, StateId);

#[derive(Clone)]
pub struct Hooks {
    pub widget: WidgetId,
    pub state: StateId,
    pub component: ComponentHandle,
    pub params: Vec<Param>,
    pub apply: Apply,
    pub event: EventHook,
    pub cleanup: CleanupHook,
    pub events: EventSet,
}

pub fn apply(build: &mut Build, params: &[Param], apply: Apply) -> Result<(), ViewError> {
    let mut values = Vec::with_capacity(params.len());
    for param in params {
        values.push(param(&mut build.component().borrow_mut())?);
    }
    apply(build, build.state_id(), &values)
}

pub struct Element<'a> {
    id: WidgetId,
    tree: &'a Rc<RefCell<WidgetTree>>,
}

impl Element<'_> {
    pub(crate) fn new(id: WidgetId, tree: &Rc<RefCell<WidgetTree>>) -> Element<'_> {
        Element { id, tree }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    pub fn alive(&self) -> bool {
        self.tree.borrow().contains(self.id)
    }

    pub fn set_text(&self, text: impl Into<String>) -> bool {
        self.tree.borrow_mut().set_text(self.id, text.into())
    }

    pub fn set_input(&self, text: impl Into<String>) -> bool {
        self.tree.borrow_mut().set_input_text(self.id, text.into())
    }

    pub fn set_checked(&self, checked: bool) -> bool {
        self.tree.borrow_mut().set_checked(self.id, checked)
    }

    pub fn set_selected(&self, selected: bool) -> bool {
        self.tree.borrow_mut().set_selected(self.id, selected)
    }

    pub fn set_disabled(&self, disabled: bool) -> bool {
        self.tree.borrow_mut().set_disabled(self.id, disabled)
    }

    pub fn set_visible(&self, visible: bool) -> bool {
        self.tree.borrow_mut().set_visible(self.id, visible)
    }

    pub fn set_offset(&self, x: i32, y: i32) -> bool {
        self.tree.borrow_mut().set_offset(self.id, x, y)
    }

    pub fn caret_area(&self) -> Option<brul_gui::CaretArea> {
        self.tree.borrow().caret_area()
    }
}
