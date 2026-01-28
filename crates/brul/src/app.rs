use std::sync::Arc;

use crate::{State, runtime::RuntimeManager, state::StateManager, window::WindowManager};
use brul_utils::{Config, Result};

mod builder;
mod core;
mod event_bus;
mod handle;
mod manager;

pub use builder::AppBuilder;
pub use event_bus::EventBus;
pub use handle::AppHandle;
pub use manager::AppManager;

#[non_exhaustive]
pub struct AppInner {
    state: StateManager,
    window: WindowManager,
    config: Config,
    event_bus: EventBus,
}

#[non_exhaustive]
pub struct App {
    handle: AppHandle,
    runtime: RuntimeManager,
    tasks: Vec<Box<dyn Fn(&AppHandle) -> () + 'static>>,
    inner: Arc<AppInner>,
}

impl App {
    pub fn run(mut self) -> Result<()> {
        tracing::info!("App run");
        tracing::info!("Try run gui eventloop");

        let runtime_handle = self.runtime.handle().clone();

        // TODO: do i need tasks later, or i can give ownership?
        let tasks = std::mem::take(&mut self.tasks);
        let app_handle = self.app_handle().clone();
        let tasks: Vec<Box<dyn Fn() -> () + 'static>> = tasks
            .into_iter()
            .map(|task| {
                // TODO: how convert task(app_handle) to task() ?
                // let task = *task;
                let app_handle = app_handle.clone();
                let task_fn = Box::new(move || task(&app_handle));
                task_fn as Box<dyn Fn()>
            })
            .collect();

        let gui_backend = brul_gui::GuiBackend::new(runtime_handle, tasks);
        gui_backend.run()?;

        tracing::info!("App ended ok");
        Ok(())
    }
}

impl AppManager for App {
    fn app_handle(&self) -> &AppHandle {
        &self.handle
    }

    fn config(&self) -> &Config {
        &self.inner.config
    }

    fn manage<T: Send + Sync + 'static>(&mut self, state: T) -> bool {
        self.inner.state.set(state)
    }

    fn state<T: Send + Sync + 'static>(&self) -> State<'_, T> {
        self.inner.state.get::<T>()
    }

    fn try_state<T: Send + Sync + 'static>(&self) -> Option<State<'_, T>> {
        self.inner.state.try_get::<T>()
    }

    fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.runtime.spawn(future)
    }
}
