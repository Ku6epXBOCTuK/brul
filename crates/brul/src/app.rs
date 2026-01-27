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
    inner: Arc<AppInner>,
}

impl App {
    pub fn run(self) -> Result<()> {
        // TODO: Implement
        tracing::info!("App run");
        tracing::info!("Try run gui eventloop");

        let gui_backend = brul_gui::GuiBackend::new();
        gui_backend.run()?;
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
