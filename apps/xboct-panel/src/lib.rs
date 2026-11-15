//! xboct-panel: a Windows services and programs panel on the DSL.

pub mod config;
pub mod generated;
pub mod log;
pub mod service;

pub use config::{Config, ConfigError, ServiceSpec, default_path};
pub use log::{LogError, tail};
pub use service::{
    Service, ServiceError, ServiceState, StartType, Started, restart, start, start_manual, stop,
};
