use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use brul_codegen::runtime::{self, Build, Hooks, accepts_prop};
use brul_gui::{TimerOwner, UiEvent, WidgetId, WidgetTree};
use brul_ident::TimerId;
use brul_reactive::EffectScope;
use brul_window::{TimerCommand, WindowCommand};

#[cfg(test)]
use crate::Host;
use crate::event::{arguments, call};
use crate::{
    Built, ComponentHandle, Dyn, Event, EventKind, EventSink, EventTarget, FunctionName, Handler,
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

pub(crate) struct AssemblyParts {
    pub(crate) structures: Vec<Structure>,
    pub(crate) capabilities: Vec<Hooks>,
    pub(crate) nested: Vec<ComponentHandle>,
    pub(crate) states: SharedStates,
    pub(crate) event_sink: Option<Arc<dyn EventSink>>,
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
    pub(crate) theme: Option<T>,
    textures: runtime::Textures,
    pub(crate) states: SharedStates,
    event_sink: Option<Arc<dyn EventSink>>,
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
            },
            Rc::clone(&commands),
            true,
        );
        ui.component.borrow_mut().set_host(CommandHost {
            commands,
            app: host,
            windows: Rc::new(HashMap::new()),
            theme: Rc::new(RefCell::new(theme.map(Theme::name).map(str::to_owned))),
        });
        ui
    }

    pub(crate) fn assemble_with(
        component: ComponentHandle,
        built: Built,
        view: fn(Option<T>, &mut Build, &ComponentHandle) -> Result<Built, ViewError>,
        theme: Option<T>,
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

    fn prune_components(&mut self) {
        let mut live: HashSet<*const RefCell<crate::Component>> = HashSet::new();
        live.insert(Rc::as_ptr(&self.component));
        for owner in self.owners.entries() {
            live.extend(owner.components().iter().map(Rc::as_ptr));
            if let ElementOwner::Structure(owner) = owner {
                live.insert(Rc::as_ptr(&owner.structure.component));
            }
        }
        for handler in &self.window_handlers {
            live.insert(Rc::as_ptr(&handler.component));
        }
        self.components
            .retain(|owned| live.contains(&Rc::as_ptr(&owned.component)));
    }

    fn rebuild(&mut self) -> Result<(), ViewError> {
        let mut build = Build::new();
        build.set_states(Rc::clone(&self.states));
        build.set_event_sink(self.event_sink.clone());
        let built = (self.view)(self.theme, &mut build, &self.component);
        let structures = build.take_structures();
        let nested = build.take_components();
        let capabilities = build.take_capabilities();
        let built = built?;
        self.components.clear();
        for handle in std::iter::once(Rc::clone(&self.component)).chain(nested) {
            let owned = self.adopt(handle);
            self.components.push(owned);
        }
        self.tree.borrow_mut().replace_root(built.widget);
        let Built {
            bindings,
            handlers: widget_handlers,
            props,
            ..
        } = built;
        let mut handlers = widget_handlers;
        handlers.extend(self.window_handlers.iter().cloned());
        self.owners.clear();
        self.requests.borrow_mut().clear();
        self.scope = EffectScope::new();
        self.bind_all(owners(bindings, props, structures, capabilities, handlers));
        self.index_handlers();
        Ok(())
    }

    pub(crate) fn set_theme(&mut self, theme: T) -> Result<(), ViewError> {
        if self.theme == Some(theme) {
            return Ok(());
        }
        let previous = self.theme.replace(theme);
        if let Err(error) = self.rebuild() {
            self.theme = previous;
            return Err(error);
        }
        Ok(())
    }

    pub fn dispatch(&mut self, events: &[UiEvent]) -> Vec<WindowCommand> {
        for event in events {
            match &event.event {
                Event::Timer { owner, function } => {
                    let component = self
                        .components
                        .iter()
                        .find(|owned| owned.owner == *owner)
                        .map(|owned| Rc::clone(&owned.component));
                    if let Some(component) = component {
                        let call = component.borrow().event_call(function);
                        if let Some(call) = call {
                            let _ = call(&mut component.borrow_mut(), &[]);
                        }
                    }
                }
                Event::Close { .. } => self.close(),
                _ => self.dispatch_event(event),
            }
            self.pump_events();
        }
        self.take_commands()
    }

    fn pump_events(&mut self) {
        for _ in 0..EVENT_DEPTH {
            let Some(event) = runtime::take_event() else {
                return;
            };
            call(&event.component, event.call, event.arguments);
            brul_reactive::flush();
        }
        tracing::error!(depth = EVENT_DEPTH, "event chain is too deep");
    }

    fn dispatch_event(&mut self, event: &UiEvent) {
        let kind = event.event.kind();
        for target in event.target.chain() {
            if let Some(widget) = target.widget() {
                for owner in self.owners.capabilities(widget) {
                    if !owner.hooks.events.contains(kind) {
                        continue;
                    }
                    let mut build = capability_context(
                        &self.tree,
                        &self.states,
                        &self.event_sink,
                        widget,
                        owner.state,
                        &owner.hooks.component,
                    );
                    if (owner.hooks.event)(&mut build, owner.state, event) {
                        return;
                    }
                }
            }
            if let Some(handler) = self.handler_index.get(&(target, kind)) {
                call(
                    &handler.component,
                    handler.call,
                    arguments(&event.event, &event.target),
                );
            }
        }
    }

    fn close(&mut self) {
        match self
            .handler_index
            .get(&(EventTarget::Window, EventKind::Close))
        {
            Some(handler) => call(&handler.component, handler.call, Vec::new()),
            None => self.commands.borrow_mut().push(WindowCommand::Close),
        }
    }

    pub(crate) fn widget_component(&self, state: StateId) -> Option<ComponentHandle> {
        self.states.borrow().owner(state)
    }

    pub(crate) fn call_widget_event(
        &self,
        state: StateId,
        event: &str,
        arguments: Vec<Value>,
    ) -> bool {
        let Some(component) = self.widget_component(state) else {
            return false;
        };
        let Some(function) = component.borrow().event_call(&FunctionName::from(event)) else {
            return false;
        };
        let arguments = arguments.into_iter().map(Dyn::from).collect();
        call(&component, function, arguments);
        true
    }

    fn take_commands(&mut self) -> Vec<WindowCommand> {
        schedule_timers(&self.components, &self.active, &self.commands);
        let mut commands = std::mem::take(&mut *self.commands.borrow_mut());
        commands.extend(std::mem::take(&mut *self.lifecycle.borrow_mut()));
        commands
    }

    pub fn settle(&mut self) {
        self.pump_events();
        let requests = std::mem::take(&mut *self.requests.borrow_mut());
        if requests.is_empty() {
            return;
        }
        let mut pending = Pending::new(
            std::mem::take(&mut self.owners),
            Rc::clone(&self.states),
            self.event_sink.clone(),
        );
        let mut changed = false;
        for parent in requests {
            changed |= pending.rebuild(parent, &self.tree, self.theme.map(Theme::name));
        }
        let Pending { owners, staged, .. } = pending;
        self.owners = owners;
        if changed {
            self.rebind(staged);
        }
    }
}

pub(crate) fn apply_prop(prop: &PropBinding) {
    let previous = prop.component.borrow().scope().clone();
    prop.component.borrow_mut().set_scope(prop.scope.clone());
    let value = match prop.expression.evaluate(&mut prop.component.borrow_mut()) {
        Ok(value) => Some(value),
        Err(error) => {
            tracing::error!(%error, "binding expression failed");
            None
        }
    };
    let id = prop.target.borrow().id(prop.name);
    if let Some(value) = value
        && accepts_prop(&value, &prop.ty)
        && let Some(id) = id
        && let Err(error) = prop.target.borrow_mut().assign_property(id, value)
    {
        tracing::error!(%error, "property assignment failed");
    }
    prop.component.borrow_mut().set_scope(previous);
}
