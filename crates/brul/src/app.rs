use crate::{
    AppHandle, Manager,
    control::{CommandReceiver, EventBus},
    runtime::RuntimeManager,
    state::StateManager,
    window::WindowManager,
};
use brul_utils::Config;

mod builder;
mod handle;
mod manager;

#[non_exhaustive]
pub struct App {
    handle: AppHandle,
    setup: Option<SetupHook>,
    runtime: RuntimeManager,
    state: StateManager,
    window: WindowManager,
    config: Config,
    event_bus: EventBus,
}

impl App {
    pub fn new() -> Self {
        Self {
            setup: None,
            runtime: RuntimeManager::new(),
            state: StateManager::new(),
            window: WindowManager::default(),
        }
    }
}

impl Manager for App {
    fn app_handle(&self) -> &AppHandle {
        &self.handle
    }

    fn config(&self) -> &brul_utils::Config {
        todo!()
    }

    fn manage<T: Send + Sync + 'static>(&mut self, state: T) -> bool {
        todo!()
    }

    fn state<T: Send + Sync + 'static>(&self) -> crate::State<T> {
        todo!()
    }

    fn try_state<T: Send + Sync + 'static>(&self) -> Option<crate::State<T>> {
        todo!()
    }
}
