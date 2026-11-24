use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use brul_codegen::runtime::{self, Build, apply_capability};
use brul_gui::{EventTarget, WidgetId, WidgetTree};
use brul_reactive::{Effect, EffectScope};

use super::apply_prop;
use crate::{
    Binding, ComponentHandle, EventSink, Handler, Hooks, PropBinding, RowInfo, SharedStates,
    StateId, Structure, Value,
};

pub(super) struct Context {
    pub(super) tree: Rc<RefCell<WidgetTree>>,
    pub(super) textures: runtime::Textures,
    pub(super) states: SharedStates,
    pub(super) sink: Option<Arc<dyn EventSink>>,
    pub(super) requests: Rc<RefCell<Vec<WidgetId>>>,
}

pub(super) struct PropOwner {
    pub(super) binding: PropBinding,
    pub(super) components: Vec<ComponentHandle>,
}

pub(super) struct StructureOwner {
    pub(super) structure: Structure,
    pub(super) components: Vec<ComponentHandle>,
    pub(super) rows: Vec<RowInfo>,
}

pub(super) struct CapabilityOwner {
    pub(super) hooks: Hooks,
    pub(super) state: StateId,
}

pub(super) enum ElementOwner {
    Binding(Binding),
    Prop(PropOwner),
    Structure(StructureOwner),
    Capability(CapabilityOwner),
    Handler(Handler),
}

pub(super) trait Owner: Sized {
    fn widget(&self) -> Option<WidgetId>;
    fn components(&self) -> &[ComponentHandle];
    fn replaced(&self, staged: &[Self]) -> bool {
        let widget = self.widget();
        staged.iter().any(|owner| owner.widget() == widget)
    }
    fn attach(&self, scope: &EffectScope, ctx: &Context);
}

impl Owner for ElementOwner {
    fn widget(&self) -> Option<WidgetId> {
        match self {
            Self::Binding(binding) => Some(binding.widget),
            Self::Prop(owner) => Some(owner.binding.widget),
            Self::Structure(owner) => Some(owner.structure.parent),
            Self::Capability(owner) => owner.hooks.target.widget(),
            Self::Handler(handler) => handler.widget,
        }
    }

    fn components(&self) -> &[ComponentHandle] {
        match self {
            Self::Binding(binding) => std::slice::from_ref(&binding.component),
            Self::Prop(owner) => &owner.components,
            Self::Structure(owner) => &owner.components,
            Self::Capability(owner) => std::slice::from_ref(&owner.hooks.component),
            Self::Handler(handler) => std::slice::from_ref(&handler.component),
        }
    }

    fn replaced(&self, staged: &[Self]) -> bool {
        if matches!(self, Self::Handler(_)) {
            return false;
        }
        let kind = std::mem::discriminant(self);
        let widget = self.widget();
        staged
            .iter()
            .any(|owner| std::mem::discriminant(owner) == kind && owner.widget() == widget)
    }

    fn attach(&self, scope: &EffectScope, ctx: &Context) {
        match self {
            Self::Binding(binding) => attach_binding(binding, scope, ctx),
            Self::Prop(owner) => attach_prop(owner, scope),
            Self::Structure(owner) => attach_structure(owner, scope, ctx),
            Self::Capability(owner) => attach_capability(owner, scope, ctx),
            Self::Handler(_) => {}
        }
    }
}

struct Entry<O> {
    owner: O,
    _scope: EffectScope,
}

pub(super) struct Registry<O: Owner> {
    entries: Vec<Entry<O>>,
    by_widget: HashMap<Option<WidgetId>, Vec<usize>>,
}

impl<O: Owner> Default for Registry<O> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            by_widget: HashMap::new(),
        }
    }
}

impl<O: Owner> Registry<O> {
    pub(super) fn bind(
        &mut self,
        parent: &EffectScope,
        ctx: &Context,
        owners: impl IntoIterator<Item = O>,
    ) {
        for owner in owners {
            let scope = parent.child();
            owner.attach(&scope, ctx);
            self.by_widget
                .entry(owner.widget())
                .or_default()
                .push(self.entries.len());
            self.entries.push(Entry {
                owner,
                _scope: scope,
            });
        }
    }

