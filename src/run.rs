use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use brul_gui::WidgetEvent;
use brul_window::{ColorScheme, WindowCommand, WindowKey, WindowSpec, run_windows};

use crate::handle::{Handle, Message, take};
use crate::ui::CommandHost;
use crate::{
    Built, Component, ComponentHandle, Host, Ui, Value, WindowConfig, execute_program, script,
};

pub struct WindowDef {
    pub name: &'static str,
    pub config: WindowConfig,
    pub view: fn(Option<&str>, &ComponentHandle) -> Result<Built, String>,
}

/// Запускает окна по модулю, сгенерированному из `.brul`.
pub fn run_generated(
    windows: Vec<WindowDef>,
    script: Option<&script::Program>,
    names: &'static [&'static str],
    theme: Option<&str>,
    seeds: &[(&str, Value)],
    host: Option<Box<dyn Host>>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_generated_with(windows, script, names, theme, seeds, host, |_| {})
}

/// Как [`run_generated`], но отдаёт [`Handle`] до старта цикла событий:
/// фоновый поток пишет им в DSL, цикл просыпается и перерисовывает окна.
pub fn run_generated_with(
    windows: Vec<WindowDef>,
    script: Option<&script::Program>,
    names: &'static [&'static str],
    theme: Option<&str>,
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
        theme = theme.unwrap_or("system"),
        "starting"
    );

    let component = generated_component(names, script, seeds)?;
    let keys = windows
        .iter()
        .enumerate()
        .map(|(index, window)| (window.name.to_owned(), WindowKey(index as u64)))
        .collect();
    let commands = Rc::new(RefCell::new(Vec::new()));
    let shown = Rc::new(RefCell::new(theme.map(str::to_owned)));
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
    let queue = Arc::new(Mutex::new(Vec::new()));
    let waking_queue = Arc::clone(&queue);
    run_windows(
        WindowKey(0),
        move |key, scheme| creator.borrow_mut().create(key, scheme),
        move |key, events| handler.borrow_mut().dispatch(key, events),
        move |key| closed.borrow_mut().close(key),
        move |waker| ready(Handle::new(queue, waker)),
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
    script: Option<&script::Program>,
    seeds: &[(&str, Value)],
) -> Result<ComponentHandle, Box<dyn std::error::Error>> {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    component.borrow_mut().set_names(names);
    for (name, value) in seeds {
        component.borrow_mut().declare(name, value.clone(), false)?;
    }
    if let Some(program) = script {
        execute_program(&mut component.borrow_mut(), program)?;
    }
    Ok(component)
}

#[cfg(test)]
pub(crate) fn generated_built(
    view: fn(Option<&str>, &ComponentHandle) -> Result<Built, String>,
    names: &'static [&'static str],
    theme: Option<&str>,
    seeds: &[(&str, Value)],
    script: Option<&script::Program>,
) -> Result<(ComponentHandle, Built), Box<dyn std::error::Error>> {
    let component = generated_component(names, script, seeds)?;
    let built = view(theme, &component).map_err(std::io::Error::other)?;
    Ok((component, built))
}

struct Runtime {
    defs: Vec<WindowDef>,
    component: ComponentHandle,
    uis: HashMap<WindowKey, Ui>,
    commands: Rc<RefCell<Vec<WindowCommand>>>,
    fixed_theme: Option<String>,
    theme: Option<String>,
    shown: Rc<RefCell<Option<String>>>,
    pending: Vec<WindowCommand>,
}

