use super::*;

impl<T: Theme> Ui<T> {
    pub(super) fn prune_components(&mut self) {
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

    pub fn dispatch(&mut self, events: &[DispatchEvent]) -> Vec<WindowCommand> {
        for event in events {
            match event {
                DispatchEvent::Raw(raw) => self.dispatch_raw_event(raw),
                DispatchEvent::Semantic(semantic) => match &semantic.event {
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
                    _ => self.dispatch_event(semantic),
                },
            }
            self.pump_events();
        }
        self.take_commands()
    }

    pub fn dispatch_raw(&mut self, events: &[RawUiEvent]) -> Vec<WindowCommand> {
        for event in events {
            self.dispatch_raw_event(event);
            self.pump_events();
        }
        self.take_commands()
    }

    fn dispatch_raw_event(&self, event: &RawUiEvent) {
        let path = DispatchPath::new(&self.tree.borrow(), event.target);
        let kind = event.event.kind().dispatch_kind();
        self.dispatch_capabilities(
            &path,
            event,
            |hooks| hooks.raw_events.contains(kind),
            |hooks, build, state, event| (hooks.raw_event)(build, state, event),
        );
    }

    fn pump_events(&mut self) {
        for _ in 0..EVENT_DEPTH {
            let Some(pending) = runtime::take_pending() else {
                return;
            };
            match pending {
                runtime::Pending::Component(event) => {
                    call(&event.component, event.call, event.arguments);
                }
                runtime::Pending::Ui(event) => self.dispatch_event(&event),
                runtime::Pending::Raw(event) => self.dispatch_raw_event(&event),
            }
            brul_reactive::flush();
        }
        tracing::error!(depth = EVENT_DEPTH, "event chain is too deep");
    }

    fn dispatch_capabilities<E>(
        &self,
        path: &DispatchPath,
        event: &E,
        accepts: impl Fn(&Hooks) -> bool,
        invoke: impl Fn(&Hooks, &mut Build, StateId, &E) -> Dispatch,
    ) -> Dispatch {
        let handled = Rc::new(Cell::new(false));
        let mut result = Dispatch::CONTINUE;
        'targets: for target in &path.capabilities {
            for owner in self.owners.capabilities(target) {
                if !accepts(&owner.hooks) {
                    continue;
                }
                let mut build = capability_context(
                    &self.tree,
                    &self.states,
                    &self.event_sink,
                    *target,
                    owner.state,
                    &owner.hooks.component,
                    Some(Rc::clone(&handled)),
                );
                let dispatch = invoke(&owner.hooks, &mut build, owner.state, event);
                if dispatch.is_handled() {
                    handled.set(true);
                }
                result = result | dispatch;
                if dispatch.stops_capabilities() {
                    break 'targets;
                }
            }
        }
        result
    }

    fn dispatch_event(&self, event: &UiEvent) {
        let path = DispatchPath::new(&self.tree.borrow(), event.target);
        let kind = event.event.kind().dispatch_kind();
        let dispatch = self.dispatch_capabilities(
            &path,
            event,
            |hooks| hooks.events.contains(kind),
            |hooks, build, state, event| (hooks.event)(build, state, event),
        );
        if dispatch.stops_scripts() {
            return;
        }
        for target in &path.handlers {
            if let Some(handler) = self.handler_index.get(&(*target, kind)) {
                call(&handler.component, handler.call, arguments(event));
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