    pub(super) fn retain_live(&mut self, tree: &WidgetTree, staged: &[O]) {
        self.entries.retain(|entry| {
            entry
                .owner
                .widget()
                .is_none_or(|id| tree.contains(id) && !entry.owner.replaced(staged))
        });
        self.reindex();
    }

    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.by_widget.clear();
    }

    pub(super) fn entries(&self) -> impl Iterator<Item = &O> {
        self.entries.iter().map(|entry| &entry.owner)
    }

    fn reindex(&mut self) {
        self.by_widget.clear();
        for (at, entry) in self.entries.iter().enumerate() {
            self.by_widget
                .entry(entry.owner.widget())
                .or_default()
                .push(at);
        }
    }
}

impl Registry<ElementOwner> {
    pub(super) fn capabilities(
        &self,
        target: &EventTarget,
    ) -> impl Iterator<Item = &CapabilityOwner> {
        self.by_widget
            .get(&target.widget())
            .into_iter()
            .flatten()
            .filter_map(|at| match &self.entries[*at].owner {
                ElementOwner::Capability(owner) => Some(owner),
                _ => None,
            })
    }

    pub(super) fn handlers(&self) -> impl Iterator<Item = &Handler> {
        self.entries.iter().filter_map(|entry| match &entry.owner {
            ElementOwner::Handler(handler) => Some(handler),
            _ => None,
        })
    }

    pub(super) fn structures(&self) -> impl Iterator<Item = &StructureOwner> {
        self.entries.iter().filter_map(|entry| match &entry.owner {
            ElementOwner::Structure(owner) => Some(owner),
            _ => None,
        })
    }

    pub(super) fn structure_mut(&mut self, parent: WidgetId) -> Option<&mut StructureOwner> {
        self.entries
            .iter_mut()
            .find_map(|entry| match &mut entry.owner {
                ElementOwner::Structure(owner) if owner.structure.parent == parent => Some(owner),
                _ => None,
            })
    }
}

pub(super) fn owners(
    bindings: Vec<Binding>,
    props: Vec<PropBinding>,
    structures: Vec<Structure>,
    capabilities: Vec<Hooks>,
    handlers: Vec<Handler>,
) -> Vec<ElementOwner> {
    let mut owners = Vec::with_capacity(
        bindings.len() + props.len() + structures.len() + capabilities.len() + handlers.len(),
    );
    owners.extend(bindings.into_iter().map(ElementOwner::Binding));
    owners.extend(props.into_iter().map(|binding| {
        ElementOwner::Prop(PropOwner {
            components: vec![Rc::clone(&binding.component), Rc::clone(&binding.target)],
            binding,
        })
    }));
    owners.extend(structures.into_iter().map(|structure| {
        ElementOwner::Structure(StructureOwner {
            structure,
            components: Vec::new(),
            rows: Vec::new(),
        })
    }));
    owners.extend(capabilities.into_iter().map(|hooks| {
        ElementOwner::Capability(CapabilityOwner {
            state: hooks.state.for_target(&hooks.target),
            hooks,
        })
    }));
    owners.extend(handlers.into_iter().map(ElementOwner::Handler));
    owners
}

pub(super) fn capability_context(
    tree: &Rc<RefCell<WidgetTree>>,
    states: &SharedStates,
    sink: &Option<Arc<dyn EventSink>>,
    target: EventTarget,
    state: StateId,
    component: &ComponentHandle,
) -> Build {
    let mut build = Build::new();
    build.set_tree(Rc::clone(tree));
    build.set_states(Rc::clone(states));
    build.set_event_sink(sink.clone());
    build.set_target(target, state, Rc::clone(component));
    build
}

fn attach_binding(binding: &Binding, scope: &EffectScope, ctx: &Context) {
    let binding = binding.clone();
    let tree = Rc::clone(&ctx.tree);
    let textures = Rc::clone(&ctx.textures);
    scope.run(|| {
        Effect::new(move || {
            let pending = runtime::evaluate(&binding, &textures);
            runtime::write(pending, binding.widget, &mut tree.borrow_mut());
        });
    });
}

fn attach_prop(owner: &PropOwner, scope: &EffectScope) {
    let prop = owner.binding.clone();
    scope.run(|| Effect::new(move || apply_prop(&prop)));
}

