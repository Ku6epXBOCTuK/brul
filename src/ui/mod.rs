use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

use brul_codegen::runtime::{self, accepts_prop};
use brul_gui::{TimerOwner, WidgetEvent, WidgetId, WidgetTree};
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

fn owner_components(
    first: u64,
    components: Vec<ComponentHandle>,
) -> Vec<(TimerOwner, ComponentHandle)> {
    components
        .into_iter()
        .enumerate()
        .map(|(offset, component)| (TimerOwner::new(first + offset as u64), component))
        .collect()
}

fn schedule_timers(
    components: &[(TimerOwner, ComponentHandle)],
    commands: &Rc<RefCell<Vec<WindowCommand>>>,
) {
    for (owner, component) in components {
        let (timers, cancels) = {
            let mut component = component.borrow_mut();
            (component.take_timers(), component.take_cancels())
        };
        commands.borrow_mut().extend(
            timers
                .into_iter()
                .filter_map(|request| timer_command(request, *owner)),
        );
        commands.borrow_mut().extend(
            cancels
                .into_iter()
                .map(|id| WindowCommand::Timer(TimerCommand::Cancel { id, owner: *owner })),
        );
    }
}
pub struct Ui<T: Theme> {
    pub component: ComponentHandle,
    components: Vec<(TimerOwner, ComponentHandle)>,
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
    _scope: EffectScope,
}

struct OwnedStructure {
    structure: Structure,
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
        let components = owner_components(
            0,
            std::iter::once(Rc::clone(&component))
                .chain(parts.nested)
                .collect(),
        );
        schedule_timers(&components, &commands);
        let tree = Rc::new(RefCell::new(WidgetTree::new(widget)));
        tree.borrow_mut().set_root_scroll(scroll);
        let window_handlers = handlers
            .iter()
            .filter(|handler| handler.widget.is_none())
            .cloned()
            .collect();
        let mut ui = Self {
            component,
            components,
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
        ui.bind(bindings);
        ui.bind_props(props);
        ui.bind_structures(parts.structures);
        ui.index_handlers();
        ui
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
                _scope: scope,
            });
        }
    }

    fn bind_props(&mut self, props: Vec<PropBinding>) {
        for prop in props {
            let widget = prop.widget;
            let scope = self.scope.child();
            scope.run(|| {
                Effect::new(move || apply_prop(&prop));
            });
            self.props.push(Owned {
                widget,
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
        } = staged;
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
        let next = self.components.len() as u64;
        self.components.extend(owner_components(next, components));
        self.index_handlers();
        self.bind(bindings);
        self.bind_props(props);
        self.bind_structures(structures);
    }

    fn rebuild(&mut self) -> Result<(), ViewError> {
        let mut build = Build::new();
        let built = (self.view)(self.theme, &mut build, &self.component);
        let structures = build.take_structures();
        let nested = build.take_components();
        let built = built?;
        self.components = owner_components(
            0,
            std::iter::once(Rc::clone(&self.component))
                .chain(nested)
                .collect(),
        );
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
                        .find(|(id, _)| *id == *owner)
                        .map(|(_, component)| Rc::clone(component));
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
        schedule_timers(&self.components, &self.commands);
        std::mem::take(&mut *self.commands.borrow_mut())
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
