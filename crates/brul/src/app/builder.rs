use crate::App;
use crate::app::handle::AppHandle;
use crate::control::EventBus;
use crate::runtime::RuntimeManager;
use crate::state::StateManager;
use crate::window::WindowManager;
use brul_utils::Config;
use std::{any::TypeId, collections::HashMap, error::Error};

type SetupHookFn = dyn FnOnce(&mut App) -> () + 'static;

#[derive(Default)]
pub struct AppBuilder {
    config: Config,
    setup_hooks: Vec<Box<SetupHookFn>>,
    managed_states: HashMap<TypeId, Box<dyn Send + Sync>>,
}

impl AppBuilder {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn default() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    pub fn setup<F>(mut self, setup_fn: F) -> Self
    where
        F: FnOnce(&mut App) -> () + 'static,
    {
        self.setup_hooks.push(Box::new(setup_fn));
        self
    }

    pub fn manage<S>(mut self, state: S) -> Self
    where
        S: Send + Sync + 'static,
    {
        self.managed_states
            .insert(TypeId::of::<S>(), Box::new(state));
        self
    }

    pub fn add_task<T>(self, _task: T) -> Self {
        // TODO: add task to App when build
        self
    }

    pub fn add_listener<F, E>(self, _listener: Box<F>) -> Self
    where
        F: Fn(&App, E) + 'static,
        E: Send + Sync + 'static,
    {
        // TODO: add listener to App when build
        self
    }

    pub fn build(self) -> App {
        let runtime = RuntimeManager::new();

        let mut app = App {
            runtime,
            config: self.config,
            handle: AppHandle::new(),
            state: StateManager::new(),
            window: WindowManager::default(),
            event_bus: EventBus::new(),
        };

        for (_, state) in self.managed_states {
            app.state.set(state);
        }

        for hook in self.setup_hooks {
            hook(&mut app);
        }

        app
    }

    pub fn run(self) -> Result<(), Box<dyn Error>> {
        let app = self.build();
        app.run();
        Ok(())
    }
}
