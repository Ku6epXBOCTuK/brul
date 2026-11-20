//! Загрузка `.brul`: разбор SFC, реестр компонентов, аргументы окна.

pub mod build;
mod commands;
mod diagnostics;
pub mod emit;
mod kit;
mod loader;
pub mod runtime;
mod source;
pub mod styles;
mod window;

pub use commands::{Command, Commands};
pub use diagnostics::{arg_mismatch, error_at, padded, sfc_error};
pub use loader::{Definition, Import, Registry, Resource, apply_overrides, load_entry, resolve};
pub use source::{Source, parse as parse_source};
pub use window::{HandlerArg, arguments, root};
