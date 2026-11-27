use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use brul_gui::{DispatchEvent, PlatformViewId};
use brul_window::{
    ColorScheme, NoopPlatformBackendFactory, PlatformBackendFactory, PlatformMessageHandler,
    PlatformSpec, ScopedCommand, Waker, WindowCommand, WindowError, WindowKey, WindowSpec,
    run_windows,
};

use crate::handle::{Handle, Message, ScopedMessage, take};
use crate::ui::{AssemblyParts, CommandHost};
use crate::{
    Build, BuildParts, Built, Component, ComponentHandle, EventSink, Host, Script, Services,
    SharedStates, StateId, Theme, Ui, Value, ViewError, WindowConfig,
};

struct HandleSink(Handle);

impl EventSink for HandleSink {
    fn widget(&self, state: StateId, event: &str, arguments: Vec<Value>) {
        self.0.widget(state, event, arguments);
    }
}

pub(crate) trait CommandSink {
    fn send(&self, command: ScopedCommand);
}

impl CommandSink for Waker {
    fn send(&self, command: ScopedCommand) {
        Waker::command(self, command.window, command.command);
    }
}

pub struct WindowDef<T: Theme> {
    pub name: &'static str,
    pub config: WindowConfig,
    pub view: fn(Option<T>, &mut Build, &ComponentHandle) -> Result<Built, ViewError>,
}

/// Runs windows from a module generated out of `.brul`.
pub fn run_generated<T: Theme>(
    windows: Vec<WindowDef<T>>,
    script: Option<Script>,
    names: &'static [&'static str],
    theme: Option<T>,
    seeds: &[(&str, Value)],
    host: Option<Box<dyn Host>>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_generated_with(windows, script, names, theme, seeds, host, |_| {})
}

/// Like [`run_generated`], but hands out a [`Handle`] before the event loop
/// starts: a background thread writes DSL through it, the loop wakes and
/// redraws the windows.
pub fn run_generated_with<T: Theme>(
    windows: Vec<WindowDef<T>>,
    script: Option<Script>,
    names: &'static [&'static str],
    theme: Option<T>,
    seeds: &[(&str, Value)],
    host: Option<Box<dyn Host>>,
    ready: impl FnOnce(Handle) + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    run_generated_with_platform(
        windows,
        script,
        names,
        theme,
        seeds,
        host,
        PlatformRunOptions {
            factory: Arc::new(NoopPlatformBackendFactory),
            ready: Box::new(ready),
        },
    )
}

pub struct PlatformRunOptions {
    pub factory: Arc<dyn PlatformBackendFactory>,
    pub ready: Box<dyn FnOnce(Handle) + 'static>,
}

