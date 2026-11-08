use std::cell::{Cell, RefCell};
use std::rc::Rc;

use brul_reactive::{Effect, EffectScope, Signal, flush};

fn counter() -> (Rc<Cell<u32>>, Rc<dyn Fn()>) {
    let calls = Rc::new(Cell::new(0));
    let bump = Rc::clone(&calls);
    (calls, Rc::new(move || bump.set(bump.get() + 1)))
}

#[test]
fn child_effects_stop_with_parent() {
    let parent = EffectScope::new();
    let child = parent.child();
    let source = Signal::new(0);
    let (runs, count) = counter();
    child.run(move || {
        Effect::new(move || {
            source.get();
            count();
        });
    });

    assert_eq!(runs.get(), 1);
    parent.dispose();
    source.set(1);
    flush();
    assert_eq!(runs.get(), 1, "child effect is stopped by parent");
}

#[test]
fn cleanups_run_once_on_dispose_and_drop() {
    let scope = EffectScope::new();
    let calls = Rc::new(Cell::new(0));
    let bump = Rc::clone(&calls);
    scope.on_cleanup(move || bump.set(bump.get() + 1));

    scope.dispose();
    scope.dispose();
    drop(scope);
    assert_eq!(calls.get(), 1);
}

#[test]
fn cleanup_registered_after_dispose_runs_immediately() {
    let scope = EffectScope::new();
    scope.dispose();
    let calls = Rc::new(Cell::new(0));
    let bump = Rc::clone(&calls);
    scope.on_cleanup(move || bump.set(bump.get() + 1));
    assert_eq!(calls.get(), 1);
}

#[test]
fn child_drop_stops_its_effects() {
    let parent = EffectScope::new();
    let source = Signal::new(0);
    let (runs, count) = counter();
    {
        let child = parent.child();
        child.run(move || {
            Effect::new(move || {
                source.get();
                count();
            });
        });
    }

    source.set(1);
    flush();
    assert_eq!(runs.get(), 1, "dropped child stops its effect");
}

#[test]
fn cleanups_run_in_reverse_order() {
    let scope = EffectScope::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    for step in ["first", "second"] {
        let log = Rc::clone(&log);
        scope.on_cleanup(move || log.borrow_mut().push(step));
    }

    scope.dispose();
    assert_eq!(*log.borrow(), vec!["second", "first"]);
}

#[test]
fn nested_children_stop_with_root() {
    let root = EffectScope::new();
    let middle = root.child();
    let leaf = middle.child();
    let calls = Rc::new(Cell::new(0));
    let bump = Rc::clone(&calls);
    leaf.on_cleanup(move || bump.set(bump.get() + 1));

    root.dispose();
    assert_eq!(calls.get(), 1);
}

#[test]
fn child_of_disposed_scope_runs_no_effects() {
    let parent = EffectScope::new();
    parent.dispose();
    let child = parent.child();
    let (runs, count) = counter();
    child.run(move || {
        Effect::new(move || count());
    });

    assert_eq!(runs.get(), 0, "effect is disposed before its first run");
}
