use std::cell::RefCell;
use std::ops::{BitOr, Range};
use std::rc::Rc;

use brul_event::{EventSet, RawEventSet};
use brul_gui::{EditBuffer, EventTarget, Position, RawUiEvent, UiEvent, WidgetId, WidgetTree};
use brul_script::{Component, Dyn, Error as ScriptError};

use super::{Build, ComponentHandle, StateId, ViewError};

pub type Param = fn(&mut Component) -> Result<Dyn, ScriptError>;
pub type Apply = fn(&mut Build, StateId, &[Dyn]) -> Result<(), ViewError>;
pub type EventHook = fn(&mut Build, StateId, &UiEvent) -> Dispatch;
pub type RawEventHook = fn(&mut Build, StateId, &RawUiEvent) -> Dispatch;
pub type CleanupHook = fn(&mut Build, StateId);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Dispatch {
    handled: bool,
    stop_capabilities: bool,
    stop_scripts: bool,
}

impl Dispatch {
    pub const CONTINUE: Self = Self {
        handled: false,
        stop_capabilities: false,
        stop_scripts: false,
    };

    pub fn handled() -> Self {
        Self {
            handled: true,
            ..Self::CONTINUE
        }
    }

    pub fn stop_capabilities() -> Self {
        Self {
            stop_capabilities: true,
            ..Self::CONTINUE
        }
    }

    pub fn stop_scripts() -> Self {
        Self {
            stop_scripts: true,
            ..Self::CONTINUE
        }
    }

    pub fn stop() -> Self {
        Self {
            handled: true,
            stop_capabilities: true,
            stop_scripts: true,
        }
    }

    pub fn and_handled(mut self) -> Self {
        self.handled = true;
        self
    }

    pub fn and_stop_capabilities(mut self) -> Self {
        self.stop_capabilities = true;
        self
    }

    pub fn and_stop_scripts(mut self) -> Self {
        self.stop_scripts = true;
        self
    }

    pub fn is_handled(self) -> bool {
        self.handled
    }

    pub fn stops_capabilities(self) -> bool {
        self.stop_capabilities
    }

    pub fn stops_scripts(self) -> bool {
        self.stop_scripts
    }
}

impl BitOr for Dispatch {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            handled: self.handled || rhs.handled,
            stop_capabilities: self.stop_capabilities || rhs.stop_capabilities,
            stop_scripts: self.stop_scripts || rhs.stop_scripts,
        }
    }
}

pub trait IntoDispatch {
    fn into_dispatch(self) -> Dispatch;
}

impl IntoDispatch for Dispatch {
    fn into_dispatch(self) -> Dispatch {
        self
    }
}

impl IntoDispatch for bool {
    fn into_dispatch(self) -> Dispatch {
        if self {
            Dispatch::stop()
        } else {
            Dispatch::CONTINUE
        }
    }
}

#[derive(Clone)]
pub struct Hooks {
    pub target: EventTarget,
    pub state: StateId,
    pub component: ComponentHandle,
    pub params: Vec<Param>,
    pub apply: Apply,
    pub event: EventHook,
    pub raw_event: RawEventHook,
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

    pub fn is_input(&self) -> bool {
        self.edit_buffer().is_some()
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

    pub fn wrap(&self) -> bool {
        self.tree.borrow().wrap(self.id)
    }

    pub fn line_start(&self, byte: usize) -> Option<usize> {
        self.tree.borrow().line_start(self.id, byte)
    }

    pub fn line_end(&self, byte: usize) -> Option<usize> {
        self.tree.borrow().line_end(self.id, byte)
    }

    pub fn caret_above(&self, byte: usize) -> Option<usize> {
        self.tree.borrow().caret_above(self.id, byte)
    }

    pub fn caret_below(&self, byte: usize) -> Option<usize> {
        self.tree.borrow().caret_below(self.id, byte)
    }

    pub fn scrollable(&self) -> bool {
        self.tree.borrow().scrollable(self.id)
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
        let clear = self.tree.borrow().focused_id() == Some(self.id);
        if focused {
            self.tree.borrow_mut().set_focused(Some(self.id))
        } else if clear {
            self.tree.borrow_mut().set_focused(None)
        } else {
            false
        }
    }

    pub fn scroll_by(&self, dy: i32) -> bool {
        self.tree.borrow_mut().scroll_by(self.id, dy)
    }

    pub fn caret_area(&self) -> Option<brul_gui::CaretArea> {
        self.tree.borrow().caret_area()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_flags_are_independent() {
        let handled = Dispatch::handled();
        assert!(handled.is_handled());
        assert!(!handled.stops_capabilities());
        assert!(!handled.stops_scripts());

        let capabilities = Dispatch::stop_capabilities();
        assert!(!capabilities.is_handled());
        assert!(capabilities.stops_capabilities());
        assert!(!capabilities.stops_scripts());

        let scripts = Dispatch::stop_scripts();
        assert!(!scripts.is_handled());
        assert!(!scripts.stops_capabilities());
        assert!(scripts.stops_scripts());
    }

    #[test]
    fn dispatch_flags_combine() {
        let combined = Dispatch::handled().and_stop_capabilities();
        assert!(combined.is_handled());
        assert!(combined.stops_capabilities());
        assert!(!combined.stops_scripts());

        let merged = combined | Dispatch::stop_scripts();
        assert_eq!(merged, Dispatch::stop());
    }

    #[test]
    fn bool_into_dispatch_preserves_legacy_stop() {
        assert_eq!(true.into_dispatch(), Dispatch::stop());
        assert_eq!(false.into_dispatch(), Dispatch::CONTINUE);
        assert_eq!(Dispatch::handled().into_dispatch(), Dispatch::handled());
    }
}
