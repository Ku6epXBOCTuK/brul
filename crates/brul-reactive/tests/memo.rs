use std::cell::Cell;
use std::rc::Rc;

use brul_reactive::{Memo, Signal};

fn counter() -> (Rc<Cell<u32>>, Rc<dyn Fn()>) {
    let calls = Rc::new(Cell::new(0));
    let bump = Rc::clone(&calls);
    (calls, Rc::new(move || bump.set(bump.get() + 1)))
}

#[test]
fn memo_is_lazy_and_cached() {
    let (calls, count) = counter();
    let source = Signal::new(2);
    let doubled = Memo::new({
        let source = source.clone();
        let count = Rc::clone(&count);
        move || {
            count();
            source.get() * 2
        }
    });

    assert_eq!(calls.get(), 0, "creation does not compute");
    assert_eq!(doubled.get(), 4);
    assert_eq!(calls.get(), 1);
    assert_eq!(doubled.get(), 4);
    assert_eq!(calls.get(), 1, "clean memo returns cache");

    source.set(3);
    assert_eq!(calls.get(), 1, "write does not compute");
    assert_eq!(doubled.get(), 6);
    assert_eq!(calls.get(), 2);
}

#[test]
fn chain_recomputes_each_node_once() {
    let source = Signal::new(1);
    let (inner_calls, inner_count) = counter();
    let (outer_calls, outer_count) = counter();

    let inner = Memo::new({
        let source = source.clone();
        move || {
            inner_count();
            source.get() + 1
        }
    });
    let outer = Memo::new({
        let inner = inner.clone();
        move || {
            outer_count();
            inner.get() * 10
        }
    });

    assert_eq!(outer.get(), 20);
    assert_eq!((inner_calls.get(), outer_calls.get()), (1, 1));

    source.set(2);
    assert_eq!(outer.get(), 30);
    assert_eq!((inner_calls.get(), outer_calls.get()), (2, 2));
}

#[test]
fn dependency_switch_drops_old_source() {
    let (calls, count) = counter();
    let flag = Signal::new(true);
    let left = Signal::new(1);
    let right = Signal::new(10);
    let picked = Memo::new({
        let (flag, left, right) = (flag.clone(), left.clone(), right.clone());
        move || {
            count();
            if flag.get() { left.get() } else { right.get() }
        }
    });

    assert_eq!(picked.get(), 1);
    assert_eq!(calls.get(), 1);

    flag.set(false);
    assert_eq!(picked.get(), 10);
    assert_eq!(calls.get(), 2);

    left.set(2);
    assert_eq!(picked.get(), 10);
    assert_eq!(calls.get(), 2, "old dependency no longer wakes memo");

    right.set(11);
    assert_eq!(picked.get(), 11);
    assert_eq!(calls.get(), 3);
}

#[test]
fn repeated_read_subscribes_once() {
    let (calls, count) = counter();
    let source = Signal::new(1);
    let sum = Memo::new({
        let source = source.clone();
        move || {
            count();
            source.get() + source.get()
        }
    });

    assert_eq!(sum.get(), 2);
    source.set(2);
    assert_eq!(sum.get(), 4);
    assert_eq!(calls.get(), 2, "repeated read does not duplicate recompute");
}
