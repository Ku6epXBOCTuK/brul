use std::rc::Rc;

use brul_gui::{Direction, Widget};
use brul_script::Scope;

use super::ComponentHandle;
use super::build::Build;
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
    pub scope: Scope,
    pub parent: Option<Rc<Slots>>,
    pub(crate) entries: Vec<SlotArgument>,
}

pub fn slot(theme: Option<&str>, build: &mut Build, name: &str) -> Result<Built, ViewError> {
    let Some(slots) = build.slots() else {
        return Ok(empty_built());
    };
    let Some(argument) = slots.entries.iter().find(|argument| argument.name == name) else {
        return Ok(empty_built());
    };
    let component = Rc::clone(&slots.component);
    let previous_scope = component.borrow().scope().clone();
    component.borrow_mut().set_scope(slots.scope.clone());
    let previous = build.replace_slots(slots.parent.clone());
    let rendered = (argument.view)(theme, build, &component);
    build.replace_slots(previous);
    component.borrow_mut().set_scope(previous_scope);
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
