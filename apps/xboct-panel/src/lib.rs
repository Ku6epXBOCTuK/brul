//! xboct-panel: a Windows services and programs panel on the DSL.

pub mod config;
pub mod console;
pub mod generated;
pub mod log;
pub mod program;
pub mod service;

pub use config::{Config, ConfigError, Entry, EntryKind, Restart, default_path};
pub use console::{Console, ConsoleError, Consoles};
pub use log::{LogError, tail};
pub use program::{Program, ProgramError, ProgramState};
pub use service::{
    Service, ServiceError, ServiceState, StartType, Started, restart, start, start_manual, stop,
};
