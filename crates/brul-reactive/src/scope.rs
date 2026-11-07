use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static SCOPES: RefCell<Vec<Rc<Inner>>> = const { RefCell::new(Vec::new()) };
}

/// Владелец эффектов: всё, что создано внутри [`run`](Self::run), останавливается
/// и отписывается в [`dispose`](Self::dispose).
///
/// ```
/// use std::cell::Cell;
/// use std::rc::Rc;
///
/// use brul_reactive::{flush, Effect, EffectScope, Signal};
///
/// let scope = EffectScope::new();
/// let count = Signal::new(0);
/// let runs = Rc::new(Cell::new(0));
/// scope.run({
///     let count = count.clone();
///     let runs = Rc::clone(&runs);
///     move || {
///         let count = count.clone();
///         let runs = Rc::clone(&runs);
///         Effect::new(move || {
///             count.get();
///             runs.set(runs.get() + 1);
///         });
///     }
/// });
/// assert_eq!(runs.get(), 1);
///
/// count.set(1);
/// flush();
/// assert_eq!(runs.get(), 2);
///
/// scope.dispose();
/// count.set(2);
/// flush();
/// assert_eq!(runs.get(), 2);
/// ```
pub struct EffectScope {
    inner: Rc<Inner>,
}

struct Inner {
    cleanups: RefCell<Vec<Box<dyn Fn()>>>,
}

impl EffectScope {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(Inner {
                cleanups: RefCell::new(Vec::new()),
            }),
        }
    }

    /// Выполняет замыкание, привязывая созданные в нём эффекты к области.
    pub fn run<R>(&self, f: impl FnOnce() -> R) -> R {
        struct Pop;
        impl Drop for Pop {
            fn drop(&mut self) {
                SCOPES.with(|scopes| {
                    scopes.borrow_mut().pop();
                });
            }
        }

        SCOPES.with(|scopes| scopes.borrow_mut().push(Rc::clone(&self.inner)));
        let _pop = Pop;
        f()
    }

    /// Останавливает эффекты области и снимает их подписки.
    pub fn dispose(self) {
        let cleanups: Vec<_> = self.inner.cleanups.borrow_mut().drain(..).collect();
        for cleanup in cleanups {
            cleanup();
        }
    }
}

impl Default for EffectScope {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn own(cleanup: Box<dyn Fn()>) {
    SCOPES.with(|scopes| {
        if let Some(scope) = scopes.borrow().last() {
            scope.cleanups.borrow_mut().push(cleanup);
        }
    });
}
