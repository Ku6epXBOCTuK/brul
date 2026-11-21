use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

use brul_codegen::runtime::{self, accepts_prop};
use brul_gui::{TimerOwner, WidgetEvent, WidgetId, WidgetTree};
use brul_ident::TimerId;
use brul_reactive::{Effect, EffectScope};
use brul_window::{TimerCommand, WindowCommand};

#[cfg(test)]
use crate::Host;
use crate::event::{arguments, call, handler_target};
use crate::{
    Binding, Build, Built, ComponentHandle, Handler, HandlerEvent, PropBinding, RowInfo, Structure,
    Theme, TimerRequest, Value, ViewError,
};

mod host;
mod rebuild;

pub(crate) use host::CommandHost;
use rebuild::{Pending, Staged, retain_live};

#[cfg(test)]
mod tests;

const EVENT_DEPTH: usize = 64;

pub(crate) struct AssemblyParts {
    pub(crate) structures: Vec<Structure>,
    pub(crate) nested: Vec<ComponentHandle>,
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
    handlers: Vec<Handler>,
    window_handlers: Vec<Handler>,
    handler_index: HashMap<(Option<WidgetId>, HandlerEvent), usize>,
    pub(crate) commands: Rc<RefCell<Vec<WindowCommand>>>,
    bindings: Vec<Owned>,
    props: Vec<Owned>,
    structures: Vec<OwnedStructure>,
    rows: Vec<(WidgetId, Vec<RowInfo>)>,
    requests: Rc<RefCell<Vec<WidgetId>>>,
    view: fn(Option<T>, &mut Build, &ComponentHandle) -> Result<Built, ViewError>,
    pub(crate) theme: Option<T>,
    textures: runtime::Textures,
    scope: EffectScope,
}

struct Owned {
    widget: WidgetId,
    components: Vec<ComponentHandle>,
    _scope: EffectScope,
}

