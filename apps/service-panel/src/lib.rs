//! service-panel: панель служб Windows на DSL.

// NOTE: UI copy is Russian, so literals in the generated view cannot stay ASCII.
#![allow(clippy::non_ascii_literal)]

pub mod config;
pub mod generated;

pub use config::{Config, ConfigError, ServiceSpec, default_path};
