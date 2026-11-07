use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::rc::Rc;

use crate::graph::{self, Kind};
use crate::scope;

/// Побочный эффект: выполняется при создании и после каждой записи в
/// прочитанные сигналы — на ближайшем [`flush`].
///
/// Дескриптор можно отпустить: пока эффект не остановлен областью, он живёт в
/// арене и продолжает работу.
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
/// let source = count.clone();
/// let _effect = Effect::new(move || log.borrow_mut().push(source.get()));
/// assert_eq!(*seen.borrow(), vec![0]);
///
/// count.set(1);
/// assert_eq!(*seen.borrow(), vec![0]);
/// flush();
/// assert_eq!(*seen.borrow(), vec![0, 1]);
/// ```
pub struct Effect {
    id: u32,
    marker: PhantomData<*const ()>,
}

impl Clone for Effect {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            marker: PhantomData,
        }
    }
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

/// Выполняет запланированные эффекты. Вызывается циклом кадра между
/// обработкой событий и отрисовкой; внутри [`batch`] ничего не делает.
///
/// Запись из эффекта планирует ещё один проход того же вызова, а не
/// перезапускает текущий.
pub fn flush() {
    if BATCH_DEPTH.with(Cell::get) > 0 {
        return;
    }
    while let Some(id) = graph::pop_queued() {
        graph::clear_queued(id);
        if graph::disposed(id) {
            continue;
        }
        if graph::take_dirty(id) {
            graph::run(id);
        }
    }
}

/// Копит записи и запускает эффекты один раз в конце внешнего батча.
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
/// let source = count.clone();
/// let _effect = Effect::new(move || {
///     source.get();
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

/// Выполняет замыкание без подписки на прочитанные сигналы.
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
/// let watched = tracked.clone();
/// let ignored = hidden.clone();
/// let _effect = Effect::new(move || {
///     watched.get();
///     untrack(|| ignored.get());
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
