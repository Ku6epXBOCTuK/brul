use std::{
    any::{Any, TypeId},
    collections::HashMap,
    error::Error,
    pin::Pin,
};

use brul_utils::Config;

use crate::App;

type SetupHookFn = dyn FnOnce(&App) -> () + 'static;
type SetupHookAsyncFn = Box<dyn FnOnce(&App) -> Pin<Box<dyn Future<Output = ()>>>>;

#[derive(Default)]
pub struct AppBuilder {
    config: Config,
    setup_hooks: Vec<Box<SetupHookFn>>,
    setup_hooks_async: Vec<SetupHookAsyncFn>,
    managed_states: HashMap<TypeId, Box<dyn Send + Sync>>,
}

impl AppBuilder {
    pub fn default() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn setup<F>(mut self, setup_fn: F) -> Self
    where
        F: FnOnce(&App) -> (),
    {
        self.setup_hooks.push(Box::new(setup_fn));
        self
    }

    pub fn setup_async<F>(mut self, setup_fn: F) -> Self
    where
        F: FnOnce(&App) -> Pin<Box<dyn Future<Output = ()>>>,
    {
        self.setup_hooks_async.push(Box::new(setup_fn));
        self
    }

    pub fn manage<S>(mut self, state: S) -> Self
    where
        S: Send + Sync + 'static,
    {
        self.state.insert(TypeId::of::<S>(), Box::new(state));
        self
    }

    pub fn add_task<T>(self, _task: T) -> Self {
        // TODO: add task to App when build
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
        todo!()
    }

    pub fn run(self) -> Result<(), Box<dyn Error>> {
        println!("Run!!!");
        let _a = "a".parse::<u32>()?;
        Ok(())
    }
}
