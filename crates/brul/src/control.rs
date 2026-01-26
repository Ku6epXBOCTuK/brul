#[derive(Default)]
pub struct EventBus {}

impl EventBus {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Default)]
pub struct CommandReceiver {}