/// Like [`run_generated_with`], but installs a backend for platform views.
pub fn run_generated_with_platform<T: Theme>(
    windows: Vec<WindowDef<T>>,
    script: Option<Script>,
    names: &'static [&'static str],
    theme: Option<T>,
    seeds: &[(&str, Value)],
    host: Option<Box<dyn Host>>,
    options: PlatformRunOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let PlatformRunOptions { factory, ready } = options;
    crate::logging::init();
    if windows.is_empty() {
        return Err(std::io::Error::other("no window declarations").into());
    }
    tracing::info!(
        windows = windows.len(),
        theme = theme.map(Theme::name).unwrap_or("system"),
        "starting"
    );

    let component = generated_component(names, script, seeds)?;
    let keys = windows
        .iter()
        .enumerate()
        .map(|(index, window)| (window.name.to_owned(), WindowKey(index as u64)))
        .collect();
    let commands = Rc::new(RefCell::new(Vec::new()));
    let current = Rc::new(RefCell::new(theme.map(Theme::name).map(str::to_owned)));
    component.borrow_mut().set_host(CommandHost {
        commands: Rc::clone(&commands),
        app: host,
        windows: Rc::new(keys),
        theme: Rc::clone(&current),
    });
    let pending = std::mem::take(&mut *commands.borrow_mut());
    let runtime = Rc::new(RefCell::new(Runtime::new(
        windows,
        component,
        commands,
        Rc::clone(&current),
        theme.is_none(),
        pending,
        None,
    )));
    runtime.borrow_mut().platform = factory;

    let creator = Rc::clone(&runtime);
    let handler = Rc::clone(&runtime);
    let closed = Rc::clone(&runtime);
    let waking = Rc::clone(&runtime);
    let attaching = Rc::clone(&runtime);
    let queue = Arc::new(Mutex::new(Vec::new()));
    let waking_queue = Arc::clone(&queue);
    run_windows(
        WindowKey(0),
        move |key, scheme| creator.borrow_mut().create(key, scheme),
        move |key, events| handler.borrow_mut().dispatch(key, events),
        move |key| closed.borrow_mut().close(key),
        move |waker| {
            attaching.borrow_mut().sink = Some(Box::new(waker.clone()));
            let handle = Handle::new(queue, waker, WindowKey(0));
            attaching.borrow_mut().handle = Some(handle.clone());
            ready(handle);
        },
        move || {
            let messages = take(&waking_queue);
            if !messages.is_empty() {
                waking.borrow_mut().apply(messages);
            }
        },
    )?;
    Ok(())
}

pub(crate) fn generated_component(
    names: &'static [&'static str],
    script: Option<Script>,
    seeds: &[(&str, Value)],
) -> Result<ComponentHandle, Box<dyn std::error::Error>> {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    component.borrow_mut().set_names(names);
    for (name, value) in seeds {
        component.borrow_mut().declare(name, value.clone(), false)?;
    }
    match script {
        Some(Script::Generated { init, dispatch }) => {
            component.borrow_mut().set_dispatch(dispatch);
            init(&mut component.borrow_mut())?;
        }
        Some(Script::Typed {
            init,
            call,
            dispatch,
        }) => {
            component.borrow_mut().set_dispatch(dispatch);
            component.borrow_mut().set_typed_call(call);
            init(&mut component.borrow_mut())?;
        }
        None => {}
    }
    Ok(component)
}

#[cfg(test)]
pub(crate) fn generated_built<T: Theme>(
    view: fn(Option<T>, &mut Build, &ComponentHandle) -> Result<Built, ViewError>,
    names: &'static [&'static str],
    theme: Option<T>,
    seeds: &[(&str, Value)],
    script: Option<Script>,
) -> Result<(ComponentHandle, Built, Vec<crate::Structure>), Box<dyn std::error::Error>> {
    let component = generated_component(names, script, seeds)?;
    let mut build = Build::new(Services::default());
    let built = view(theme, &mut build, &component).map_err(std::io::Error::other)?;
    Ok((component, built, build.finish().structures))
}

struct Runtime<T: Theme> {
    defs: Vec<WindowDef<T>>,
    component: ComponentHandle,
    uis: HashMap<WindowKey, Ui<T>>,
    commands: Rc<RefCell<Vec<WindowCommand>>>,
    theme: Rc<RefCell<Option<String>>>,
    follows_system: bool,
    pending: Vec<WindowCommand>,
    sink: Option<Box<dyn CommandSink>>,
    handle: Option<Handle>,
    platform: Arc<dyn PlatformBackendFactory>,
}

impl<T: Theme> Runtime<T> {
    fn new(
        defs: Vec<WindowDef<T>>,
        component: ComponentHandle,
        commands: Rc<RefCell<Vec<WindowCommand>>>,
        theme: Rc<RefCell<Option<String>>>,
        follows_system: bool,
        pending: Vec<WindowCommand>,
        sink: Option<Box<dyn CommandSink>>,
    ) -> Self {
        let mut runtime = Self {
            defs,
            component,
            uis: HashMap::new(),
            commands,
            theme,
            follows_system,
            pending,
            sink,
            handle: None,
            platform: Arc::new(NoopPlatformBackendFactory),
        };
        let pending = std::mem::take(&mut runtime.pending);
        runtime.pending = runtime.absorb(pending);
        runtime
    }

