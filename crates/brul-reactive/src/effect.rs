use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::rc::Rc;

use crate::graph::{self, Kind};
use crate::scope;

/// A side effect: runs on creation and after every write to a signal it read,
/// at the next [`flush`].
///
/// The handle can be dropped: while the effect is not stopped by a scope, it
/// lives in the arena and keeps running.
///
/// ```
/// use std::cell::RefCell;
/// use std::rc::Rc;
///
/// use brul_reactive::{flush, Effect, Signal};
///
/// let count = Signal::new(0);
/// let seen = Rc::new(RefCell::new(Vec::new()));
/// let log = Rc::clone(&seen);
/// let _effect = Effect::new(move || log.borrow_mut().push(count.get()));
/// assert_eq!(*seen.borrow(), vec![0]);
///
/// count.set(1);
/// assert_eq!(*seen.borrow(), vec![0]);
/// flush();
/// assert_eq!(*seen.borrow(), vec![0, 1]);
/// ```
#[derive(Clone, Copy)]
pub struct Effect {
    #[expect(dead_code, reason = "arena index identifies the effect")]
    id: u32,
    marker: PhantomData<*const ()>,
}

thread_local! {
    static BATCH_DEPTH: Cell<usize> = const { Cell::new(0) };
}

impl Effect {
    pub fn new(callback: impl Fn() + 'static) -> Self {
        let callback: Box<dyn Fn()> = Box::new(callback);
        let id = graph::create(Kind::Effect, Rc::new(RefCell::new(callback)));
        scope::own(id);
        graph::run(id);
        Self {
            id,
            marker: PhantomData,
        }
    }
}

/// Runs scheduled effects. The frame loop calls it between event handling and
/// drawing; inside [`batch`] it does nothing.
///
/// A write from an effect schedules another pass of the same call rather than
/// restarting the current one.
///
/// ```
/// use std::cell::Cell;
/// use std::rc::Rc;
///
/// use brul_reactive::{flush, Effect, Signal};
///
/// let count = Signal::new(0);
/// let runs = Rc::new(Cell::new(0));
/// let seen = Rc::clone(&runs);
/// let _effect = Effect::new(move || {
///     count.get();
///     seen.set(seen.get() + 1);
/// });
/// assert_eq!(runs.get(), 1);
///
/// count.set(1);
/// assert_eq!(runs.get(), 1, "write only schedules");
/// flush();
/// assert_eq!(runs.get(), 2);
/// ```
pub fn flush() {
    if BATCH_DEPTH.with(Cell::get) > 0 {
        return;
    }
    while let Some(id) = graph::pop_queued() {
        graph::clear_queued(id);
        if graph::disposed(id) {
            continue;
        }
        // NOTE: a queue entry outlives its effect, and a reused slot may hold
        // a signal or memo now
        if !graph::is_effect(id) {
            continue;
        }
        if graph::take_dirty(id) {
            graph::run(id);
        }
    }
}

/// Collects writes and runs effects once at the end of the outer batch.
///
/// ```
/// use std::cell::Cell;
/// use std::rc::Rc;
///
/// use brul_reactive::{batch, Effect, Signal};
///
/// let count = Signal::new(0);
/// let runs = Rc::new(Cell::new(0));
/// let seen = Rc::clone(&runs);
/// let _effect = Effect::new(move || {
///     count.get();
///     seen.set(seen.get() + 1);
/// });
/// assert_eq!(runs.get(), 1);
///
/// batch(|| {
///     count.set(1);
///     count.set(2);
/// });
/// assert_eq!(runs.get(), 2);
/// ```
pub fn batch<R>(f: impl FnOnce() -> R) -> R {
    struct Depth;
    impl Drop for Depth {
        fn drop(&mut self) {
            BATCH_DEPTH.with(|depth| depth.set(depth.get() - 1));
        }
    }

    BATCH_DEPTH.with(|depth| depth.set(depth.get() + 1));
    let depth = Depth;
    let result = f();
    drop(depth);
    flush();
    result
}

/// Runs a closure without subscribing to the signals it reads.
///
/// ```
/// use std::cell::Cell;
/// use std::rc::Rc;
///
/// use brul_reactive::{flush, untrack, Effect, Signal};
///
/// let tracked = Signal::new(0);
/// let hidden = Signal::new(0);
/// let runs = Rc::new(Cell::new(0));
/// let seen = Rc::clone(&runs);
/// let _effect = Effect::new(move || {
///     tracked.get();
///     untrack(|| hidden.get());
///     seen.set(seen.get() + 1);
/// });
///
/// hidden.set(1);
/// flush();
/// assert_eq!(runs.get(), 1);
///
/// tracked.set(1);
/// flush();
/// assert_eq!(runs.get(), 2);
/// ```
pub fn untrack<R>(f: impl FnOnce() -> R) -> R {
    graph::untrack(f)
}
