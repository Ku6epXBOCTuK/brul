use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

pub(crate) trait Node {
    fn id(&self) -> u32;
    fn invalidate(&self);
    fn depends_on(&self, subs: Rc<Subs>);
}

pub(crate) struct Subs(RefCell<Vec<(u32, Rc<dyn Node>)>>);

impl Subs {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self(RefCell::new(Vec::new())))
    }

    fn add(&self, node: &Rc<dyn Node>) {
        let id = node.id();
        let mut subs = self.0.borrow_mut();
        if !subs.iter().any(|(known, _)| *known == id) {
            subs.push((id, Rc::clone(node)));
        }
    }

    pub(crate) fn remove(&self, id: u32) {
        self.0.borrow_mut().retain(|(known, _)| *known != id);
    }

    pub(crate) fn notify(&self) {
        for (_, node) in self.0.borrow().iter() {
            node.invalidate();
        }
    }
}

thread_local! {
    static TRACKERS: RefCell<Vec<Rc<dyn Node>>> = const { RefCell::new(Vec::new()) };
    static UNTRACKED: Cell<usize> = const { Cell::new(0) };
}

pub(crate) fn next_id() -> u32 {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

pub(crate) fn track(subs: &Rc<Subs>) {
    if UNTRACKED.with(Cell::get) > 0 {
        return;
    }
    TRACKERS.with(|trackers| {
        if let Some(node) = trackers.borrow().last() {
            subs.add(node);
            node.depends_on(Rc::clone(subs));
        }
    });
}

pub(crate) fn untrack<R>(f: impl FnOnce() -> R) -> R {
    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            UNTRACKED.with(|depth| depth.set(depth.get() - 1));
        }
    }

    UNTRACKED.with(|depth| depth.set(depth.get() + 1));
    let _restore = Restore;
    f()
}

pub(crate) fn with<R>(node: Rc<dyn Node>, f: impl FnOnce() -> R) -> R {
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            TRACKERS.with(|trackers| {
                trackers.borrow_mut().pop();
            });
        }
    }

    TRACKERS.with(|trackers| trackers.borrow_mut().push(node));
    let _pop = Pop;
    f()
}
