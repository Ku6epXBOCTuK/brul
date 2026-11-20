use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use brul_gui::WidgetEvent;
use brul_window::{
    ColorScheme, Waker, WindowCommand, WindowError, WindowKey, WindowSpec, run_windows,
};

use crate::handle::{Handle, Message, take};
use crate::ui::CommandHost;
use crate::{
    Build, Built, Component, ComponentHandle, Host, Script, Theme, Ui, Value, ViewError,
    WindowConfig,
};

pub struct WindowDef<T: Theme> {
    pub name: &'static str,
    pub config: WindowConfig,
    pub view: fn(Option<T>, &mut Build, &ComponentHandle) -> Result<Built, ViewError>,
}

/// Запускает окна по модулю, сгенерированному из `.brul`.
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

/// Как [`run_generated`], но отдаёт [`Handle`] до старта цикла событий:
/// фоновый поток пишет им в DSL, цикл просыпается и перерисовывает окна.
pub fn run_generated_with<T: Theme>(
    windows: Vec<WindowDef<T>>,
    script: Option<Script>,
    names: &'static [&'static str],
    theme: Option<T>,
    seeds: &[(&str, Value)],
    host: Option<Box<dyn Host>>,
    ready: impl FnOnce(Handle) + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let shown = Rc::new(RefCell::new(theme.map(Theme::name).map(str::to_owned)));
    component.borrow_mut().set_host(CommandHost {
        commands: Rc::clone(&commands),
        app: host,
        windows: Rc::new(keys),
        theme: Rc::clone(&shown),
    });
    let pending = std::mem::take(&mut *commands.borrow_mut());
    let runtime = Rc::new(RefCell::new(Runtime::new(
        windows, component, commands, theme, shown, pending,
    )));

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
            attaching.borrow_mut().waker = Some(waker.clone());
            ready(Handle::new(queue, waker));
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
    let mut build = Build::new();
    let built = view(theme, &mut build, &component).map_err(std::io::Error::other)?;
    Ok((component, built, build.take_structures()))
}

struct Runtime<T: Theme> {
    defs: Vec<WindowDef<T>>,
    component: ComponentHandle,
    uis: HashMap<WindowKey, Ui<T>>,
    commands: Rc<RefCell<Vec<WindowCommand>>>,
    fixed_theme: Option<T>,
    theme: Option<T>,
    shown: Rc<RefCell<Option<String>>>,
    pending: Vec<WindowCommand>,
    waker: Option<Waker>,
}

impl<T: Theme> Runtime<T> {
    fn new(
        defs: Vec<WindowDef<T>>,
        component: ComponentHandle,
        commands: Rc<RefCell<Vec<WindowCommand>>>,
        fixed_theme: Option<T>,
        shown: Rc<RefCell<Option<String>>>,
        pending: Vec<WindowCommand>,
    ) -> Self {
        let mut runtime = Self {
            defs,
            component,
            uis: HashMap::new(),
            commands,
            fixed_theme,
            theme: None,
            shown,
            pending,
            waker: None,
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
        self.theme = Some(theme);
        *self.shown.borrow_mut() = Some(name.to_owned());
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
        self.theme
            .or(self.fixed_theme)
            .or_else(|| T::parse(scheme_theme(None, scheme)))
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
        *self.shown.borrow_mut() = theme.map(Theme::name).map(str::to_owned);
        if let Some(ui) = self.uis.get_mut(&key) {
            tracing::debug!(%key, "reusing window");
            if let Some(theme) = theme {
                ui.set_theme(theme).map_err(WindowError::app)?;
            }
            self.component.borrow_mut().touch();
            brul_reactive::flush();
            ui.settle();
            return Ok(WindowSpec {
                config,
                tree: Rc::clone(&ui.tree),
                commands: Vec::new(),
            });
        }

        tracing::info!(
            %key,
            theme = theme.map(Theme::name).unwrap_or("default"),
            "creating window"
        );
        let mut build = Build::new();
        let built =
            view(self.fixed_theme, &mut build, &self.component).map_err(WindowError::app)?;
        let structures = build.take_structures();
        let mut ui = Ui::assemble_with(
            Rc::clone(&self.component),
            built,
            view,
            self.fixed_theme,
            structures,
            Rc::clone(&self.commands),
            config.scroll,
        );
        if let Some(theme) = theme {
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
        })
    }

