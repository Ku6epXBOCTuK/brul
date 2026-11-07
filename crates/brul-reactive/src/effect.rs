use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use crate::scope;
use crate::track::{self, Node, Subs};

/// Побочный эффект: выполняется при создании и после каждой записи в
/// прочитанные сигналы — на ближайшем [`flush`].
///
/// Дескриптор можно отпустить: пока эффект не остановлен областью или
/// программой, он живёт в реестре и продолжает работу.
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
    inner: Rc<Inner>,
}

impl Clone for Effect {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

struct Inner {
    id: u32,
    callback: RefCell<Box<dyn Fn()>>,
    deps: RefCell<Vec<Rc<Subs>>>,
    dirty: Cell<bool>,
    queued: Cell<bool>,
    disposed: Cell<bool>,
}

thread_local! {
    static QUEUE: RefCell<VecDeque<u32>> = const { RefCell::new(VecDeque::new()) };
    static REGISTRY: RefCell<HashMap<u32, Rc<Inner>>> = RefCell::new(HashMap::new());
    static BATCH_DEPTH: Cell<usize> = const { Cell::new(0) };
}

impl Effect {
    pub fn new(callback: impl Fn() + 'static) -> Self {
        let inner = Rc::new(Inner {
            id: track::next_id(),
            callback: RefCell::new(Box::new(callback)),
            deps: RefCell::new(Vec::new()),
            dirty: Cell::new(false),
            queued: Cell::new(false),
            disposed: Cell::new(false),
        });
        REGISTRY.with(|registry| registry.borrow_mut().insert(inner.id, Rc::clone(&inner)));
        scope::own({
            let inner = Rc::clone(&inner);
            Box::new(move || inner.dispose())
        });
        inner.run();
        Self { inner }
    }
}

impl Inner {
    fn run(self: &Rc<Self>) {
        if self.disposed.get() {
            return;
        }
        self.drop_dependencies();
        self.dirty.set(false);
        let callback = self.callback.borrow();
        let node: Rc<dyn Node> = Rc::clone(self) as Rc<dyn Node>;
        track::with(node, &**callback);
    }

    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.drop_dependencies();
        REGISTRY.with(|registry| registry.borrow_mut().remove(&self.id));
    }

    fn drop_dependencies(&self) {
        for subs in self.deps.borrow_mut().drain(..) {
            subs.remove(self.id);
        }
    }
}

impl Node for Inner {
    fn id(&self) -> u32 {
        self.id
    }

    fn invalidate(&self) {
        if self.disposed.get() {
            return;
        }
        self.dirty.set(true);
        if self.queued.replace(true) {
            return;
        }
        QUEUE.with(|queue| queue.borrow_mut().push_back(self.id));
    }

    fn depends_on(&self, subs: Rc<Subs>) {
        self.deps.borrow_mut().push(subs);
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
    while let Some(id) = QUEUE.with(|queue| queue.borrow_mut().pop_front()) {
        let effect = REGISTRY.with(|registry| registry.borrow().get(&id).cloned());
        let Some(effect) = effect else {
            continue;
        };
        effect.queued.set(false);
        if effect.dirty.replace(false) {
            effect.run();
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
    track::untrack(f)
}
