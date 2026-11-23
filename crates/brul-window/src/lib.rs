//! BRUL window: winit, wgpu and the event loop.
//!
//! [`run`] opens an empty window; [`run_tree`] and [`run_tree_themed`] open a
//! window with a widget tree: the tree is laid out, the snapshot is drawn into
//! a CPU frame and uploaded to the wgpu surface as one texture. Only dirty
//! regions change; resize and DPI redraw the whole frame. [`run_windows`]
//! keeps several windows in one loop, keyed by [`WindowKey`].
//!
//! Config is in logical pixels, the surface in physical ones; scale comes from
//! the window. Header height defines the drag area. Esc and window close
//! close the window, and the last window ends the loop. The window is created
//! hidden and shown after the first frame.
//!
//! ```no_run
//! use brul_window::{WindowConfig, run};
//!
//! run(WindowConfig::default()).expect("window");
//! ```

mod gpu;
mod gpu_frame;
mod input;
#[cfg(windows)]
mod layered;
pub mod perf;
mod platform;
mod software;
mod surface;
mod tree;
mod window;
mod windows;

pub use platform::{
    NoopPlatformBackendFactory, PlatformBackendFactory, PlatformMessageHandler, PlatformSpec,
    PlatformViewBackend, PlatformViewBounds, PlatformWindow,
};
pub use window::{
    Backend, ColorScheme, ScopedCommand, TimerCommand, UnknownBackend, Waker, WindowCommand,
    WindowConfig, WindowError, WindowKey, WindowPosition, WindowSpec, run, run_tree,
    run_tree_themed, run_windows,
};
