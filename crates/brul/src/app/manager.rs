use crate::{State, app::handle::AppHandle};
use brul_utils::Config;
use tokio::task::JoinHandle;

pub trait AppManager {
    fn app_handle(&self) -> &AppHandle;

    fn config(&self) -> &Config; // is env are configs?

    // fn env(&self) -> &Env; // TODO: path, arguments, etc

    fn manage<T: Send + Sync + 'static>(&mut self, state: T) -> bool;

    fn state<T: Send + Sync + 'static>(&self) -> State<'_, T>;

    fn try_state<T: Send + Sync + 'static>(&self) -> Option<State<'_, T>>;

    fn spawn<F>(&self, future: F) -> JoinHandle<()>
    where
        F: Future<Output = ()> + Send + 'static;
}
