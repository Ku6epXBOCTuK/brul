//! Loading `.brul` files: SFC parsing, component registry, window arguments.

mod attachments;
mod bindings;
pub mod build;
mod builtin;
pub mod cli;
pub mod commands;
mod diagnostics;
mod discovery;
pub mod emit;
mod generate;
mod kit;
mod loader;
pub mod runtime;
mod source;
pub mod styles;
mod widgets;
mod window;

pub use commands::{Command, Commands};
pub use diagnostics::{arg_mismatch, error_at, padded, sfc_error};
pub use discovery::Capabilities;
pub use generate::{generate, generate_with};
pub use loader::{Definition, Import, Registry, Resource, apply_overrides, load_entry, resolve};
pub use source::{Source, parse as parse_source};
pub use window::{HandlerArg, arguments, root};
