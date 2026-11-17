use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use brul_reactive::{Memo, Signal};

struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn allocations() -> usize {
    ALLOCATIONS.load(Ordering::Relaxed)
}

#[test]
fn a_bump_allocates_nothing_once_the_buffers_are_warm() {
    let plain = Signal::new(0_u32);
    plain.set(1);
    let before = allocations();
    for value in 0..1000 {
        plain.set(value);
    }
    assert_eq!(allocations() - before, 0, "bump without subscribers");

    let signal = Signal::new(0_u32);
    let doubled = Memo::new(move || signal.get() * 2);
    assert_eq!(doubled.get(), 0);
    signal.set(1);
    assert_eq!(doubled.get(), 2);
    let before = allocations();
    for value in 0..1000 {
        signal.set(value);
    }
    assert_eq!(allocations() - before, 0, "bump with a memo");
    assert_eq!(doubled.get(), 1998);
}
