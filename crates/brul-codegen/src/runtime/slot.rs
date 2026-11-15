use std::cell::RefCell;
use std::rc::Rc;

use brul_gui::{Direction, Widget};
use brul_script::Variable;

use super::ComponentHandle;
use super::error::ViewError;
use super::structure::{Built, View};

pub const DEFAULT_SLOT: &str = "";

#[derive(Clone, Copy)]
pub struct SlotArgument {
    pub name: &'static str,
    pub view: View,
}

#[derive(Clone)]
pub struct Slots {
    pub component: ComponentHandle,
    pub scopes: Rc<[Rc<[Variable]>]>,
    pub parent: Option<Rc<Slots>>,
    pub(crate) entries: Vec<SlotArgument>,
}

thread_local! {
    static SLOTS: RefCell<Option<Rc<Slots>>> = const { RefCell::new(None) };
}

pub(crate) fn current_slots() -> Option<Rc<Slots>> {
    SLOTS.with(|slots| slots.borrow().clone())
}

pub(crate) fn with_slots<T>(slots: Option<Rc<Slots>>, body: impl FnOnce() -> T) -> T {
    let previous = SLOTS.with(|current| current.replace(slots));
    let result = body();
    SLOTS.with(|current| *current.borrow_mut() = previous);
    result
}

pub fn slot(theme: Option<&str>, name: &str) -> Result<Built, ViewError> {
    let Some(slots) = current_slots() else {
        return Ok(empty_built());
    };
    let Some(argument) = slots.entries.iter().find(|argument| argument.name == name) else {
        return Ok(empty_built());
    };
    let component = Rc::clone(&slots.component);
    let count = slots.scopes.len();
    component.borrow_mut().push_binding_scopes(&slots.scopes);
    let rendered = with_slots(slots.parent.clone(), || (argument.view)(theme, &component));
    component.borrow_mut().pop_binding_scopes(count);
    rendered
}

fn empty_built() -> Built {
    Built {
        widget: Widget::view(Direction::Column, Vec::new()),
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    }
}
