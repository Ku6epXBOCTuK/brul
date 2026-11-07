use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

pub(crate) enum Kind {
    Signal,
    Memo,
    Effect,
}

struct Node {
    kind: Kind,
    subscribers: Vec<u32>,
    dependencies: Vec<u32>,
    dirty: bool,
    queued: bool,
    disposed: bool,
    payload: Option<Rc<dyn Any>>,
}

#[derive(Default)]
struct Arena {
    nodes: Vec<Node>,
    queue: VecDeque<u32>,
}

thread_local! {
    static ARENA: RefCell<Arena> = RefCell::new(Arena::default());
    static TRACKERS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static UNTRACKED: Cell<usize> = const { Cell::new(0) };
}

pub(crate) fn create(kind: Kind, payload: Rc<dyn Any>) -> u32 {
    ARENA.with(|arena| {
        let mut arena = arena.borrow_mut();
        let id = arena.nodes.len() as u32;
        arena.nodes.push(Node {
            kind,
            subscribers: Vec::new(),
            dependencies: Vec::new(),
            dirty: false,
            queued: false,
            disposed: false,
            payload: Some(payload),
        });
        id
    })
}

pub(crate) fn payload<T: 'static>(id: u32) -> Rc<T> {
    let any = ARENA.with(|arena| {
        arena.borrow().nodes[id as usize]
            .payload
            .as_ref()
            .map(Rc::clone)
    });
    let any = any.expect("node payload missing");
    match any.downcast::<T>() {
        Ok(value) => value,
        Err(_) => panic!("node payload type"),
    }
}

pub(crate) fn track(source: u32) {
    if UNTRACKED.with(Cell::get) > 0 {
        return;
    }
    let tracker = TRACKERS.with(|trackers| trackers.borrow().last().copied());
    if let Some(tracker) = tracker {
        subscribe(source, tracker);
    }
}

fn subscribe(source: u32, tracker: u32) {
    ARENA.with(|arena| {
        let mut arena = arena.borrow_mut();
        let node = &mut arena.nodes[source as usize];
        if node.subscribers.contains(&tracker) {
            return;
        }
        node.subscribers.push(tracker);
        arena.nodes[tracker as usize].dependencies.push(source);
    });
}

pub(crate) fn with_tracker<R>(id: u32, f: impl FnOnce() -> R) -> R {
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            TRACKERS.with(|trackers| {
                trackers.borrow_mut().pop();
            });
        }
    }

    TRACKERS.with(|trackers| trackers.borrow_mut().push(id));
    let _pop = Pop;
    f()
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

pub(crate) fn notify(id: u32) {
    ARENA.with(|arena| {
        let mut arena = arena.borrow_mut();
        let mut stack = vec![id];
        while let Some(current) = stack.pop() {
            let subscribers = arena.nodes[current as usize].subscribers.clone();
            for subscriber in subscribers {
                let node = &mut arena.nodes[subscriber as usize];
                if node.disposed {
                    continue;
                }
                match node.kind {
                    Kind::Effect => {
                        if node.queued {
                            continue;
                        }
                        node.queued = true;
                        node.dirty = true;
                        arena.queue.push_back(subscriber);
                    }
                    Kind::Memo => {
                        if node.dirty {
                            continue;
                        }
                        node.dirty = true;
                        stack.push(subscriber);
                    }
                    Kind::Signal => {}
                }
            }
        }
    });
}

pub(crate) fn clear_dependencies(id: u32) {
    ARENA.with(|arena| {
        let mut arena = arena.borrow_mut();
        let dependencies = std::mem::take(&mut arena.nodes[id as usize].dependencies);
        for dependency in dependencies {
            arena.nodes[dependency as usize]
                .subscribers
                .retain(|subscriber| *subscriber != id);
        }
    });
}

pub(crate) fn is_dirty(id: u32) -> bool {
    ARENA.with(|arena| arena.borrow().nodes[id as usize].dirty)
}

pub(crate) fn set_dirty(id: u32, dirty: bool) {
    ARENA.with(|arena| arena.borrow_mut().nodes[id as usize].dirty = dirty);
}

pub(crate) fn take_dirty(id: u32) -> bool {
    ARENA.with(|arena| {
        let mut arena = arena.borrow_mut();
        let node = &mut arena.nodes[id as usize];
        std::mem::replace(&mut node.dirty, false)
    })
}

pub(crate) fn pop_queued() -> Option<u32> {
    ARENA.with(|arena| arena.borrow_mut().queue.pop_front())
}

pub(crate) fn clear_queued(id: u32) {
    ARENA.with(|arena| arena.borrow_mut().nodes[id as usize].queued = false);
}

pub(crate) fn disposed(id: u32) -> bool {
    ARENA.with(|arena| arena.borrow().nodes[id as usize].disposed)
}

pub(crate) fn run(id: u32) {
    if disposed(id) {
        return;
    }
    clear_dependencies(id);
    set_dirty(id, false);
    let callback = payload::<RefCell<Box<dyn Fn()>>>(id);
    let callback = callback.borrow();
    with_tracker(id, &**callback);
}

pub(crate) fn dispose(id: u32) {
    ARENA.with(|arena| {
        let mut arena = arena.borrow_mut();
        let node = &mut arena.nodes[id as usize];
        if node.disposed {
            return;
        }
        node.disposed = true;
        node.dirty = false;
        node.queued = false;
        node.payload = None;
        let dependencies = std::mem::take(&mut node.dependencies);
        for dependency in dependencies {
            arena.nodes[dependency as usize]
                .subscribers
                .retain(|subscriber| *subscriber != id);
        }
    });
}
