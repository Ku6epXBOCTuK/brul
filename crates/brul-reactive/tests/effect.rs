use std::cell::{Cell, RefCell};
use std::rc::Rc;

use brul_reactive::{Effect, EffectScope, Memo, Signal, batch, flush, untrack};

fn counter() -> (Rc<Cell<u32>>, Rc<dyn Fn()>) {
    let calls = Rc::new(Cell::new(0));
    let bump = Rc::clone(&calls);
    (calls, Rc::new(move || bump.set(bump.get() + 1)))
}

#[test]
fn effect_runs_at_creation_and_waits_for_flush() {
    let source = Signal::new(1);
    let (runs, count) = counter();
    let _effect = Effect::new(move || {
        source.get();
        count();
    });

    assert_eq!(runs.get(), 1, "effect runs on creation");
    source.set(2);
    assert_eq!(runs.get(), 1, "write only schedules");
    flush();
    assert_eq!(runs.get(), 2);
    flush();
    assert_eq!(runs.get(), 2, "clean effect does not rerun");
}

#[test]
fn batch_runs_effects_once_at_the_end() {
    let source = Signal::new(0);
    let (runs, count) = counter();
    let _effect = Effect::new(move || {
        source.get();
        count();
    });

    batch(|| {
        source.set(1);
        source.set(2);
        assert_eq!(runs.get(), 1, "effects wait for the batch");
    });
    assert_eq!(runs.get(), 2);

    batch(|| {
        source.set(3);
        batch(|| source.set(4));
        assert_eq!(runs.get(), 2, "inner batch waits for the outer one");
    });
    assert_eq!(runs.get(), 3);
}

#[test]
fn untrack_reads_without_subscribing() {
    let tracked = Signal::new(0);
    let hidden = Signal::new(0);
    let (runs, count) = counter();
    let _effect = Effect::new(move || {
        tracked.get();
        untrack(|| hidden.get());
        count();
    });

    hidden.set(1);
    flush();
    assert_eq!(runs.get(), 1, "untracked read does not subscribe");

    tracked.set(1);
    flush();
    assert_eq!(runs.get(), 2);
}

#[test]
fn scope_dispose_stops_effects_and_subscriptions() {
    let scope = EffectScope::new();
    let source = Signal::new(0);
    let (runs, count) = counter();
    scope.run(move || {
        Effect::new(move || {
            source.get();
            count();
        });
    });

    assert_eq!(runs.get(), 1);
    source.set(1);
    flush();
    assert_eq!(runs.get(), 2);

    scope.dispose();
    source.set(2);
    flush();
    assert_eq!(runs.get(), 2, "disposed effect ignores writes");
}

#[test]
fn effect_follows_memo_chain() {
    let source = Signal::new(1);
    let doubled = Memo::new(move || source.get() * 2);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&seen);
    let _effect = Effect::new(move || log.borrow_mut().push(doubled.get()));

    assert_eq!(*seen.borrow(), vec![2]);
    source.set(3);
    flush();
    assert_eq!(*seen.borrow(), vec![2, 6]);
}

#[test]
fn effect_write_schedules_second_pass() {
    let source = Signal::new(0);
    let mirror = Signal::new(0);
    let (runs, count) = counter();
    let _effect = Effect::new(move || {
        let value = source.get();
        count();
        if mirror.get() != value {
            mirror.set(value);
        }
    });

    assert_eq!(runs.get(), 1);
    source.set(2);
    flush();
    assert_eq!(mirror.get(), 2);
    assert_eq!(runs.get(), 3, "write inside effect schedules one more pass");
}

#[test]
fn effect_drops_stale_dependencies() {
    let flag = Signal::new(true);
    let left = Signal::new(1);
    let right = Signal::new(10);
    let (runs, count) = counter();
    let _effect = Effect::new(move || {
        if flag.get() {
            left.get()
        } else {
            right.get()
        };
        count();
    });

    assert_eq!(runs.get(), 1);
    flag.set(false);
    flush();
    assert_eq!(runs.get(), 2);

    left.set(2);
    flush();
    assert_eq!(runs.get(), 2, "old dependency does not wake effect");

    right.set(11);
    flush();
    assert_eq!(runs.get(), 3);
}
