use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use brul_codegen::runtime::{
    self, Build, BuildParts, CapabilityContext, Clipboard, Hooks, Services, accepts_prop,
};
use brul_gui::{DispatchEvent, EventTarget, RawUiEvent, TimerOwner, UiEvent, WidgetId, WidgetTree};
use brul_ident::TimerId;
use brul_reactive::EffectScope;
use brul_window::{TimerCommand, WindowCommand};

#[cfg(test)]
use crate::Host;
use crate::event::{arguments, call};
use crate::{
    Built, ComponentHandle, Dispatch, Dyn, Event, EventKind, EventSink, FunctionName, Handler,
    PropBinding, SharedStates, StateId, Structure, Theme, TimerRequest, Value, ViewError,
};

mod host;
mod rebuild;
mod registry;

pub(crate) use host::CommandHost;
use rebuild::{Pending, Staged};
use registry::{Context, ElementOwner, Owner, Registry, capability_context, owners};

#[cfg(test)]
mod tests;

const EVENT_DEPTH: usize = 64;

struct DispatchPath {
    capabilities: Vec<EventTarget>,
    handlers: Vec<EventTarget>,
}

impl DispatchPath {
    fn new(tree: &WidgetTree, target: EventTarget) -> Self {
        let capabilities = match target {
            EventTarget::Window => vec![EventTarget::Window],
            EventTarget::Widget(id) => tree
                .path_to_id(id)
                .into_iter()
                .rev()
                .map(EventTarget::Widget)
                .chain(std::iter::once(EventTarget::Window))
                .collect(),
        };
        let handlers = match target {
            EventTarget::Window => vec![target],
            EventTarget::Widget(_) => vec![target, EventTarget::Window],
        };
        Self {
            capabilities,
            handlers,
        }
    }
}

pub(crate) struct AssemblyParts {
    pub(crate) structures: Vec<Structure>,
    pub(crate) capabilities: Vec<Hooks>,
    pub(crate) nested: Vec<ComponentHandle>,
    pub(crate) states: SharedStates,
    pub(crate) event_sink: Option<Arc<dyn EventSink>>,
    pub(crate) clipboard: Option<Rc<dyn Clipboard>>,
}

fn timer_command(request: TimerRequest, owner: TimerOwner) -> Option<WindowCommand> {
    if request.interval_ms.is_finite() && request.interval_ms > 0.0 {
        Some(WindowCommand::Timer(TimerCommand::Start {
            id: request.id,
            owner,
            function: request.function,
            interval: Duration::from_secs_f64(request.interval_ms / 1000.0),
            kind: request.kind,
        }))
    } else {
        tracing::error!(
            interval_ms = request.interval_ms,
            "every interval must be a positive number"
        );
        None
    }
}

type ActiveTimers = Rc<RefCell<HashMap<TimerOwner, HashSet<TimerId>>>>;

fn schedule_timers(
    components: &[OwnedComponent],
    active: &ActiveTimers,
    commands: &Rc<RefCell<Vec<WindowCommand>>>,
) {
    for owned in components {
        let (timers, cancels) = {
            let mut component = owned.component.borrow_mut();
            (component.take_timers(), component.take_cancels())
        };
        let starts: Vec<_> = timers
            .into_iter()
            .filter_map(|request| timer_command(request, owned.owner))
            .collect();
        {
            let mut active = active.borrow_mut();
            let ids = active.entry(owned.owner).or_default();
            for command in &starts {
                if let WindowCommand::Timer(TimerCommand::Start { id, .. }) = command {
                    ids.insert(*id);
                }
            }
            for id in &cancels {
                ids.remove(id);
            }
        }
        commands.borrow_mut().extend(starts);
        commands.borrow_mut().extend(cancels.into_iter().map(|id| {
            WindowCommand::Timer(TimerCommand::Cancel {
                id,
                owner: owned.owner,
            })
        }));
    }
}
struct OwnedComponent {
    owner: TimerOwner,
    component: ComponentHandle,
    _scope: EffectScope,
}

pub struct Ui<T: Theme> {
    pub component: ComponentHandle,
    components: Vec<OwnedComponent>,
    active: ActiveTimers,
    lifecycle: Rc<RefCell<Vec<WindowCommand>>>,
    next_owner: u64,
    pub tree: Rc<RefCell<WidgetTree>>,
    owners: Registry<ElementOwner>,
    window_handlers: Vec<Handler>,
    handler_index: HashMap<(EventTarget, EventKind), Handler>,
    pub(crate) commands: Rc<RefCell<Vec<WindowCommand>>>,
    requests: Rc<RefCell<Vec<WidgetId>>>,
    view: fn(Option<T>, &mut Build, &ComponentHandle) -> Result<Built, ViewError>,
    pub(crate) theme: Rc<RefCell<Option<String>>>,
    textures: runtime::Textures,
    pub(crate) states: SharedStates,
    event_sink: Option<Arc<dyn EventSink>>,
    clipboard: Option<Rc<dyn Clipboard>>,
    scope: EffectScope,
}

impl<T: Theme> Ui<T> {
    #[cfg(test)]
    pub(crate) fn assemble(
        component: ComponentHandle,
        built: Built,
        view: fn(Option<T>, &mut Build, &ComponentHandle) -> Result<Built, ViewError>,
        theme: Option<T>,
        structures: Vec<Structure>,
        host: Option<Box<dyn Host>>,
    ) -> Self {
        let commands = Rc::new(RefCell::new(Vec::new()));
        let ui = Self::assemble_with(
            component,
            built,
            view,
            theme,
            AssemblyParts {
                structures,
                capabilities: Vec::new(),
                nested: Vec::new(),
                states: Rc::new(RefCell::new(crate::States::default())),
                event_sink: None,
                clipboard: None,
            },
            Rc::clone(&commands),
            true,
        );
        ui.component.borrow_mut().set_host(CommandHost {
            commands,
            app: host,
            windows: Rc::new(HashMap::new()),
            theme: Rc::clone(&ui.theme),
        });
        ui
    }