    fn absorb(&mut self, commands: Vec<WindowCommand>) -> Vec<WindowCommand> {
        let mut rest = Vec::new();
        for command in commands {
            match command {
                WindowCommand::SetTheme(theme) => self.apply_theme(&theme),
                command => rest.push(command),
            }
        }
        rest
    }

    fn apply_theme(&mut self, name: &str) {
        let Some(theme) = T::parse(name) else {
            let error = crate::UnknownTheme::new(name);
            tracing::error!(%error, "theme change failed");
            return;
        };
        tracing::info!(theme = name, "applying theme");
        self.follows_system = false;
        *self.theme.borrow_mut() = Some(theme.name().to_owned());
        for ui in self.uis.values_mut() {
            if let Err(error) = ui.set_theme(theme) {
                tracing::error!(%error, "theme change failed");
            }
        }
        self.component.borrow_mut().touch();
        brul_reactive::flush();
        for ui in self.uis.values_mut() {
            ui.settle();
        }
    }

    fn theme_for(&self, scheme: ColorScheme) -> Option<T> {
        if self.follows_system {
            T::parse(scheme_theme(None, scheme))
        } else {
            self.theme.borrow().as_deref().and_then(T::parse)
        }
    }

    fn create(&mut self, key: WindowKey, scheme: ColorScheme) -> Result<WindowSpec, WindowError> {
        let index = usize::try_from(key.0)
            .ok()
            .filter(|index| *index < self.defs.len())
            .ok_or(WindowError::Undeclared(key))?;
        let (config, view) = {
            let def = &self.defs[index];
            (def.config.clone(), def.view)
        };
        let theme = self.theme_for(scheme);
        let messages: PlatformMessageHandler = match self.handle.clone() {
            Some(handle) => Arc::new(move |id: PlatformViewId, body: String| {
                handle.widget(id.state_id(), "message", vec![Value::String(body.into())]);
            }),
            None => Arc::new(|_, _| {}),
        };
        if let Some(ui) = self.uis.get_mut(&key) {
            tracing::debug!(%key, "reusing window");
            let current = self.theme.borrow().as_deref().and_then(T::parse);
            if let Some(theme) = theme.filter(|theme| Some(*theme) != current) {
                ui.set_theme(theme).map_err(WindowError::app)?;
            }
            self.component.borrow_mut().touch();
            brul_reactive::flush();
            ui.settle();
            return Ok(WindowSpec {
                config,
                tree: Rc::clone(&ui.tree),
                commands: Vec::new(),
                platform: Some(PlatformSpec {
                    factory: Arc::clone(&self.platform),
                    messages: Arc::clone(&messages),
                }),
            });
        }

        tracing::info!(
            %key,
            theme = theme.map(Theme::name).unwrap_or("default"),
            "creating window"
        );
        let initial = self.theme.borrow().as_deref().and_then(T::parse);
        let states = SharedStates::default();
        let event_sink = self
            .handle
            .clone()
            .map(|handle| Arc::new(HandleSink(handle)) as Arc<dyn EventSink>);
        let mut build = Build::new(Services {
            states: Rc::clone(&states),
            event_sink: event_sink.clone(),
            clipboard: None,
        });
        let built = view(initial, &mut build, &self.component).map_err(WindowError::app)?;
        let BuildParts {
            structures,
            components,
            capabilities,
        } = build.finish();
        let mut ui = Ui::assemble_in(
            Rc::clone(&self.component),
            built,
            view,
            Rc::clone(&self.theme),
            AssemblyParts {
                structures,
                capabilities,
                nested: components,
                states,
                event_sink,
                clipboard: None,
            },
            Rc::clone(&self.commands),
            config.scroll,
        );
        if let Some(theme) = theme.filter(|theme| Some(*theme) != initial) {
            ui.set_theme(theme).map_err(WindowError::app)?;
        }
        let mut commands = std::mem::take(&mut self.pending);
        commands.extend(ui.dispatch(&[]));
        let tree = Rc::clone(&ui.tree);
        self.uis.insert(key, ui);
        Ok(WindowSpec {
            config,
            tree,
            commands,
            platform: Some(PlatformSpec {
                factory: Arc::clone(&self.platform),
                messages,
            }),
        })
    }