impl Runtime {
    fn new(
        defs: Vec<WindowDef>,
        component: ComponentHandle,
        commands: Rc<RefCell<Vec<WindowCommand>>>,
        fixed_theme: Option<&str>,
        shown: Rc<RefCell<Option<String>>>,
        pending: Vec<WindowCommand>,
    ) -> Self {
        let mut runtime = Self {
            defs,
            component,
            uis: HashMap::new(),
            commands,
            fixed_theme: fixed_theme.map(str::to_owned),
            theme: None,
            shown,
            pending,
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

    fn apply_theme(&mut self, theme: &str) {
        tracing::info!(theme, "applying theme");
        self.theme = Some(theme.to_owned());
        *self.shown.borrow_mut() = Some(theme.to_owned());
        for ui in self.uis.values_mut() {
            if let Err(error) = ui.set_theme(theme)
                && !error.message.contains("unknown theme")
            {
                tracing::error!(%error, "theme change failed");
            }
        }
        self.component.borrow_mut().touch();
        brul_reactive::flush();
        for ui in self.uis.values_mut() {
            ui.settle();
        }
    }

    fn theme_for(&self, scheme: ColorScheme) -> String {
        match &self.theme {
            Some(theme) => theme.clone(),
            None => scheme_theme(self.fixed_theme.as_deref(), scheme).to_owned(),
        }
    }

    fn create(&mut self, key: WindowKey, scheme: ColorScheme) -> Result<WindowSpec, String> {
        let index = usize::try_from(key.0)
            .ok()
            .filter(|index| *index < self.defs.len())
            .ok_or_else(|| format!("{key} is not declared"))?;
        let (config, view) = {
            let def = &self.defs[index];
            (def.config.clone(), def.view)
        };
        let theme = self.theme_for(scheme);
        *self.shown.borrow_mut() = Some(theme.clone());
        if let Some(ui) = self.uis.get_mut(&key) {
            tracing::debug!(%key, %theme, "reusing window");
            if let Err(error) = ui.set_theme(&theme)
                && !error.message.contains("unknown theme")
            {
                tracing::error!(%error, "theme change failed");
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

        tracing::info!(%key, %theme, "creating window");
        let _ = crate::take_structures();
        let built = view(self.fixed_theme.as_deref(), &self.component)?;
        let mut ui = Ui::assemble_with(
            Rc::clone(&self.component),
            built,
            view,
            self.fixed_theme.clone(),
            Rc::clone(&self.commands),
        );
        if let Err(error) = ui.set_theme(&theme)
            && !error.message.contains("unknown theme")
        {
            tracing::error!(%error, "theme change failed");
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
                Message::Set(name, signal) => {
                    if let Err(error) = self.component.borrow_mut().assign(&name, signal.value()) {
                        tracing::error!(%error, %name, "handle set failed");
                    }
                }
                Message::Call(function) => {
                    if let Err(error) = self.component.borrow_mut().call(&function, &[]) {
                        tracing::error!(%error, %function, "handle call failed");
                    }
                }
            }
        }
        brul_reactive::flush();
        for ui in self.uis.values_mut() {
            ui.settle();
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

    use std::collections::HashMap;

    use brul_gui::{Widget, WidgetEvent, WidgetId};
    use brul_syntax::ast::{Expr, ExprKind, Position};

    use crate::handle::Signal;
    use crate::ui::{CommandHost, Ui};
    use crate::{Binding, BindingTarget, Built, ComponentHandle, Handler, HandlerEvent, Value};

    const NAMES: &[&str] = &["name"];

    fn name(component: &mut crate::Component) -> Result<Value, crate::Error> {
        component.read("name")
    }

    fn generated_view(theme: Option<&str>, component: &ComponentHandle) -> Result<Built, String> {
        if theme == Some("sepia") {
            return Err("unknown theme".to_owned());
        }
        let widget = Widget::text("");
        let id = widget.id();
        Ok(Built {
            widget,
            bindings: vec![Binding::new(component, id, BindingTarget::Text(name))],
            handlers: Vec::new(),
            props: Vec::new(),
        })
    }

    fn defs() -> Vec<WindowDef> {
        ["First", "Second"]
            .into_iter()
            .map(|name| WindowDef {
                name,
                config: WindowConfig::default(),
                view: generated_view,
            })
            .collect()
    }

    fn runtime(theme: Option<&str>) -> Runtime {
        let component = generated_component(NAMES, None, &[]).expect("component");
        Runtime::new(
            defs(),
            component,
            Rc::new(RefCell::new(Vec::new())),
            theme,
            Rc::new(RefCell::new(theme.map(str::to_owned))),
            Vec::new(),
        )
    }

    fn assembled(
        theme: Option<&str>,
        seeds: &[(&str, Value)],
        script: Option<&script::Program>,
    ) -> (WidgetId, Ui) {
        let (component, built) =
            generated_built(generated_view, NAMES, theme, seeds, script).expect("launch succeeds");
        let id = built.widget.id();
        (
            id,
            Ui::assemble(component, built, generated_view, None, None),
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
        let program = script::Program {
            functions: Vec::new(),
            statements: vec![script::Statement::Let {
                constant: false,
                name: "name".to_owned(),
                value: Some(Expr::new(
                    ExprKind::String("from script".to_owned()),
                    Position::default(),
                )),
            }],
        };
        let (id, ui) = assembled(None, &[], Some(&program));
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
        assert_eq!(error, "window 7 is not declared");
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
            assert_eq!(ui.theme.as_deref(), Some("light"));
        }
    }

    thread_local! {
        static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn theme_text(component: &mut crate::Component) -> Result<Value, crate::Error> {
        let theme = component.call("theme", &[])?;
        let name = match &theme {
            Value::String(name) => name.to_string(),
            _ => "none".to_owned(),
        };
        SEEN.with(|seen| seen.borrow_mut().push(name));
        Ok(theme)
    }

    fn theme_view(_theme: Option<&str>, component: &ComponentHandle) -> Result<Built, String> {
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

    fn seeded_view(_theme: Option<&str>, component: &ComponentHandle) -> Result<Built, String> {
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

        runtime.apply(vec![Message::Set(
            "name".to_owned(),
            Signal::Text("live".to_owned()),
        )]);

        let tree = &mut runtime.uis[&WindowKey(0)].tree.borrow_mut();
        assert!(!tree.set_text(id, "live".to_owned()));
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

    fn key_view(component: &ComponentHandle, function: &str) -> Result<Built, String> {
        let widget = Widget::text("");
        Ok(Built {
            widget,
            bindings: Vec::new(),
            handlers: vec![Handler::new(component, None, HandlerEvent::Key, function)],
            props: Vec::new(),
        })
    }

    fn picker_view(_theme: Option<&str>, component: &ComponentHandle) -> Result<Built, String> {
        key_view(component, "handleKey")
    }

    fn settings_view(_theme: Option<&str>, component: &ComponentHandle) -> Result<Built, String> {
        key_view(component, "handleSettingsKey")
    }

    fn exit_program() -> script::Program {
        let exit = script::Statement::Expr(Expr::new(
            ExprKind::Call {
                target: Box::new(Expr::new(
                    ExprKind::Ident("exit".to_owned()),
                    Position::default(),
                )),
                args: Vec::new(),
            },
            Position::default(),
        ));
        script::Program {
            functions: ["handleKey", "handleSettingsKey"]
                .into_iter()
                .map(|name| script::Function {
                    name: name.to_owned(),
                    params: vec!["ev".to_owned()],
                    body: vec![exit.clone()],
                })
                .collect(),
            statements: Vec::new(),
        }
    }

    #[test]
    fn a_key_closes_only_the_window_that_received_it() {
        let names: &'static [&'static str] =
            &["handleKey", "handleSettingsKey", "exit", "ev", "code"];
        let program = exit_program();
        let component = generated_component(names, Some(&program), &[]).expect("component");
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
                code: "Escape".to_owned(),
                modifiers: brul_gui::KeyModifiers::default(),
            }],
        );

        assert_eq!(commands, vec![WindowCommand::Close]);
        assert!(runtime.uis.contains_key(&WindowKey(0)));
    }

    #[test]
    fn explicit_theme_ignores_the_color_scheme() {
        assert_eq!(scheme_theme(Some("dark"), ColorScheme::Light), "dark");
        assert_eq!(scheme_theme(Some("light"), ColorScheme::Dark), "light");
    }

    #[test]
    fn system_theme_follows_the_color_scheme() {
        assert_eq!(scheme_theme(None, ColorScheme::Light), "light");
        assert_eq!(scheme_theme(None, ColorScheme::Dark), "dark");
    }
}
