use std::{
    collections::HashMap,
    mem::Discriminant,
    sync::{Arc, RwLock, atomic::AtomicU64},
};

#[derive(Debug, Clone)]
pub enum Event {
    AppStarted,
    AppShutdown,
}

type EventDiscriminant = Discriminant<Event>;

struct Handler {
    id: u64,
    callback: Box<dyn Fn(&Event) + Send + Sync + 'static>,
}

pub struct EventBus {
    handlers: Arc<RwLock<HashMap<EventDiscriminant, Vec<Handler>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn subscribe<F>(&self, event: Discriminant<Event>, callback: F) -> u64
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().unwrap();
        let id = generate_id();
        handlers.entry(event).or_insert(Vec::new()).push(Handler {
            id,
            callback: Box::new(callback),
        });
        id
    }

    pub fn unsubscribe(&self, event: Discriminant<Event>, id: u64) {
        let mut handlers = self.handlers.write().unwrap();
        if let Some(handlers) = handlers.get_mut(&event) {
            handlers.retain(|handler| handler.id != id);
        }
    }
}

fn generate_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}
