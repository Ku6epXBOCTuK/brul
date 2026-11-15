use std::cell::RefCell;
use std::rc::Rc;

use brul_gui::{Widget, WidgetId};
use brul_script::{Component, Error as ScriptError, Value, Variable};

use super::ComponentHandle;
use super::binding::{Binding, PropBinding};
use super::error::ViewError;
use super::event::Handler;
use super::slot::{Slots, current_slots, with_slots};

pub type StructureValue = fn(&mut Component) -> Result<Value, ScriptError>;
pub type Rebuilt = Result<(Built, Vec<RowInfo>), ViewError>;
pub type StructureRebuild = fn(Option<&str>, &ComponentHandle) -> Rebuilt;

pub type View = fn(Option<&str>, &ComponentHandle) -> Result<Built, ViewError>;

#[derive(Clone)]
pub struct Structure {
    pub parent: WidgetId,
    pub each: bool,
    pub component: ComponentHandle,
    pub scopes: Rc<[Rc<[Variable]>]>,
    pub slots: Option<Rc<Slots>>,
    pub value: StructureValue,
    pub rebuild: StructureRebuild,
}

#[derive(Debug, Clone)]
pub struct RowInfo {
    pub key: Option<Value>,
    pub item: Value,
    pub start: usize,
    pub count: usize,
}

pub struct Built {
    pub widget: Widget,
    pub bindings: Vec<Binding>,
    pub handlers: Vec<Handler>,
    pub props: Vec<PropBinding>,
}

thread_local! {
    static STRUCTURES: RefCell<Vec<(ComponentHandle, Vec<Structure>)>> = const { RefCell::new(Vec::new()) };
}

pub fn register_structure(
    component: &ComponentHandle,
    parent: WidgetId,
    each: bool,
    value: StructureValue,
    rebuild: StructureRebuild,
) {
    let structure = Structure {
        parent,
        each,
        component: Rc::clone(component),
        scopes: component.borrow().build_scopes(),
        slots: current_slots(),
        value,
        rebuild,
    };
    STRUCTURES.with(|registry| {
        let mut registry = registry.borrow_mut();
        match registry
            .iter_mut()
            .find(|(owner, _)| Rc::ptr_eq(owner, component))
        {
            Some((_, structures)) => structures.push(structure),
            None => registry.push((Rc::clone(component), vec![structure])),
        }
    });
}

pub fn take_structures() -> Vec<Structure> {
    STRUCTURES.with(|registry| {
        registry
            .borrow_mut()
            .drain(..)
            .flat_map(|(_, structures)| structures)
            .collect()
    })
}

pub fn rebuild_structure(structure: &Structure, theme: Option<&str>) -> Rebuilt {
    with_slots(structure.slots.clone(), || {
        (structure.rebuild)(theme, &structure.component)
    })
}
