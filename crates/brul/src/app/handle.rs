use std::sync::Arc;

use brul_utils::Color;
use tokio::runtime::Handle;

use crate::{
    State,
    app::{AppInner, manager::AppManager},
};

#[derive(Clone)]
pub struct AppHandle {
    inner: Arc<AppInner>,
    runtime_handle: Handle,
}

impl AppHandle {
    pub fn new(inner: Arc<AppInner>, runtime_handle: Handle) -> Self {
        Self {
            inner,
            runtime_handle,
        }
    }

    pub fn set_background_color(&self, _color: Color) {
        // TODO:
        todo!()
    }
}

impl AppManager for AppHandle {
    fn app_handle(&self) -> &AppHandle {
        self
    }

    fn config(&self) -> &brul_utils::Config {
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
        self.runtime_handle.spawn(future)
    }
}
