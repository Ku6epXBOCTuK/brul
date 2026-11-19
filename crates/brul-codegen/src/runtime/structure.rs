use std::rc::Rc;

use brul_gui::{Widget, WidgetId};
use brul_script::{Component, Error as ScriptError, Scope, Value};

use super::ComponentHandle;
use super::binding::{Binding, PropBinding};
use super::build::Build;
use super::error::ViewError;
use super::event::Handler;
use super::slot::Slots;

pub type StructureValue = fn(&mut Component) -> Result<Value, ScriptError>;
pub type Rebuilt = Result<(Built, Vec<RowInfo>), ViewError>;
pub type StructureRebuild = fn(Option<&str>, &mut Build, &ComponentHandle, &[RowInfo]) -> Rebuilt;

pub type View = fn(Option<&str>, &mut Build, &ComponentHandle) -> Result<Built, ViewError>;

#[derive(Clone)]
pub struct Structure {
    pub parent: WidgetId,
    pub each: bool,
    pub component: ComponentHandle,
    pub scope: Scope,
    pub slots: Option<Rc<Slots>>,
    pub value: StructureValue,
    pub rebuild: StructureRebuild,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RowKey {
    Explicit(Value),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct RowInfo {
    pub key: RowKey,
    pub item: Value,
    pub start: usize,
    pub count: usize,
    pub reuse: Option<usize>,
}

pub struct Built {
    pub widget: Widget,
    pub bindings: Vec<Binding>,
    pub handlers: Vec<Handler>,
    pub props: Vec<PropBinding>,
}

pub fn register_structure(
    build: &mut Build,
    component: &ComponentHandle,
    parent: WidgetId,
    each: bool,
    value: StructureValue,
    rebuild: StructureRebuild,
) {
    build.register(Structure {
        parent,
        each,
        component: Rc::clone(component),
        scope: component.borrow().scope().clone(),
        slots: build.slots(),
        value,
        rebuild,
    });
}
