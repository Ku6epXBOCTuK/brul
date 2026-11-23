use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use brul_gui::{WidgetId, WidgetTree};

use super::{Owned, OwnedStructure};
use crate::each::reconcile_each;
use crate::{
    Binding, Build, Built, ComponentHandle, EventSink, Handler, PropBinding, RowInfo, SharedStates,
    Structure,
};

#[derive(Default)]
pub(super) struct Staged {
    pub(super) bindings: Vec<Binding>,
    pub(super) handlers: Vec<Handler>,
    pub(super) props: Vec<PropBinding>,
    pub(super) structures: Vec<Structure>,
    pub(super) components: Vec<(WidgetId, ComponentHandle)>,
    pub(super) rebuilt: Vec<WidgetId>,
}

struct Rebuilt {
    built: Built,
    rows: Vec<RowInfo>,
    structures: Vec<Structure>,
    components: Vec<ComponentHandle>,
}

pub(super) struct Pending {
    pub(super) structures: Vec<OwnedStructure>,
    pub(super) rows: Vec<(WidgetId, Vec<RowInfo>)>,
    pub(super) by_parent: HashMap<WidgetId, Structure>,
    states: SharedStates,
    event_sink: Option<Arc<dyn EventSink>>,
    pub(super) staged: Staged,
}

impl Pending {
    pub(super) fn new(
        structures: Vec<OwnedStructure>,
        rows: Vec<(WidgetId, Vec<RowInfo>)>,
        states: SharedStates,
        event_sink: Option<Arc<dyn EventSink>>,
    ) -> Self {
        let by_parent = structures
            .iter()
            .map(|owned| (owned.structure.parent, owned.structure.clone()))
            .collect();
        Self {
            structures,
            rows,
            by_parent,
            states,
            event_sink,
            staged: Staged::default(),
        }
    }

    pub(super) fn rebuild(
        &mut self,
        parent: WidgetId,
        tree: &Rc<RefCell<WidgetTree>>,
        theme: Option<&str>,
    ) -> bool {
        let Some(structure) = self.by_parent.get(&parent) else {
            return false;
        };
        if !tree.borrow().contains(parent) {
            return false;
        }
        let old_rows = take_rows(&mut self.rows, parent, structure.each);
        let Some(Rebuilt {
            built,
            rows,
            structures,
            components,
        }) = rebuild_structure(
            structure,
            theme,
            &old_rows,
            Rc::clone(&self.states),
            self.event_sink.clone(),
        )
        else {
            if structure.each {
                self.rows.push((parent, old_rows));
            }
            return false;
        };
        let Built {
            widget,
            bindings,
            handlers,
            props,
        } = built;
        let mut tree = tree.borrow_mut();
        let children = if structure.each {
            self.rows.push((parent, rows.clone()));
            reconcile_each(&mut tree, parent, &old_rows, widget, &rows)
        } else {
            widget.into_children()
        };
        tree.replace_all_children(parent, children);
        self.staged.bindings.extend(bindings);
        self.staged.handlers.extend(handlers);
        self.staged.props.extend(props);
        self.staged.structures.extend(structures);
        self.staged
            .components
            .extend(components.into_iter().map(|component| (parent, component)));
        self.staged.rebuilt.push(parent);
        true
    }
}

fn take_rows(
    rows: &mut Vec<(WidgetId, Vec<RowInfo>)>,
    parent: WidgetId,
    each: bool,
) -> Vec<RowInfo> {
    if !each {
        return Vec::new();
    }
    rows.iter()
        .position(|(id, _)| *id == parent)
        .map(|at| rows.remove(at).1)
        .unwrap_or_default()
}

fn rebuild_structure(
    structure: &Structure,
    theme: Option<&str>,
    old_rows: &[RowInfo],
    states: SharedStates,
    event_sink: Option<Arc<dyn EventSink>>,
) -> Option<Rebuilt> {
    let component = Rc::clone(&structure.component);
    let previous = component.borrow().scope().clone();
    component.borrow_mut().set_scope(structure.scope.clone());
    let mut build = Build::with_slots(structure.slots.clone());
    build.set_states(states);
    build.set_event_sink(event_sink);
    let rebuilt = (structure.rebuild)(theme, &mut build, &component, old_rows);
    let structures = build.take_structures();
    let components = build.take_components();
    component.borrow_mut().set_scope(previous);
    let (built, rows) = match rebuilt {
        Ok(result) => result,
        Err(message) => {
            tracing::error!(%message, "view rebuild failed");
            return None;
        }
    };
    Some(Rebuilt {
        built,
        rows,
        structures,
        components,
    })
}

pub(super) trait WidgetRef {
    fn widget(&self) -> Option<WidgetId>;
}

impl WidgetRef for Owned {
    fn widget(&self) -> Option<WidgetId> {
        Some(self.widget)
    }
}

impl WidgetRef for OwnedStructure {
    fn widget(&self) -> Option<WidgetId> {
        Some(self.structure.parent)
    }
}

impl WidgetRef for (WidgetId, Vec<RowInfo>) {
    fn widget(&self) -> Option<WidgetId> {
        Some(self.0)
    }
}

impl WidgetRef for Handler {
    fn widget(&self) -> Option<WidgetId> {
        self.widget
    }
}

pub(super) fn retain_live<T: WidgetRef>(
    items: &mut Vec<T>,
    tree: &WidgetTree,
    replaced: &HashSet<WidgetId>,
) {
    items.retain(|item| {
        item.widget()
            .is_none_or(|id| tree.contains(id) && !replaced.contains(&id))
    });
}
