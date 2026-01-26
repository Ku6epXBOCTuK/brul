use tokio::{
    runtime::{Builder, Handle},
    task::JoinHandle,
};

pub(crate) struct RuntimeManager {
    handle: Handle,
}

impl RuntimeManager {
    pub(crate) fn new() -> Self {
        let runtime = Builder::new_current_thread().enable_all().build().unwrap();

        let handle = runtime.handle().clone();
        Self { handle }
    }

    pub(crate) fn spawn<F>(&self, spawn_fn: F) -> JoinHandle<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.handle.spawn(spawn_fn)
    }

    pub(crate) fn block_on<F>(&self, spawn_fn: F) -> ()
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.handle.block_on(spawn_fn)
    }
}
