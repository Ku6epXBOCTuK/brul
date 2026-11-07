use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::track::{self, Node, Subs};

/// Производное значение: кэш, который пересчитывается при чтении после
/// изменения зависимостей.
///
/// Создание не считает: первое вычисление происходит при первом `get`.
///
/// ```
/// use brul_reactive::{Memo, Signal};
///
/// let count = Signal::new(1);
/// let doubled = Memo::new({
///     let count = count.clone();
///     move || count.get() * 2
/// });
///
/// assert_eq!(doubled.get(), 2);
/// count.set(5);
/// assert_eq!(doubled.get(), 10);
/// ```
pub struct Memo<T> {
    inner: Rc<Inner<T>>,
}

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

struct Inner<T> {
    id: u32,
    compute: RefCell<Box<dyn Fn() -> T>>,
    deps: RefCell<Vec<Rc<Subs>>>,
    subs: Rc<Subs>,
    value: RefCell<Option<T>>,
    dirty: Cell<bool>,
}

impl<T: Clone + 'static> Memo<T> {
    pub fn new(compute: impl Fn() -> T + 'static) -> Self {
        Self {
            inner: Rc::new(Inner {
                id: track::next_id(),
                compute: RefCell::new(Box::new(compute)),
                deps: RefCell::new(Vec::new()),
                subs: Subs::new(),
                value: RefCell::new(None),
                dirty: Cell::new(true),
            }),
        }
    }

    pub fn get(&self) -> T {
        self.inner.refresh();
        self.inner
            .value
            .borrow()
            .clone()
            .expect("memo computed after refresh")
    }
}

impl<T: Clone + 'static> Inner<T> {
    fn refresh(self: &Rc<Self>) {
        track::track(&self.subs);
        if !self.dirty.get() {
            return;
        }
        self.drop_dependencies();
        let value = {
            let compute = self.compute.borrow();
            let node: Rc<dyn Node> = Rc::clone(self) as Rc<dyn Node>;
            track::with(node, &**compute)
        };
        *self.value.borrow_mut() = Some(value);
        self.dirty.set(false);
    }

    fn drop_dependencies(&self) {
        for subs in self.deps.borrow_mut().drain(..) {
            subs.remove(self.id);
        }
    }
}

impl<T: Clone + 'static> Node for Inner<T> {
    fn id(&self) -> u32 {
        self.id
    }

    fn invalidate(&self) {
        if !self.dirty.replace(true) {
            self.subs.notify();
        }
    }

    fn depends_on(&self, subs: Rc<Subs>) {
        self.deps.borrow_mut().push(subs);
    }
}
