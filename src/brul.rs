use std::{
    any::{Any, TypeId},
    collections::HashMap,
    error::Error,
};

#[non_exhaustive]
#[derive(Default)]
pub struct Brul {
    state: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    tasks: Vec<Box<dyn FnOnce(&mut Brul) + Send + Sync + 'static>>,
}

impl Brul {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build() -> Self {
        Self::new()
    }

    pub fn state<T: Send + Sync + 'static>(mut self, state: T) -> Self {
        self.state.insert(TypeId::of::<T>(), Box::new(state));
        self
    }

    

    pub fn add_task<F>(mut self, process: F) -> Self
    where
        F: FnOnce(&mut Brul) + Send + Sync + 'static,
    {
        self.tasks.push(Box::new(process));
        self
    }

    pub fn run(self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
