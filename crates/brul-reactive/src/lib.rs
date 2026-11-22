//! BRUL reactive core: signals, memos and effects.
//!
//! The graph is single-threaded and lives on the UI thread. Nodes are arena
//! indices, not `Rc` links, so subscription cycles are impossible.
//! A signal write marks downstream nodes dirty and queues effects; a dirty
//! memo recomputes on the next read. Effects run on [`flush`], at most once
//! per pass, and value versions are not compared. Disposed nodes give their
//! slots to the next ones, so rebuilds do not grow the arena.
//!
//! ```
//! use brul_reactive::{flush, Effect, Memo, Signal};
//!
//! let count = Signal::new(1);
//! let doubled = Memo::new(move || count.get() * 2);
//! let seen = std::rc::Rc::new(std::cell::Cell::new(0));
//! let runs = std::rc::Rc::clone(&seen);
//! let _effect = Effect::new(move || runs.set(doubled.get()));
//!
//! assert_eq!(seen.get(), 2);
//! count.set(5);
//! flush();
//! assert_eq!(seen.get(), 10);
//! ```

mod effect;
mod graph;
mod memo;
mod scope;
mod signal;

pub use effect::{Effect, batch, flush, untrack};
pub use memo::Memo;
pub use scope::EffectScope;
pub use signal::Signal;

/// Number of arena nodes that have not been removed, for test diagnostics.
/// Removed slots stay in the arena, so memory is not tracked by this counter
/// after [`EffectScope::dispose`].
pub fn live_nodes() -> usize {
    graph::live_nodes()
}
