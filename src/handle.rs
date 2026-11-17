use std::sync::{Arc, Mutex, PoisonError};

use brul_window::Waker;

use crate::Value;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Message {
    Set(String, Value),
    Call(String, Vec<Value>),
}

/// Путь из фонового потока в DSL: `Handle` клонируется, переживает поток и
/// безопасен после выхода из цикла событий.
#[derive(Clone)]
pub struct Handle {
    queue: Arc<Mutex<Vec<Message>>>,
    waker: Waker,
}

impl Handle {
    pub(crate) fn new(queue: Arc<Mutex<Vec<Message>>>, waker: Waker) -> Self {
        Self { queue, waker }
    }

    pub fn set(&self, name: &str, value: impl Into<Value>) {
        self.push(Message::Set(name.to_owned(), value.into()));
    }

    pub fn call(&self, function: &str) {
        self.call_with(function, Vec::new());
    }

    /// Зовёт функцию скрипта с аргументами и будит цикл событий.
    pub fn call_with(&self, function: &str, arguments: Vec<Value>) {
        self.push(Message::Call(function.to_owned(), arguments));
    }

    fn push(&self, message: Message) {
        self.queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(message);
        self.waker.wake();
    }
}

pub(crate) fn take(queue: &Mutex<Vec<Message>>) -> Vec<Message> {
    let mut queue = queue.lock().unwrap_or_else(PoisonError::into_inner);
    std::mem::take(&mut queue)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::{Message, take};

    #[test]
    fn a_message_crosses_threads() {
        fn assert_send<T: Send>() {}
        assert_send::<Message>();
    }

    #[test]
    fn taking_drains_the_queue() {
        let queue = Mutex::new(vec![Message::Call("refresh".to_owned(), Vec::new())]);

        assert_eq!(
            take(&queue),
            vec![Message::Call("refresh".to_owned(), Vec::new())]
        );
        assert!(take(&queue).is_empty());
    }
}