    fn dispatch(&mut self, key: WindowKey, events: &[DispatchEvent]) -> Vec<WindowCommand> {
        let Some(ui) = self.uis.get_mut(&key) else {
            tracing::warn!(%key, "event for an unknown window");
            return Vec::new();
        };
        tracing::debug!(%key, events = events.len(), "dispatching events");
        let mut commands = ui.dispatch(events);
        brul_reactive::flush();
        for ui in self.uis.values_mut() {
            ui.settle();
        }
        commands.extend(std::mem::take(&mut *self.commands.borrow_mut()));
        tracing::debug!(%key, ?commands, "window commands");
        self.absorb(commands)
    }

    fn close(&mut self, key: WindowKey) {
        tracing::info!(%key, "closing window");
        self.uis.remove(&key);
    }

    fn apply(&mut self, messages: Vec<ScopedMessage>) {
        for scoped in messages {
            let window = scoped.window;
            match scoped.message {
                Message::Set(name, value) => {
                    if let Err(error) = self.component.borrow_mut().assign(&name, value) {
                        tracing::error!(%error, %name, "handle set failed");
                    }
                }
                Message::Call(function, arguments) => {
                    if let Err(error) = self
                        .component
                        .borrow_mut()
                        .call_function(&function, arguments)
                    {
                        tracing::error!(%error, %function, "handle call failed");
                    }
                }
                Message::Invoke(message) => {
                    if let Err(error) = self.component.borrow_mut().call_typed(&*message) {
                        tracing::error!(%error, "typed handle call failed");
                    }
                }
                Message::Widget {
                    state,
                    event,
                    arguments,
                } => {
                    let routed = self
                        .uis
                        .get(&window)
                        .is_some_and(|ui| ui.call_widget_event(state, &event, arguments));
                    if !routed {
                        tracing::warn!(?state, %event, "widget event has no owner");
                    }
                }
            }
            brul_reactive::flush();
            for ui in self.uis.values_mut() {
                ui.settle();
            }
            self.drain_commands(window);
        }
    }

    fn drain_commands(&mut self, window: WindowKey) {
        let commands = std::mem::take(&mut *self.commands.borrow_mut());
        let rest = self.absorb(commands);
        if let Some(sink) = &self.sink {
            for command in rest {
                sink.send(ScopedCommand { window, command });
            }
        }
    }
}

pub(crate) fn scheme_theme(fixed: Option<&str>, scheme: ColorScheme) -> &str {
    fixed.unwrap_or(match scheme {
        ColorScheme::Light => "light",
        ColorScheme::Dark => "dark",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestTheme;

    use std::collections::HashMap;

    use brul_gui::{Event, Widget, WidgetId};

    use crate::ui::{CommandHost, Ui};
    use crate::{
        Binding, BindingTarget, Built, ComponentHandle, Dyn, EventKind, Handler, StateId, Value,
    };

    use fixtures::*;
    use launch::*;
    use theme::*;
    #[path = "parts/fixtures.rs"]
    mod fixtures;
    #[path = "parts/handles.rs"]
    mod handles;
    #[path = "parts/launch.rs"]
    mod launch;
    #[path = "parts/theme.rs"]
    mod theme;
    #[path = "parts/windows.rs"]
    mod windows;
}
