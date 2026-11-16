const COUNTING: &str = r#"#[cfg(brul_alloc_count)]
mod brul_allocations {
    use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
    static REALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
    static BYTES: AtomicUsize = AtomicUsize::new(0);
    static INITIAL_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
    static INITIAL_REALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
    static INITIAL_BYTES: AtomicUsize = AtomicUsize::new(0);

    struct Counting;

    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: AllocLayout) -> *mut u8 {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            unsafe { System.alloc(layout) }
        }

        unsafe fn realloc(&self, pointer: *mut u8, layout: AllocLayout, size: usize) -> *mut u8 {
            REALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(size, Ordering::Relaxed);
            unsafe { System.realloc(pointer, layout, size) }
        }

        unsafe fn dealloc(&self, pointer: *mut u8, layout: AllocLayout) {
            unsafe { System.dealloc(pointer, layout) }
        }
    }

    #[global_allocator]
    static ALLOCATOR: Counting = Counting;

    pub fn reset() {
        INITIAL_ALLOCATIONS.store(ALLOCATIONS.swap(0, Ordering::Relaxed), Ordering::Relaxed);
        INITIAL_REALLOCATIONS.store(REALLOCATIONS.swap(0, Ordering::Relaxed), Ordering::Relaxed);
        INITIAL_BYTES.store(BYTES.swap(0, Ordering::Relaxed), Ordering::Relaxed);
    }

    pub fn report() {
        brul::logging::info!(
            target: "brul",
            "brul allocations: init {} allocs, {} reallocs, {} bytes; ui {} allocs, {} reallocs, {} bytes",
            INITIAL_ALLOCATIONS.load(Ordering::Relaxed),
            INITIAL_REALLOCATIONS.load(Ordering::Relaxed),
            INITIAL_BYTES.load(Ordering::Relaxed),
            ALLOCATIONS.load(Ordering::Relaxed),
            REALLOCATIONS.load(Ordering::Relaxed),
            BYTES.load(Ordering::Relaxed),
        );
    }
}

"#;

const REGISTRATION: &str = r#"    #[cfg(brul_alloc_count)]
    brul::perf::measure(brul_allocations::reset, brul_allocations::report);
"#;

const INCLUDE: &str = "include!(concat!(env!(\"OUT_DIR\"), \"/allocations.rs\"));\n";

pub fn allocations() -> &'static str {
    COUNTING
}

pub(super) fn include(out: &mut String) {
    out.push_str(INCLUDE);
}

pub(super) fn registration(out: &mut String) {
    out.push_str(REGISTRATION);
}
