//! Загрузка `.brul`: разбор SFC, реестр компонентов, аргументы окна.

pub mod build;
mod diagnostics;
pub mod emit;
mod loader;
pub mod runtime;
mod source;
pub mod styles;
mod window;

pub use diagnostics::{error_at, padded, sfc_error};
pub use loader::{Definition, Import, Registry, Resource, resolve};
pub use source::{Source, parse as parse_source};
pub use window::{HandlerArg, arguments, root};
