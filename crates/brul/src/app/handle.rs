use brul_utils::Color;

use crate::app::manager::AppManager;

#[derive(Clone)]
pub struct AppHandle {}

impl AppHandle {
    pub fn new() -> Self {
        Self {}
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