struct OwnedStructure {
    structure: Structure,
    components: Vec<ComponentHandle>,
    _scope: EffectScope,
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
                nested: Vec::new(),
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
            handlers,
            window_handlers,
            handler_index: HashMap::new(),
            commands,
            bindings: Vec::new(),
            props: Vec::new(),
            structures: Vec::new(),
            rows: Vec::new(),
            requests: Rc::new(RefCell::new(Vec::new())),
            view,
            theme,
            textures: runtime::Textures::default(),
            scope: EffectScope::new(),
        };
        for handle in std::iter::once(component).chain(parts.nested) {
            let owned = ui.adopt(handle);
            ui.components.push(owned);
        }
        schedule_timers(&ui.components, &ui.active, &ui.commands);
        ui.bind(bindings);
        ui.bind_props(props);
        ui.bind_structures(parts.structures);
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

    fn index_handlers(&mut self) {
        self.handler_index = self
            .handlers
            .iter()
            .enumerate()
            .rev()
            .map(|(at, handler)| ((handler.widget, handler.event), at))
            .collect();
    }

    fn bind(&mut self, bindings: Vec<Binding>) {
        for binding in bindings {
            let widget = binding.widget;
            let components = vec![Rc::clone(&binding.component)];
            let tree = Rc::clone(&self.tree);
            let textures = Rc::clone(&self.textures);
            let scope = self.scope.child();
            scope.run(|| {
                Effect::new(move || {
                    let pending = runtime::evaluate(&binding, &textures);
                    runtime::write(pending, binding.widget, &mut tree.borrow_mut());
                });
            });
            self.bindings.push(Owned {
                widget,
                components,
                _scope: scope,
            });
        }
    }

    fn bind_props(&mut self, props: Vec<PropBinding>) {
        for prop in props {
            let widget = prop.widget;
            let components = vec![Rc::clone(&prop.component), Rc::clone(&prop.target)];
            let scope = self.scope.child();
            scope.run(|| {
                Effect::new(move || apply_prop(&prop));
            });
            self.props.push(Owned {
                widget,
                components,
                _scope: scope,
            });
        }
    }

    fn bind_structures(&mut self, structures: Vec<Structure>) {
        for structure in structures {
            let component = Rc::clone(&structure.component);
            let each_scope = structure.scope.clone();
            let structure_value = Rc::clone(&structure.value);
            let requests = Rc::clone(&self.requests);
            let initialized = Rc::new(Cell::new(false));
            let last: Rc<Cell<Option<Value>>> = Rc::new(Cell::new(None));
            let scope = self.scope.child();
            scope.run(|| {
                Effect::new(move || {
                    let previous = component.borrow().scope().clone();
                    component.borrow_mut().set_scope(each_scope.clone());
                    let evaluated = structure_value.call(&mut component.borrow_mut());
                    component.borrow_mut().set_scope(previous);
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
            self.structures.push(OwnedStructure {
                structure,
                components: Vec::new(),
                _scope: scope,
            });
        }
    }

    fn rebind(&mut self, staged: Staged) {
        let Staged {
            bindings,
            handlers,
            props,
            structures,
            components,
            rebuilt,
        } = staged;
        for owned in &mut self.structures {
            if rebuilt.contains(&owned.structure.parent) {
                owned.components.clear();
            }
        }
        let parents: HashSet<WidgetId> = structures.iter().map(|s| s.parent).collect();
        let widgets: HashSet<WidgetId> = bindings.iter().map(|b| b.widget).collect();
        let prop_widgets: HashSet<WidgetId> = props.iter().map(|p| p.widget).collect();
        let none = HashSet::new();
        // NOTE: bind runs effects immediately, so a live tree borrow here panics on borrow_mut
        {
            let tree = self.tree.borrow();
            retain_live(&mut self.structures, &tree, &parents);
            retain_live(&mut self.rows, &tree, &none);
            retain_live(&mut self.bindings, &tree, &widgets);
            retain_live(&mut self.props, &tree, &prop_widgets);
            self.handlers.extend(handlers);
            retain_live(&mut self.handlers, &tree, &none);
        }
        self.index_handlers();
        self.bind(bindings);
        self.bind_props(props);
        self.bind_structures(structures);
        for (parent, handle) in components {
            let owned = self.adopt(handle);
            if let Some(entry) = self
                .structures
                .iter_mut()
                .find(|entry| entry.structure.parent == parent)
            {
                entry.components.push(Rc::clone(&owned.component));
            }
            self.components.push(owned);
        }
        self.prune_components();
    }

    fn prune_components(&mut self) {
        let mut live: HashSet<*const RefCell<crate::Component>> = HashSet::new();
        live.insert(Rc::as_ptr(&self.component));
        for owned in &self.bindings {
            live.extend(owned.components.iter().map(Rc::as_ptr));
        }
        for owned in &self.props {
            live.extend(owned.components.iter().map(Rc::as_ptr));
        }
        for handler in self.handlers.iter().chain(&self.window_handlers) {
            live.insert(Rc::as_ptr(&handler.component));
        }
        for owned in &self.structures {
            live.insert(Rc::as_ptr(&owned.structure.component));
            live.extend(owned.components.iter().map(Rc::as_ptr));
        }
        self.components
            .retain(|owned| live.contains(&Rc::as_ptr(&owned.component)));
    }

    fn rebuild(&mut self) -> Result<(), ViewError> {
        let mut build = Build::new();
        let built = (self.view)(self.theme, &mut build, &self.component);
        let structures = build.take_structures();
        let nested = build.take_components();
        let built = built?;
        self.components.clear();
        for handle in std::iter::once(Rc::clone(&self.component)).chain(nested) {
            let owned = self.adopt(handle);
            self.components.push(owned);
        }
        self.tree.borrow_mut().replace_root(built.widget);
        let mut handlers = built.handlers;
        handlers.extend(self.window_handlers.iter().cloned());
        self.handlers = handlers;
        self.index_handlers();
        self.bindings.clear();
        self.props.clear();
        self.structures.clear();
        self.rows.clear();
        self.requests.borrow_mut().clear();
        self.scope = EffectScope::new();
        self.bind(built.bindings);
        self.bind_props(built.props);
        self.bind_structures(structures);
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

    pub fn dispatch(&mut self, events: &[WidgetEvent]) -> Vec<WindowCommand> {
        for event in events {
            match event {
                WidgetEvent::Timer(owner, function) => {
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
                WidgetEvent::Close => self.close(),
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

    fn dispatch_event(&mut self, event: &WidgetEvent) {
        let Some((widget, target)) = handler_target(event) else {
            return;
        };
        let Some(&at) = self.handler_index.get(&(widget, target)) else {
            return;
        };
        let handler = &self.handlers[at];
        call(&handler.component, handler.call, arguments(event));
    }

    fn close(&mut self) {
        match self.handler_index.get(&(None, HandlerEvent::Close)) {
            Some(&at) => {
                let handler = &self.handlers[at];
                call(&handler.component, handler.call, Vec::new());
            }
            None => self.commands.borrow_mut().push(WindowCommand::Close),
        }
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
            std::mem::take(&mut self.structures),
            std::mem::take(&mut self.rows),
        );
        let mut changed = false;
        for parent in requests {
            changed |= pending.rebuild(parent, &self.tree, self.theme.map(Theme::name));
        }
        let Pending {
            structures,
            rows,
            staged,
            by_parent: _,
        } = pending;
        self.structures = structures;
        self.rows = rows;
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