    fn dispatch(&mut self, key: WindowKey, events: &[WidgetEvent]) -> Vec<WindowCommand> {
        let Some(ui) = self.uis.get_mut(&key) else {
            tracing::warn!(%key, "event for an unknown window");
            return Vec::new();
        };
        tracing::debug!(%key, events = events.len(), "dispatching events");
        let commands = ui.dispatch(events);
        tracing::debug!(%key, ?commands, "window commands");
        brul_reactive::flush();
        for ui in self.uis.values_mut() {
            ui.settle();
        }
        self.absorb(commands)
    }

    fn close(&mut self, key: WindowKey) {
        tracing::info!(%key, "closing window");
        self.uis.remove(&key);
    }

    fn apply(&mut self, messages: Vec<Message>) {
        for message in messages {
            match message {
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
            }
        }
        brul_reactive::flush();
        for ui in self.uis.values_mut() {
            ui.settle();
        }
        self.drain_commands();
    }

    fn drain_commands(&mut self) {
        let commands = std::mem::take(&mut *self.commands.borrow_mut());
        let rest = self.absorb(commands);
        if let Some(waker) = &self.waker {
            for command in rest {
                waker.command(command);
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

    use brul_gui::{Widget, WidgetEvent, WidgetId};

    use crate::ui::{CommandHost, Ui};
    use crate::{
        Binding, BindingTarget, Built, ComponentHandle, Dyn, Handler, HandlerEvent, Value,
    };

    const NAMES: &[&str] = &["name"];

    fn name(component: &mut crate::Component) -> Result<String, crate::Error> {
        (&component.read("name")?).try_into()
    }

    fn noop_init(_: &mut Component) -> Result<(), crate::Error> {
        Ok(())
    }

    fn noop(_: &mut Component, _: &str, _: Vec<Dyn>) -> Result<Dyn, crate::Error> {
        Ok(Dyn::null())
    }

    fn declares_name(component: &mut Component) -> Result<(), crate::Error> {
        component.declare("name", Value::String("from script".into()), false)
    }

    fn mark(component: &mut Component, name: &str, _: Vec<Dyn>) -> Result<Dyn, crate::Error> {
        if name != "mark" {
            return Err(crate::Error::UnknownFunction(name.to_owned()));
        }
        component.assign("name", Value::String("called".into()))?;
        Ok(Dyn::null())
    }

    fn mark_with_arg(
        component: &mut Component,
        name: &str,
        arguments: Vec<Dyn>,
    ) -> Result<Dyn, crate::Error> {
        if name != "mark" {
            return Err(crate::Error::UnknownFunction(name.to_owned()));
        }
        let label = Value::from(arguments[0].clone());
        component.assign("name", label)?;
        Ok(Dyn::null())
    }

    fn generated_view(
        _theme: Option<TestTheme>,
        _build: &mut Build,
        component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        let widget = Widget::text("");
        let id = widget.id();
        Ok(Built {
            widget,
            bindings: vec![Binding::new(component, id, BindingTarget::Text(name))],
            handlers: Vec::new(),
            props: Vec::new(),
        })
    }

    fn defs() -> Vec<WindowDef<TestTheme>> {
        ["First", "Second"]
            .into_iter()
            .map(|name| WindowDef {
                name,
                config: WindowConfig::default(),
                view: generated_view,
            })
            .collect()
    }

    fn runtime(theme: Option<TestTheme>) -> Runtime<TestTheme> {
        let component = generated_component(NAMES, None, &[]).expect("component");
        Runtime::new(
            defs(),
            component,
            Rc::new(RefCell::new(Vec::new())),
            theme,
            Rc::new(RefCell::new(
                theme.map(crate::Theme::name).map(str::to_owned),
            )),
            Vec::new(),
        )
    }

    fn assembled(
        theme: Option<TestTheme>,
        seeds: &[(&str, Value)],
        script: Option<Script>,
    ) -> (WidgetId, Ui<TestTheme>) {
        let (component, built, structures) =
            generated_built(generated_view, NAMES, theme, seeds, script).expect("launch succeeds");
        let id = built.widget.id();
        (
            id,
            Ui::assemble(component, built, generated_view, None, structures, None),
        )
    }

    #[test]
    fn generated_launch_declares_seeds() {
        let seeds = [("name", Value::String("from seed".into()))];
        let (id, ui) = assembled(None, &seeds, None);
        brul_reactive::flush();
        assert!(!ui.tree.borrow_mut().set_text(id, "from seed".to_owned()));
    }

    #[test]
    fn generated_launch_runs_the_script() {
        let script = Script::Generated {
            init: declares_name,
            dispatch: noop,
        };
        let (id, ui) = assembled(None, &[], Some(script));
        brul_reactive::flush();
        assert!(!ui.tree.borrow_mut().set_text(id, "from script".to_owned()));
    }

    #[test]
    fn a_window_is_built_once_and_reused() {
        let mut runtime = runtime(None);
        let first = runtime
            .create(WindowKey(1), ColorScheme::Dark)
            .expect("second window");
        let again = runtime
            .create(WindowKey(1), ColorScheme::Light)
            .expect("second window again");

        assert!(Rc::ptr_eq(&first.tree, &again.tree));
        assert_eq!(first.config.title, again.config.title);
    }

    #[test]
    fn an_unknown_key_is_an_error() {
        let mut runtime = runtime(None);
        let error = match runtime.create(WindowKey(7), ColorScheme::Dark) {
            Ok(_) => panic!("unknown window was created"),
            Err(error) => error,
        };
        assert!(
            matches!(error, WindowError::Undeclared(WindowKey(7))),
            "{error}"
        );
    }

    #[test]
    fn a_theme_command_reaches_every_open_window() {
        let mut runtime = runtime(None);
        runtime
            .create(WindowKey(0), ColorScheme::Dark)
            .expect("first window");
        runtime
            .create(WindowKey(1), ColorScheme::Dark)
            .expect("second window");
        runtime
            .commands
            .borrow_mut()
            .push(WindowCommand::SetTheme("light".to_owned()));

        assert!(runtime.dispatch(WindowKey(0), &[]).is_empty());
        for ui in runtime.uis.values() {
            assert_eq!(ui.theme, Some(TestTheme::Light));
        }
    }

    thread_local! {
        static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn theme_text(component: &mut crate::Component) -> Result<String, crate::Error> {
        let theme = component.call("theme", &[])?;
        let name = match &theme {
            Value::String(name) => name.to_string(),
            _ => "none".to_owned(),
        };
        SEEN.with(|seen| seen.borrow_mut().push(name.clone()));
        Ok(name)
    }

    fn theme_view(
        _theme: Option<TestTheme>,
        _build: &mut Build,
        component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        let widget = Widget::text("");
        let id = widget.id();
        Ok(Built {
            widget,
            bindings: vec![Binding::new(component, id, BindingTarget::Text(theme_text))],
            handlers: Vec::new(),
            props: Vec::new(),
        })
    }

    #[test]
    fn a_theme_change_rereads_the_theme_from_the_script() {
        let component = generated_component(NAMES, None, &[]).expect("component");
        let commands = Rc::new(RefCell::new(Vec::new()));
        let shown = Rc::new(RefCell::new(None));
        component.borrow_mut().set_host(CommandHost {
            commands: Rc::clone(&commands),
            app: None,
            windows: Rc::new(HashMap::new()),
            theme: Rc::clone(&shown),
        });
        let defs = vec![WindowDef {
            name: "Only",
            config: WindowConfig::default(),
            view: theme_view,
        }];
        let mut runtime = Runtime::new(defs, component, commands, None, shown, Vec::new());

        runtime
            .create(WindowKey(0), ColorScheme::Dark)
            .expect("window");
        assert_eq!(
            SEEN.with(|seen| seen.borrow().last().cloned()),
            Some("dark".to_owned())
        );

        runtime
            .commands
            .borrow_mut()
            .push(WindowCommand::SetTheme("light".to_owned()));
        runtime.dispatch(WindowKey(0), &[]);
        assert_eq!(
            SEEN.with(|seen| seen.borrow().last().cloned()),
            Some("light".to_owned())
        );
    }

    thread_local! {
        static LAST_ID: RefCell<Option<WidgetId>> = const { RefCell::new(None) };
    }

    fn seeded_view(
        _theme: Option<TestTheme>,
        _build: &mut Build,
        component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        let widget = Widget::text("");
        let id = widget.id();
        LAST_ID.with(|slot| *slot.borrow_mut() = Some(id));
        Ok(Built {
            widget,
            bindings: vec![Binding::new(component, id, BindingTarget::Text(name))],
            handlers: Vec::new(),
            props: Vec::new(),
        })
    }

    #[test]
    fn a_handle_message_reaches_the_component() {
        let seeds = [("name", Value::String("".into()))];
        let component = generated_component(NAMES, None, &seeds).expect("component");
        let defs = vec![WindowDef {
            name: "Only",
            config: WindowConfig::default(),
            view: seeded_view,
        }];
        let mut runtime = Runtime::new(
            defs,
            component,
            Rc::new(RefCell::new(Vec::new())),
            None,
            Rc::new(RefCell::new(None)),
            Vec::new(),
        );
        runtime
            .create(WindowKey(0), ColorScheme::Dark)
            .expect("window");
        let id = LAST_ID
            .with(|slot| slot.borrow_mut().take())
            .expect("widget");

        runtime.apply(vec![Message::Set("name".to_owned(), "live".into())]);

        let tree = &mut runtime.uis[&WindowKey(0)].tree.borrow_mut();
        assert!(!tree.set_text(id, "live".to_owned()));
    }

    #[test]
    fn a_handle_call_runs_the_script_function() {
        let seeds = [("name", Value::String("".into()))];
        let script = Script::Generated {
            init: noop_init,
            dispatch: mark,
        };
        let component = generated_component(NAMES, Some(script), &seeds).expect("component");
        let defs = vec![WindowDef {
            name: "Only",
            config: WindowConfig::default(),
            view: seeded_view,
        }];
        let mut runtime = Runtime::new(
            defs,
            component,
            Rc::new(RefCell::new(Vec::new())),
            None,
            Rc::new(RefCell::new(None)),
            Vec::new(),
        );
        runtime
            .create(WindowKey(0), ColorScheme::Dark)
            .expect("window");
        let id = LAST_ID
            .with(|slot| slot.borrow_mut().take())
            .expect("widget");

        runtime.apply(vec![Message::Call("mark".to_owned(), Vec::new())]);

        let tree = &mut runtime.uis[&WindowKey(0)].tree.borrow_mut();
        assert!(!tree.set_text(id, "called".to_owned()));
    }

    #[test]
    fn a_handle_call_passes_arguments() {
        const CALL_NAMES: &[&str] = &["name", "label"];
        let seeds = [("name", Value::from(""))];
        let script = Script::Generated {
            init: noop_init,
            dispatch: mark_with_arg,
        };
        let component = generated_component(CALL_NAMES, Some(script), &seeds).expect("component");
        let mut runtime = Runtime::new(
            defs(),
            component,
            Rc::new(RefCell::new(Vec::new())),
            None,
            Rc::new(RefCell::new(None)),
            Vec::new(),
        );

        runtime.apply(vec![Message::Call(
            "mark".to_owned(),
            vec![Value::from("argument")],
        )]);

        assert_eq!(
            runtime.component.borrow().read("name"),
            Ok(Value::from("argument"))
        );
    }

    #[test]
    fn a_handle_set_carries_a_list() {
        let component =
            generated_component(NAMES, None, &[("name", Value::Null)]).expect("component");
        let mut runtime = Runtime::new(
            defs(),
            component,
            Rc::new(RefCell::new(Vec::new())),
            None,
            Rc::new(RefCell::new(None)),
            Vec::new(),
        );
        let rows = Value::Array(vec![Value::from(1.0), Value::from(2.0)].into());

        runtime.apply(vec![Message::Set("name".to_owned(), rows.clone())]);

        assert_eq!(runtime.component.borrow().read("name"), Ok(rows));
    }

    #[test]
    fn closing_a_window_drops_its_ui() {
        let mut runtime = runtime(None);
        runtime
            .create(WindowKey(0), ColorScheme::Dark)
            .expect("first window");
        runtime.close(WindowKey(0));
        assert!(runtime.uis.is_empty());
    }

    fn key_view(component: &ComponentHandle, call: crate::EventCall) -> Result<Built, ViewError> {
        let widget = Widget::text("");
        Ok(Built {
            widget,
            bindings: Vec::new(),
            handlers: vec![Handler::new(component, None, HandlerEvent::Key, call)],
            props: Vec::new(),
        })
    }

    fn picker_view(
        _theme: Option<TestTheme>,
        _build: &mut Build,
        component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        key_view(component, key_exit)
    }

    fn settings_view(
        _theme: Option<TestTheme>,
        _build: &mut Build,
        component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        key_view(component, key_exit)
    }

    fn key_exit(component: &mut Component, arguments: &[Dyn]) -> Result<Dyn, crate::Error> {
        if arguments[0].field("code")?.as_text()? == "Escape" {
            <crate::builtins::Exit as crate::HostCommand>::invoke(component, ())?;
        }
        Ok(Dyn::null())
    }

    fn key_exit_dispatch(
        component: &mut Component,
        name: &str,
        arguments: Vec<Dyn>,
    ) -> Result<Dyn, crate::Error> {
        if name != "handleKey" && name != "handleSettingsKey" {
            return Err(crate::Error::UnknownFunction(name.to_owned()));
        }
        let code = crate::eval::read_field(&Value::from(arguments[0].clone()), "code")?;
        if matches!(code, Value::String(text) if text.as_ref() == "Escape") {
            crate::eval::call(component, "exit", Vec::new())?;
        }
        Ok(Dyn::null())
    }

    #[test]
    fn a_key_closes_only_the_window_that_received_it() {
        let names: &'static [&'static str] =
            &["handleKey", "handleSettingsKey", "exit", "ev", "code"];
        let script = Script::Generated {
            init: noop_init,
            dispatch: key_exit_dispatch,
        };
        let component = generated_component(names, Some(script), &[]).expect("component");
        let commands = Rc::new(RefCell::new(Vec::new()));
        let keys = HashMap::from([
            ("Picker".to_owned(), WindowKey(0)),
            ("Settings".to_owned(), WindowKey(1)),
        ]);
        component.borrow_mut().set_host(CommandHost {
            commands: Rc::clone(&commands),
            app: None,
            windows: Rc::new(keys),
            theme: Rc::new(RefCell::new(None)),
        });
        let defs = vec![
            WindowDef {
                name: "Picker",
                config: WindowConfig::default(),
                view: picker_view,
            },
            WindowDef {
                name: "Settings",
                config: WindowConfig::default(),
                view: settings_view,
            },
        ];
        let mut runtime = Runtime::new(
            defs,
            component,
            commands,
            None,
            Rc::new(RefCell::new(None)),
            Vec::new(),
        );
        runtime
            .create(WindowKey(0), ColorScheme::Dark)
            .expect("picker");
        runtime
            .create(WindowKey(1), ColorScheme::Dark)
            .expect("settings");

        let commands = runtime.dispatch(
            WindowKey(1),
            &[WidgetEvent::Key {
                key: brul_gui::KeyPress::from("Escape"),
                modifiers: brul_gui::KeyModifiers::default(),
            }],
        );

        assert_eq!(commands, vec![WindowCommand::Close]);
        assert!(runtime.uis.contains_key(&WindowKey(0)));
    }

    #[test]
    fn a_theme_parses_from_its_name() {
        assert_eq!(TestTheme::parse("dark"), Some(TestTheme::Dark));
        assert_eq!(TestTheme::parse("sepia"), None);
    }

    #[test]
    fn system_theme_follows_the_color_scheme() {
        assert_eq!(scheme_theme(None, ColorScheme::Light), "light");
        assert_eq!(scheme_theme(None, ColorScheme::Dark), "dark");
    }
}
