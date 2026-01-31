use crate::{State, runtime::RuntimeManager, state::StateManager, window::WindowManager};
use brul_utils::{AppControlMessage, Config, EVProxy, GuiControlMessage, Result};
use std::sync::{Arc, mpsc};

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
    event_loop_proxy: EVProxy,
    runtime: RuntimeManager,
    tasks: Vec<Box<dyn Fn(&AppHandle) -> () + Send + 'static>>,
    inner: Arc<AppInner>,
}

impl App {
    pub fn run(mut self) -> Result<()> {
        tracing::info!("App run");

        let (tx, rx) = mpsc::channel::<AppControlMessage>();
        tx.send(AppControlMessage::AppStarted).unwrap();

        let gui_backend = brul_gui::GuiBackend::new(tx.clone())?;
        let event_loop_proxy = gui_backend.get_proxy();
        self.event_loop_proxy.set_proxy(event_loop_proxy.clone());

        // TODO: do i need tasks later, or i can give ownership?
        let tasks = std::mem::take(&mut self.tasks);
        let app_handle = self.app_handle().clone();
        let tasks: Vec<Box<dyn Fn() -> () + Send + 'static>> = tasks
            .into_iter()
            .map(|task| {
                let app_handle = app_handle.clone();
                let task_fn = Box::new(move || task(&app_handle));
                task_fn as Box<dyn Fn() + Send>
            })
            .collect();

        for task in tasks {
            self.runtime.spawn(async move {
                task();
            });
        }

        let handle = self.runtime.spawn(async move {
            tracing::info!("Event receiver start");
            while let Ok(event) = rx.recv() {
                match event {
                    AppControlMessage::RequestShutdown => {
                        tracing::info!("Received shutdown event");
                        break;
                    }
                    AppControlMessage::AppStarted => {
                        tracing::info!("Received app started event");
                    }
                    _ => {
                        tracing::info!("Received unknown event");
                    }
                }
            }
            // TODO: send shotdown to gui backend
            let result = event_loop_proxy.send_event(GuiControlMessage::Shutdown);

            tracing::debug!("Try send shutdown event: {:?}", result);
            tracing::info!("Event loop ended");
        });

        tracing::info!("Try run gui eventloop");
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
