use std::sync::{Arc, Mutex, PoisonError};

use brul_window::Waker;

use crate::Value;

/// Значение, которое фоновый поток может передать в DSL.
#[derive(Clone, Debug, PartialEq)]
pub enum Signal {
    Text(String),
    Number(f64),
    Flag(bool),
}

impl From<&str> for Signal {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

impl From<String> for Signal {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<f64> for Signal {
    fn from(number: f64) -> Self {
        Self::Number(number)
    }
}

impl From<bool> for Signal {
    fn from(flag: bool) -> Self {
        Self::Flag(flag)
    }
}

impl Signal {
    pub(crate) fn value(self) -> Value {
        match self {
            Self::Text(text) => Value::String(text.into()),
            Self::Number(number) => Value::Number(number),
            Self::Flag(flag) => Value::Bool(flag),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Message {
    Set(String, Signal),
    Call(String),
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

    /// Пишет переменную DSL и будит цикл событий.
    pub fn set(&self, name: &str, signal: impl Into<Signal>) {
        self.push(Message::Set(name.to_owned(), signal.into()));
    }

    /// Зовёт функцию скрипта верхнего уровня и будит цикл событий.
    pub fn call(&self, function: &str) {
        self.push(Message::Call(function.to_owned()));
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

    use super::{Message, Signal, take};
    use crate::Value;

    #[test]
    fn a_signal_becomes_a_value() {
        assert_eq!(Signal::from("text").value(), Value::String("text".into()));
        assert_eq!(Signal::from(1.5_f64).value(), Value::Number(1.5));
        assert_eq!(Signal::from(true).value(), Value::Bool(true));
    }

    #[test]
    fn taking_drains_the_queue() {
        let queue = Mutex::new(vec![Message::Call("refresh".to_owned())]);

        assert_eq!(take(&queue), vec![Message::Call("refresh".to_owned())]);
        assert!(take(&queue).is_empty());
    }
}
