use std::sync::{Arc, Mutex, PoisonError};

use brul_window::{Waker, WindowKey};

use crate::Value;

pub enum Message {
    Set(String, Value),
    Call(String, Vec<Value>),
    Invoke(Box<dyn std::any::Any + Send>),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Set(name, value) => formatter
                .debug_tuple("Set")
                .field(name)
                .field(value)
                .finish(),
            Self::Call(name, arguments) => formatter
                .debug_tuple("Call")
                .field(name)
                .field(arguments)
                .finish(),
            Self::Invoke(_) => formatter.write_str("Invoke(..)"),
        }
    }
}

#[derive(Debug)]
pub struct ScopedMessage {
    pub(crate) window: WindowKey,
    pub(crate) message: Message,
}

/// A path from a background thread into the DSL: `Handle` is addressed to a
/// window, cloneable, outlives the thread and is safe after the event loop
/// exits.
#[derive(Clone)]
pub struct Handle {
    queue: Arc<Mutex<Vec<ScopedMessage>>>,
    waker: Waker,
    window: WindowKey,
}

impl Handle {
    pub(crate) fn new(
        queue: Arc<Mutex<Vec<ScopedMessage>>>,
        waker: Waker,
        window: WindowKey,
    ) -> Self {
        Self {
            queue,
            waker,
            window,
        }
    }

    pub fn window(&self, window: WindowKey) -> Self {
        Self {
            window,
            ..self.clone()
        }
    }

    pub fn set(&self, name: &str, value: impl Into<Value>) {
        self.push(Message::Set(name.to_owned(), value.into()));
    }

    pub fn call(&self, function: &str) {
        self.call_with(function, Vec::new());
    }

    /// Calls a script function with `arguments` and wakes the event loop.
    pub fn call_with(&self, function: &str, arguments: Vec<Value>) {
        self.push(Message::Call(function.to_owned(), arguments));
    }

    pub fn send<M: Into<Message> + Send + 'static>(&self, message: M) {
        self.push(message.into());
    }

    fn push(&self, message: Message) {
        self.queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(ScopedMessage {
                window: self.window,
                message,
            });
        self.waker.wake();
    }
}

pub(crate) fn take(queue: &Mutex<Vec<ScopedMessage>>) -> Vec<ScopedMessage> {
    let mut queue = queue.lock().unwrap_or_else(PoisonError::into_inner);
    std::mem::take(&mut queue)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use brul_window::WindowKey;

    use super::{Message, ScopedMessage, take};

    #[test]
    fn a_message_crosses_threads() {
        fn assert_send<T: Send>() {}
        assert_send::<ScopedMessage>();
    }

    #[test]
    fn taking_drains_the_queue() {
        let queue = Mutex::new(vec![ScopedMessage {
            window: WindowKey(1),
            message: Message::Call("refresh".to_owned(), Vec::new()),
        }]);

        let messages = take(&queue);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].window, WindowKey(1));
        assert!(
            matches!(&messages[0].message, Message::Call(name, arguments) if name == "refresh" && arguments.is_empty())
        );
        assert!(take(&queue).is_empty());
    }
}
