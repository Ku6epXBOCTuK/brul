//! xboct-panel: a Windows services and programs panel on the DSL.

pub mod autostart;
pub mod config;
pub mod console;
pub mod generated;
pub mod log;
pub mod nssm;
pub mod program;
pub mod service;
pub mod single {
    //! One panel per session: a second launch reveals the first window.

    pub use crate::sys::single::{Claim, InstanceError};

    const TITLE: &str = "xboct-panel";
    const NAME: &str = r"Local\xboct-panel";

    pub fn claim() -> Result<Claim, InstanceError> {
        crate::sys::single::claim(NAME)
    }

    pub fn reveal() -> bool {
        crate::sys::single::reveal(TITLE)
    }
}
pub(crate) mod sys;
pub mod tray;

pub use autostart::{AutostartError, Removed};
pub use config::{Config, ConfigError, Entry, EntryKind, Restart, StartMode, default_path};
pub use console::{Console, ConsoleError, Consoles};
pub use log::{LogError, tail};
pub use nssm::NssmError;
pub use program::{Program, ProgramError, ProgramState};
pub use service::{
    Service, ServiceError, ServiceState, StartType, Started, known, restart, start, start_manual,
    stop,
};