fn attach_structure(owner: &StructureOwner, scope: &EffectScope, ctx: &Context) {
    let structure = owner.structure.clone();
    let requests = Rc::clone(&ctx.requests);
    let initialized = Rc::new(Cell::new(false));
    let last: Rc<Cell<Option<Value>>> = Rc::new(Cell::new(None));
    scope.run(|| {
        Effect::new(move || {
            let previous = structure.component.borrow().scope().clone();
            structure
                .component
                .borrow_mut()
                .set_scope(structure.scope.clone());
            let evaluated = structure.value.call(&mut structure.component.borrow_mut());
            structure.component.borrow_mut().set_scope(previous);
            let value = match evaluated {
                Ok(value) => Some(value),
                Err(error) => {
                    tracing::error!(%error, "structure expression failed");
                    None
                }
            };
            if !initialized.replace(true) {
                last.set(value);
                return;
            }
            let changed = last.take().as_ref() != value.as_ref();
            last.set(value);
            if !changed {
                return;
            }
            let mut requests = requests.borrow_mut();
            if !requests.contains(&structure.parent) {
                requests.push(structure.parent);
            }
        });
    });
}

fn attach_capability(owner: &CapabilityOwner, scope: &EffectScope, ctx: &Context) {
    let target = owner.hooks.target;
    let state = owner.state;
    let component = Rc::clone(&owner.hooks.component);
    let cleanup = owner.hooks.cleanup;
    let params = owner.hooks.params.clone();
    let apply = owner.hooks.apply;
    let tree = Rc::clone(&ctx.tree);
    let states = Rc::clone(&ctx.states);
    let sink = ctx.sink.clone();
    let cleanup_component = Rc::clone(&component);
    let cleanup_tree = Rc::clone(&tree);
    let cleanup_states = Rc::clone(&states);
    let cleanup_sink = sink.clone();

    scope.on_cleanup(move || {
        let mut build = capability_context(
            &cleanup_tree,
            &cleanup_states,
            &cleanup_sink,
            target,
            state,
            &cleanup_component,
        );
        cleanup(&mut build, state);
    });

    let apply_component = Rc::clone(&component);
    let apply_tree = Rc::clone(&tree);
    let apply_states = Rc::clone(&states);
    let apply_sink = sink.clone();
    scope.run(|| {
        Effect::new(move || {
            let mut build = capability_context(
                &apply_tree,
                &apply_states,
                &apply_sink,
                target,
                state,
                &apply_component,
            );
            if let Err(message) = apply_capability(&mut build, &params, apply) {
                tracing::error!(%message, "capability failed");
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use brul_gui::Widget;

    struct Probe {
        widget: Option<WidgetId>,
        name: &'static str,
        log: Rc<RefCell<Vec<&'static str>>>,
    }

    impl Owner for Probe {
        fn widget(&self) -> Option<WidgetId> {
            self.widget
        }

        fn components(&self) -> &[ComponentHandle] {
            &[]
        }

        fn attach(&self, scope: &EffectScope, _ctx: &Context) {
            let log = Rc::clone(&self.log);
            let name = self.name;
            scope.on_cleanup(move || log.borrow_mut().push(name));
        }
    }

    fn context(tree: &Rc<RefCell<WidgetTree>>) -> Context {
        Context {
            tree: Rc::clone(tree),
            textures: runtime::Textures::default(),
            states: Rc::new(RefCell::new(crate::States::default())),
            sink: None,
            requests: Rc::new(RefCell::new(Vec::new())),
        }
    }

    #[test]
    fn a_new_owner_type_uses_the_shared_lifecycle() {
        let root = Widget::view(brul_gui::Direction::Column, Vec::new());
        let live = root.id();
        let dead = Widget::text("").id();
        let tree = Rc::new(RefCell::new(WidgetTree::new(root)));
        let log = Rc::new(RefCell::new(Vec::new()));
        let parent = EffectScope::new();
        let mut registry = Registry::default();
        registry.bind(
            &parent,
            &context(&tree),
            [
                Probe {
                    widget: Some(live),
                    name: "live",
                    log: Rc::clone(&log),
                },
                Probe {
                    widget: Some(dead),
                    name: "dead",
                    log: Rc::clone(&log),
                },
            ],
        );
        assert!(log.borrow().is_empty());

        registry.retain_live(&tree.borrow(), &[]);
        assert_eq!(*log.borrow(), ["dead"]);

        registry.clear();
        assert_eq!(*log.borrow(), ["dead", "live"]);
    }
}
