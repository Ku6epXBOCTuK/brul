use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::graph;

thread_local! {
    static SCOPES: RefCell<Vec<Rc<Inner>>> = const { RefCell::new(Vec::new()) };
}

/// Владелец очисток: эффекты, дочерние области и замыкания [`on_cleanup`]
/// останавливаются в [`dispose`](Self::dispose), при дропе и вместе с родителем.
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
/// let cleanups = Rc::new(Cell::new(0));
/// let stopped = Rc::clone(&cleanups);
/// scope.on_cleanup(move || stopped.set(stopped.get() + 1));
/// scope.run({
///     let runs = Rc::clone(&runs);
///     move || {
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
/// scope.dispose();
/// count.set(2);
/// flush();
/// assert_eq!(runs.get(), 2);
/// assert_eq!(cleanups.get(), 1);
/// ```
pub struct EffectScope {
    inner: Rc<Inner>,
}

struct Inner {
    effects: RefCell<Vec<u32>>,
    children: RefCell<Vec<Rc<Inner>>>,
    cleanups: RefCell<Vec<Box<dyn FnOnce()>>>,
    disposed: Cell<bool>,
}

impl Inner {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let effects: Vec<_> = self.effects.borrow_mut().drain(..).collect();
        for effect in effects {
            graph::dispose(effect);
        }
        let children: Vec<_> = self.children.borrow_mut().drain(..).collect();
        for child in children {
            child.dispose();
        }
        let cleanups: Vec<_> = self.cleanups.borrow_mut().drain(..).collect();
        for cleanup in cleanups.into_iter().rev() {
            cleanup();
        }
    }
}

impl EffectScope {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(Inner {
                effects: RefCell::new(Vec::new()),
                children: RefCell::new(Vec::new()),
                cleanups: RefCell::new(Vec::new()),
                disposed: Cell::new(false),
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

    /// Создаёт дочернюю область; её очистки выполняются вместе с этой.
    pub fn child(&self) -> Self {
        let child = EffectScope::new();
        if self.inner.disposed.get() {
            child.inner.dispose();
        } else {
            self.inner
                .children
                .borrow_mut()
                .push(Rc::clone(&child.inner));
        }
        child
    }

    /// Регистрирует очистку таймера, подписки или другого ресурса. Замыкания
    /// выполняются в обратном порядке регистрации; на уже освобождённой области
    /// очистка выполняется сразу.
    pub fn on_cleanup(&self, cleanup: impl FnOnce() + 'static) {
        if self.inner.disposed.get() {
            cleanup();
        } else {
            self.inner.cleanups.borrow_mut().push(Box::new(cleanup));
        }
    }

    /// Останавливает эффекты области и дочерних областей, выполняет очистки.
    /// Повторный вызов и дроп после него ничего не делают.
    pub fn dispose(&self) {
        self.inner.dispose();
    }
}

impl Drop for EffectScope {
    fn drop(&mut self) {
        self.inner.dispose();
    }
}

impl Default for EffectScope {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn own(id: u32) {
    SCOPES.with(|scopes| {
        let scope = scopes.borrow().last().cloned();
        if let Some(scope) = scope {
            if scope.disposed.get() {
                graph::dispose(id);
            } else {
                scope.effects.borrow_mut().push(id);
            }
        }
    });
}
