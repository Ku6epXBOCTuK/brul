use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use brul_gui::{WidgetId, WidgetTree};

use super::registry::owners;
use super::{ElementOwner, Registry};
use crate::each::reconcile_each;
use crate::{
    Build, BuildParts, Built, Clipboard, ComponentHandle, EventSink, Hooks, RowInfo, Services,
    SharedStates, Structure,
};

#[derive(Default)]
pub(super) struct Staged {
    pub(super) owners: Vec<ElementOwner>,
    pub(super) components: Vec<(WidgetId, ComponentHandle)>,
    pub(super) rebuilt: Vec<WidgetId>,
}

struct Rebuilt {
    built: Built,
    rows: Vec<RowInfo>,
    structures: Vec<Structure>,
    capabilities: Vec<Hooks>,
    components: Vec<ComponentHandle>,
}

pub(super) struct Pending {
    pub(super) owners: Registry<ElementOwner>,
    by_parent: HashMap<WidgetId, Structure>,
    states: SharedStates,
    event_sink: Option<Arc<dyn EventSink>>,
    clipboard: Option<Rc<dyn Clipboard>>,
    pub(super) staged: Staged,
}

impl Pending {
    pub(super) fn new(
        owners: Registry<ElementOwner>,
        states: SharedStates,
        event_sink: Option<Arc<dyn EventSink>>,
        clipboard: Option<Rc<dyn Clipboard>>,
    ) -> Self {
        let by_parent = owners
            .structures()
            .map(|owner| (owner.structure.parent, owner.structure.clone()))
            .collect();
        Self {
            owners,
            by_parent,
            states,
            event_sink,
            clipboard,
            staged: Staged::default(),
        }
    }

    pub(super) fn rebuild(
        &mut self,
        parent: WidgetId,
        tree: &Rc<RefCell<WidgetTree>>,
        theme: Option<&str>,
    ) -> bool {
        let Some(structure) = self.by_parent.get(&parent).cloned() else {
            return false;
        };
        if !tree.borrow().contains(parent) {
            return false;
        }
        let old_rows = self
            .owners
            .structure_mut(parent)
            .map(|owner| std::mem::take(&mut owner.rows))
            .unwrap_or_default();
        let Some(Rebuilt {
            built,
            rows,
            structures,
            capabilities,
            components,
        }) = rebuild_structure(
            &structure,
            theme,
            &old_rows,
            Rc::clone(&self.states),
            self.event_sink.clone(),
            self.clipboard.clone(),
        )
        else {
            if structure.each
                && let Some(owner) = self.owners.structure_mut(parent)
            {
                owner.rows = old_rows;
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
            reconcile_each(&mut tree, parent, &old_rows, widget, &rows)
        } else {
            widget.into_children()
        };
        tree.replace_all_children(parent, children);
        if structure.each
            && let Some(owner) = self.owners.structure_mut(parent)
        {
            owner.rows = rows;
        }
        self.staged
            .owners
            .extend(owners(bindings, props, structures, capabilities, handlers));
        self.staged
            .components
            .extend(components.into_iter().map(|component| (parent, component)));
        self.staged.rebuilt.push(parent);
        true
    }
}

fn rebuild_structure(
    structure: &Structure,
    theme: Option<&str>,
    old_rows: &[RowInfo],
    states: SharedStates,
    event_sink: Option<Arc<dyn EventSink>>,
    clipboard: Option<Rc<dyn Clipboard>>,
) -> Option<Rebuilt> {
    let component = Rc::clone(&structure.component);
    let previous = component.borrow().scope().clone();
    component.borrow_mut().set_scope(structure.scope.clone());
    let mut build = Build::with_slots(
        structure.slots.clone(),
        Services {
            states,
            event_sink,
            clipboard,
        },
    );
    let rebuilt = (structure.rebuild)(theme, &mut build, &component, old_rows);
    let BuildParts {
        structures,
        components,
        capabilities,
    } = build.finish();
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
        capabilities,
        components,
    })
}
