use std::sync::Arc;

use brul_utils::Color;
use tokio::runtime::Handle;

use crate::app::{AppInner, manager::AppManager};

#[derive(Clone)]
pub struct AppHandle {
    inner: Arc<AppInner>,
    runtime_hande: Handle,
}

impl AppHandle {
    pub fn new(inner: Arc<AppInner>, runtime_hande: Handle) -> Self {
        Self {
            inner,
            runtime_hande,
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
        todo!()
    }

    fn manage<T: Send + Sync + 'static>(&mut self, state: T) -> bool {
        todo!()
    }

    fn state<T: Send + Sync + 'static>(&self) -> crate::State<'_, T> {
        todo!()
    }

    fn try_state<T: Send + Sync + 'static>(&self) -> Option<crate::State<'_, T>> {
        todo!()
    }
}
