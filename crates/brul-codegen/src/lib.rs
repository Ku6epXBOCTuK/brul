//! Загрузка `.brul`: разбор SFC, реестр компонентов, аргументы окна.

mod diagnostics;
mod loader;
mod source;
mod window;

pub use diagnostics::{error_at, padded, sfc_error};
pub use loader::{Definition, Import, Registry, resolve};
pub use source::{Source, parse as parse_source};
pub use window::{HandlerArg, arguments, root};