    #[cfg(test)]
    pub(crate) fn assemble_with(
        component: ComponentHandle,
        built: Built,
        view: fn(Option<T>, &mut Build, &ComponentHandle) -> Result<Built, ViewError>,
        theme: Option<T>,
        parts: AssemblyParts,
        commands: Rc<RefCell<Vec<WindowCommand>>>,
        scroll: bool,
    ) -> Self {
        let current = Rc::new(RefCell::new(theme.map(Theme::name).map(str::to_owned)));
        Self::assemble_in(component, built, view, current, parts, commands, scroll)
    }

    pub(crate) fn assemble_in(
        component: ComponentHandle,
        built: Built,
        view: fn(Option<T>, &mut Build, &ComponentHandle) -> Result<Built, ViewError>,
        theme: Rc<RefCell<Option<String>>>,
        parts: AssemblyParts,
        commands: Rc<RefCell<Vec<WindowCommand>>>,
        scroll: bool,
    ) -> Self {
        let Built {
            widget,
            bindings,
            handlers,
            props,
        } = built;
        let AssemblyParts {
            structures,
            capabilities,
            nested,
            states,
            event_sink,
            clipboard,
        } = parts;
        let tree = Rc::new(RefCell::new(WidgetTree::new(widget)));
        tree.borrow_mut().set_root_scroll(scroll);
        let window_handlers = handlers
            .iter()
            .filter(|handler| handler.widget.is_none())
            .cloned()
            .collect();
        let mut ui = Self {
            component: Rc::clone(&component),
            components: Vec::new(),
            active: Rc::new(RefCell::new(HashMap::new())),
            lifecycle: Rc::new(RefCell::new(Vec::new())),
            next_owner: 0,
            tree,
            owners: Registry::default(),
            window_handlers,
            handler_index: HashMap::new(),
            commands,
            requests: Rc::new(RefCell::new(Vec::new())),
            view,
            theme,
            textures: runtime::Textures::default(),
            states,
            event_sink,
            clipboard,
            scope: EffectScope::new(),
        };
        for handle in std::iter::once(component).chain(nested) {
            let owned = ui.adopt(handle);
            ui.components.push(owned);
        }
        schedule_timers(&ui.components, &ui.active, &ui.commands);
        ui.bind_all(owners(bindings, props, structures, capabilities, handlers));
        ui.index_handlers();
        ui
    }

    fn adopt(&mut self, component: ComponentHandle) -> OwnedComponent {
        let owner = TimerOwner::new(self.next_owner);
        self.next_owner += 1;
        let scope = self.scope.child();
        let active = Rc::clone(&self.active);
        let lifecycle = Rc::clone(&self.lifecycle);
        scope.on_cleanup(move || {
            if let Some(ids) = active.borrow_mut().remove(&owner)
                && !ids.is_empty()
            {
                lifecycle.borrow_mut().extend(
                    ids.into_iter()
                        .map(|id| WindowCommand::Timer(TimerCommand::Cancel { id, owner })),
                );
            }
        });
        OwnedComponent {
            owner,
            component,
            _scope: scope,
        }
    }

    fn context(&self) -> Context {
        Context {
            tree: Rc::clone(&self.tree),
            textures: Rc::clone(&self.textures),
            states: Rc::clone(&self.states),
            sink: self.event_sink.clone(),
            clipboard: self.clipboard.clone(),
            requests: Rc::clone(&self.requests),
        }
    }

    fn bind_all(&mut self, owners: Vec<ElementOwner>) {
        let ctx = self.context();
        self.owners.bind(&self.scope, &ctx, owners);
    }

    fn index_handlers(&mut self) {
        self.handler_index = self
            .owners
            .handlers()
            .fold(HashMap::new(), |mut index, handler| {
                let target = handler
                    .widget
                    .map_or(EventTarget::Window, EventTarget::Widget);
                index
                    .entry((target, handler.event))
                    .or_insert_with(|| handler.clone());
                index
            });
    }

    fn rebind(&mut self, staged: Staged) {
        let Staged {
            owners: mut staged_owners,
            components,
            rebuilt,
        } = staged;
        for parent in &rebuilt {
            if let Some(owner) = self.owners.structure_mut(*parent) {
                owner.components.clear();
            }
        }
        let structure_parents: HashSet<WidgetId> = staged_owners
            .iter()
            .filter_map(|owner| match owner {
                ElementOwner::Structure(owner) => Some(owner.structure.parent),
                _ => None,
            })
            .collect();
        let mut carried = HashMap::new();
        for parent in &structure_parents {
            if let Some(owner) = self.owners.structure_mut(*parent) {
                carried.insert(*parent, std::mem::take(&mut owner.rows));
            }
        }
        // NOTE: bind runs effects immediately, so a live tree borrow here panics on borrow_mut
        {
            let tree = self.tree.borrow();
            self.owners.retain_live(&tree, &staged_owners);
        }
        for owner in &mut staged_owners {
            if let ElementOwner::Structure(owner) = owner
                && let Some(rows) = carried.remove(&owner.structure.parent)
            {
                owner.rows = rows;
            }
        }
        self.bind_all(staged_owners);
        self.index_handlers();
        for (parent, handle) in components {
            let owned = self.adopt(handle);
            if let Some(entry) = self.owners.structure_mut(parent) {
                entry.components.push(Rc::clone(&owned.component));
            }
            self.components.push(owned);
        }
        self.prune_components();
    }
}
#[path = "parts/methods.rs"]
mod methods;
use methods::apply_prop;
