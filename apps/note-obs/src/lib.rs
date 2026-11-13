//! note-obs: текст в источник OBS с окном на DSL.

// NOTE: UI copy is Russian, so literals in the generated view cannot stay ASCII.
#![allow(clippy::non_ascii_literal)]

pub mod afk;
pub mod config;
pub mod generated;
pub mod obs;
pub mod worker;

pub use config::{Config, ConfigError, default_path};
