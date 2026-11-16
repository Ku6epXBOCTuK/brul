//! Safe wrappers over the Windows calls the panel needs: the only module with `unsafe`.

mod error;
mod handle;
mod spawn;
mod text;

pub(crate) mod command;
pub(crate) mod process;
pub(crate) mod service;
pub(crate) mod window;

pub(crate) use error::Error;
pub(crate) use handle::{Handle, Job};
pub(crate) use spawn::{spawn_console, spawn_detached};
