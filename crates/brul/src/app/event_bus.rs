use std::{
    collections::HashMap,
    panic::{RefUnwindSafe, UnwindSafe},
    sync::{Arc, RwLock, atomic::AtomicU64},
};
use strum::{EnumDiscriminants, EnumMessage};

#[derive(Debug, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(EnumMessage, Hash))]
pub enum Event {
    AppStarted,
    AppShutdown,
}

#[derive(Clone)]
struct Handler {
    id: u64,
    callback: Arc<dyn Fn(&Event) + UnwindSafe + RefUnwindSafe + Send + Sync + 'static>,
}

pub struct EventBus {
    handlers: Arc<RwLock<HashMap<EventDiscriminants, Vec<Handler>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn emit(&self, event: &Event) {
        // let handlers = self.handlers.read().unwrap();
        let discriminant = EventDiscriminants::from(event);

        let handlers = {
            let handlers_map = self.handlers.read().unwrap();
            handlers_map
                .get(&discriminant)
                .cloned()
                .unwrap_or_else(Vec::new)
        };

        for handler in handlers {
            if let Err(err) = std::panic::catch_unwind(|| (handler.callback)(event)) {
                // TODO: log error
                eprintln!("Error in event handler: {:?}", err)
            };
        }
    }

    pub fn subscribe<F>(&self, event: EventDiscriminants, callback: F) -> u64
    where
        F: Fn(&Event) + UnwindSafe + RefUnwindSafe + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().unwrap();
        let id = generate_id();
        handlers.entry(event).or_insert(Vec::new()).push(Handler {
            id,
            callback: Arc::new(callback),
        });
        id
    }

    pub fn unsubscribe(&self, event: EventDiscriminants, id: u64) {
        let mut handlers = self.handlers.write().unwrap();
        if let Some(handlers) = handlers.get_mut(&event) {
            handlers.retain(|handler| handler.id != id);
        }
    }
}

fn generate_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
