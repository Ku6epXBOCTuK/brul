use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::graph::{self, Kind};

/// Производное значение: считается лениво и кэшируется между проходами.
///
/// ```
/// use brul_reactive::{Memo, Signal};
///
/// let count = Signal::new(1);
/// let doubled = Memo::new(move || count.get() * 2);
///
/// assert_eq!(doubled.get(), 2);
/// count.set(5);
/// assert_eq!(doubled.get(), 10);
/// ```
pub struct Memo<T> {
    id: u32,
    not_send: PhantomData<*const ()>,
    value: PhantomData<fn() -> T>,
}

impl<T> Copy for Memo<T> {}

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}

struct MemoCell<T> {
    compute: RefCell<Box<dyn Fn() -> T>>,
    value: RefCell<Option<T>>,
}

impl<T: Clone + 'static> Memo<T> {
    pub fn new(compute: impl Fn() -> T + 'static) -> Self {
        let memo = Self {
            id: graph::create(
                Kind::Memo,
                Rc::new(MemoCell {
                    compute: RefCell::new(Box::new(compute)),
                    value: RefCell::new(None),
                }),
            ),
            not_send: PhantomData,
            value: PhantomData,
        };
        graph::set_dirty(memo.id, true);
        memo
    }

    pub fn get(&self) -> T {
        self.refresh();
        let cell = graph::payload::<MemoCell<T>>(self.id);
        cell.value
            .borrow()
            .clone()
            .expect("memo computed after refresh")
    }

    fn refresh(&self) {
        graph::track(self.id);
        if !graph::is_dirty(self.id) {
            return;
        }
        graph::clear_dependencies(self.id);
        let cell = graph::payload::<MemoCell<T>>(self.id);
        let value = {
            let compute = cell.compute.borrow();
            graph::with_tracker(self.id, &**compute)
        };
        *cell.value.borrow_mut() = Some(value);
        graph::set_dirty(self.id, false);
    }
}
