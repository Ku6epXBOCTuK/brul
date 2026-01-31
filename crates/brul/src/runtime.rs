use brul_utils::AppControlMessage;
use std::sync::mpsc::{self, Sender};
use tokio::{
    runtime::{Handle, Runtime},
    task::JoinHandle,
};

pub(crate) struct RuntimeManager {
    handle: Handle,
    control_tx: Sender<AppControlMessage>,
}

impl RuntimeManager {
    pub(crate) fn new() -> Self {
        let (handle_tx, handle_rx) = mpsc::channel::<Handle>();
        let (control_tx, control_rx) = mpsc::channel::<AppControlMessage>();

        let backgound_thread = std::thread::spawn(move || {
            let runtime = Runtime::new().unwrap();
            let handle = runtime.handle().clone();

            handle_tx.send(handle).unwrap();

            runtime.block_on(async {
                while let Ok(message) = control_rx.recv() {
                    match message {
                        AppControlMessage::RequestShutdown => break,
                        _ => {}
                    }
                }
            })
        });

        let handle = handle_rx.recv().unwrap();

        Self { handle, control_tx }
    }

    pub(crate) fn handle(&self) -> &Handle {
        &self.handle
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

    pub(crate) fn shutdown(&self) {
        self.control_tx
            .send(AppControlMessage::RequestShutdown)
            .unwrap();
    }

    // pub(crate) fn send_control(&self, message: ControlMessage) {
    //     self.control_tx.send(message).unwrap();
    // }
}
