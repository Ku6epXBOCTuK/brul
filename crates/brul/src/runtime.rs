use tokio::runtime::Runtime;

pub(crate) struct RuntimeManager {
    runtime: Runtime,
}

impl RuntimeManager {
    pub(crate) fn new() -> Self {
        Self {
            runtime: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap(),
        }
    }
}
