use crate::app::EventBus;
use crate::app::handle::AppHandle;
use crate::runtime::RuntimeManager;
use crate::state::StateManager;
use crate::window::WindowManager;
use crate::{App, app::AppInner};
use brul_utils::{Config, Result};
use std::sync::Arc;
use std::{any::TypeId, collections::HashMap};

type SetupHookFn = dyn FnOnce(&mut App) -> () + 'static;

#[derive(Default)]
pub struct AppBuilder {
    config: Config,
    setup_hooks: Vec<Box<SetupHookFn>>,
    managed_states: HashMap<TypeId, Box<dyn Send + Sync>>,
    tasks: Vec<Box<dyn Fn(&AppHandle) -> () + 'static>>,
}

impl AppBuilder {
    pub fn new() -> Self {
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

    pub fn add_task<F>(mut self, task: F) -> Self
    where
        F: Fn(&AppHandle) -> () + 'static,
    {
        self.tasks.push(Box::new(task));
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
        tracing::info!("Building app");
        let runtime = RuntimeManager::new();

        let inner = Arc::new(AppInner {
            config: self.config,
            state: StateManager::new(),
            window: WindowManager::default(),
            event_bus: EventBus::new(),
        });

        let handle = AppHandle::new(Arc::clone(&inner), runtime.handle().clone());

        let tasks = self.tasks;

        let mut app = App {
            runtime,
            handle,
            tasks,
            inner: inner,
        };

        for (_, state) in self.managed_states {
            app.inner.state.set(state);
        }

        for hook in self.setup_hooks {
            hook(&mut app);
        }

        app
    }

    pub fn run(self) -> Result<()> {
        let app = self.build();
        tracing::info!("Running app");
        let result = app.run();
        tracing::debug!("App finished with result: {:?}", result);
        Ok(())
    }
}
