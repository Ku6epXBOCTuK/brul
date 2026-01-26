use crate::{State, runtime::RuntimeManager, state::StateManager, window::WindowManager};
use brul_utils::Config;

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
pub struct App {
    handle: AppHandle,
    runtime: RuntimeManager,
    state: StateManager,
    window: WindowManager,
    config: Config,
    event_bus: EventBus,
}

impl App {
    pub fn run(self) -> () {
        // TODO: Implement
        println!("App run");
    }
}

impl AppManager for App {
    fn app_handle(&self) -> &AppHandle {
        &self.handle
    }

    fn config(&self) -> &Config {
        todo!()
    }

    fn manage<T: Send + Sync + 'static>(&mut self, state: T) -> bool {
        self.state.set(state)
    }

    fn state<T: Send + Sync + 'static>(&self) -> State<'_, T> {
        self.state.get::<T>()
    }

    fn try_state<T: Send + Sync + 'static>(&self) -> Option<State<'_, T>> {
        self.state.try_get::<T>()
    }
}
