//! service-panel: a Windows services panel on the DSL.

pub mod config;
pub mod generated;
pub mod service;

pub use config::{Config, ConfigError, ServiceSpec, default_path};
pub use service::{Service, ServiceError, ServiceState, StartType};
