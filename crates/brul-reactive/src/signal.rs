use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::graph::{self, Kind};

/// A reactive value cell.
///
/// Reads return a clone and writes wake subscribers: memos that read the
/// signal inside their computation are marked dirty and recomputed on the next
/// read.
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
    id: u32,
    not_send: PhantomData<*const ()>,
    value: PhantomData<fn() -> T>,
}

impl<T> Copy for Signal<T> {}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Signal<T> {
    pub fn new(value: T) -> Self {
        Self {
            id: graph::create(Kind::Signal, Rc::new(RefCell::new(value))),
            not_send: PhantomData,
            value: PhantomData,
        }
    }
}

impl<T: Clone + 'static> Signal<T> {
    pub fn get(&self) -> T {
        graph::track(self.id);
        let cell = graph::payload::<RefCell<T>>(self.id);
        cell.borrow().clone()
    }

    pub fn set(&self, value: T) {
        {
            let cell = graph::payload::<RefCell<T>>(self.id);
            *cell.borrow_mut() = value;
        }
        graph::notify(self.id);
    }
}

impl<T: 'static> Signal<T> {
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        {
            let cell = graph::payload::<RefCell<T>>(self.id);
            f(&mut cell.borrow_mut());
        }
        graph::notify(self.id);
    }
}
