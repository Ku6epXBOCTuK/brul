use std::cell::RefCell;
use std::rc::Rc;

use crate::track::{self, Subs};

/// Реактивная ячейка значения.
///
/// Чтение возвращает клон, запись будит подписчиков: memo, прочитавшие сигнал
/// внутри своего вычисления, помечаются грязными и пересчитываются при
/// следующем чтении.
///
/// ```
/// use brul_reactive::Signal;
///
/// let count = Signal::new(1);
/// assert_eq!(count.get(), 1);
/// count.set(2);
/// count.update(|value| *value += 1);
/// assert_eq!(count.get(), 3);
/// ```
pub struct Signal<T> {
    value: Rc<RefCell<T>>,
    subs: Rc<Subs>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            value: Rc::clone(&self.value),
            subs: Rc::clone(&self.subs),
        }
    }
}

impl<T> Signal<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
            subs: Subs::new(),
        }
    }
}

impl<T: Clone> Signal<T> {
    pub fn get(&self) -> T {
        track::track(&self.subs);
        self.value.borrow().clone()
    }

    pub fn set(&self, value: T) {
        *self.value.borrow_mut() = value;
        self.subs.notify();
    }
}

impl<T> Signal<T> {
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut self.value.borrow_mut());
        self.subs.notify();
    }
}
