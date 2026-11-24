use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use brul_event::{EventSet, RawEventSet};
use brul_gui::{DispatchEvent, EditBuffer, EventTarget, Position, WidgetId, WidgetTree};
use brul_script::{Component, Dyn, Error as ScriptError};

use super::{Build, ComponentHandle, StateId, ViewError};

pub type Param = fn(&mut Component) -> Result<Dyn, ScriptError>;
pub type Apply = fn(&mut Build, StateId, &[Dyn]) -> Result<(), ViewError>;
pub type EventHook = fn(&mut Build, StateId, &DispatchEvent) -> Dispatch;
pub type CleanupHook = fn(&mut Build, StateId);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dispatch {
    Continue,
    Stop,
}

#[derive(Clone)]
pub struct Hooks {
    pub target: EventTarget,
    pub state: StateId,
    pub component: ComponentHandle,
    pub params: Vec<Param>,
    pub apply: Apply,
    pub event: EventHook,
    pub cleanup: CleanupHook,
    pub events: EventSet,
    pub raw_events: RawEventSet,
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

    pub fn edit_buffer(&self) -> Option<EditBuffer> {
        self.tree.borrow().edit_buffer(self.id)
    }

    pub fn with_edit<R>(&self, edit: impl FnOnce(&mut EditBuffer) -> R) -> Option<R> {
        let mut tree = self.tree.borrow_mut();
        let mut buffer = tree.edit_buffer(self.id)?;
        let result = edit(&mut buffer);
        tree.commit_edit(self.id, &buffer).then_some(result)
    }

    pub fn caret(&self) -> Option<Position> {
        self.tree.borrow().caret_position(self.id)
    }

    pub fn caret_at_pixel(&self, x: i32, y: i32) -> Option<Position> {
        self.tree.borrow().caret_at_pixel(self.id, x, y)
    }

    pub fn selection(&self) -> Range<Position> {
        self.tree
            .borrow()
            .selection_range(self.id)
            .unwrap_or(Position::new(0, 0)..Position::new(0, 0))
    }

    pub fn hovered(&self) -> bool {
        self.tree.borrow().hovered(self.id)
    }

    pub fn pressed(&self) -> bool {
        self.tree.borrow().pressed(self.id)
    }

    pub fn focused(&self) -> bool {
        self.tree.borrow().focused_id() == Some(self.id)
    }

    pub fn set_hovered(&self, hovered: bool) -> bool {
        self.tree.borrow_mut().set_hovered(self.id, hovered)
    }

    pub fn set_pressed(&self, pressed: bool) -> bool {
        self.tree.borrow_mut().set_pressed(self.id, pressed)
    }

    pub fn set_focused(&self, focused: bool) -> bool {
        self.tree
            .borrow_mut()
            .set_focused(focused.then_some(self.id))
    }

    pub fn scroll_by(&self, dy: i32) -> bool {
        self.tree.borrow_mut().scroll_by(self.id, dy)
    }

    pub fn caret_area(&self) -> Option<brul_gui::CaretArea> {
        self.tree.borrow().caret_area()
    }
}
