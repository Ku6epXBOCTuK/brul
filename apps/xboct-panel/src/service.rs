//! Windows services from the SCM: state, start type and display name.

use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::config::{Entry, EntryKind};
use crate::sys::service::{
    self, ACCESS_DENIED, ALREADY_RUNNING, AUTO_START, BOOT_START, CONTINUE_PENDING, CONTROL,
    DEMAND_START, DISABLED, MISSING, NOT_ACTIVE, PAUSE_PENDING, PAUSED, QUERY, RUNNING,
    START_PENDING, STOP_PENDING, STOPPED, SYSTEM_START, ScManager, ScService,
};

const POLL: Duration = Duration::from_millis(100);
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceState {
    Stopped,
    StartPending,
    StopPending,
    Running,
    ContinuePending,
    PausePending,
    Paused,
    Unknown(u32),
}

impl ServiceState {
    pub fn from_raw(raw: u32) -> Self {
        match raw {
            STOPPED => Self::Stopped,
            START_PENDING => Self::StartPending,
            STOP_PENDING => Self::StopPending,
            RUNNING => Self::Running,
            CONTINUE_PENDING => Self::ContinuePending,
            PAUSE_PENDING => Self::PausePending,
            PAUSED => Self::Paused,
            other => Self::Unknown(other),
        }
    }

    pub fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }
}

impl fmt::Display for ServiceState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Stopped => "stopped",
            Self::StartPending => "starting",
            Self::StopPending => "stopping",
            Self::Running => "running",
            Self::ContinuePending => "resuming",
            Self::PausePending => "pausing",
            Self::Paused => "paused",
            Self::Unknown(_) => "unknown",
        };
        formatter.write_str(label)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartType {
    Boot,
    System,
    Automatic,
    Manual,
    Disabled,
    Unknown(u32),
}

impl StartType {
    pub fn from_raw(raw: u32) -> Self {
        match raw {
            BOOT_START => Self::Boot,
            SYSTEM_START => Self::System,
            AUTO_START => Self::Automatic,
            DEMAND_START => Self::Manual,
            DISABLED => Self::Disabled,
            other => Self::Unknown(other),
        }
    }
}

impl fmt::Display for StartType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Boot => "boot",
            Self::System => "system",
            Self::Automatic => "automatic",
            Self::Manual => "manual",
            Self::Disabled => "disabled",
            Self::Unknown(_) => "unknown",
        };
        formatter.write_str(label)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Service {
    pub name: String,
    pub display: String,
    pub state: ServiceState,
    pub start_type: StartType,
    pub port: Option<u16>,
    pub log: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ServiceError {
    #[error("failed to open the service manager: error code {code}")]
    Manager { code: u32 },
    #[error("failed to open service {name}: error code {code}")]
    Open { name: String, code: u32 },
    #[error("failed to read the state of {name}: error code {code}")]
    Status { name: String, code: u32 },
    #[error("failed to read the configuration of {name}: error code {code}")]
    Config { name: String, code: u32 },
    #[error("failed to start {name}: error code {code}")]
    Start { name: String, code: u32 },
    #[error("failed to stop {name}: error code {code}")]
    Stop { name: String, code: u32 },
    #[error("access to service {name} is denied: run the panel as administrator")]
    AccessDenied { name: String },
    #[error("service {name} is {state}, expected {expected}")]
    Wait {
        name: String,
        state: ServiceState,
        expected: ServiceState,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Started {
    pub started: Vec<String>,
    pub failed: Vec<(String, ServiceError)>,
}

pub fn query_all(specs: &[Entry]) -> Result<Vec<Service>, ServiceError> {
    let manager = Manager::open()?;
    specs
        .iter()
        .filter(|spec| spec.kind == EntryKind::Service)
        .map(|spec| manager.query(spec))
        .collect()
}

pub fn start(name: &str) -> Result<(), ServiceError> {
    Manager::open()?.start(name)
}

pub fn stop(name: &str) -> Result<(), ServiceError> {
    Manager::open()?.stop(name)
}

pub fn restart(name: &str) -> Result<(), ServiceError> {
    Manager::open()?.restart(name)
}

pub fn known(name: &str) -> Result<bool, ServiceError> {
    match Manager::open()?.service(name, QUERY) {
        Ok(_) => Ok(true),
        Err(ServiceError::Open { code, .. }) if code == MISSING => Ok(false),
        Err(failure) => Err(failure),
    }
}

pub fn start_manual(specs: &[Entry]) -> Result<Started, ServiceError> {
    let manager = Manager::open()?;
    let mut report = Started::default();
    for spec in specs.iter().filter(|spec| spec.kind == EntryKind::Service) {
        let service = manager.query(spec)?;
        if !needs_start(service.state, service.start_type) {
            continue;
        }
        match manager.start(&service.name) {
            Ok(()) => report.started.push(service.name),
            Err(failure) => report.failed.push((service.name, failure)),
        }
    }
    Ok(report)
}

fn needs_start(state: ServiceState, start_type: StartType) -> bool {
    start_type == StartType::Manual && !state.is_running()
}

struct Manager(ScManager);

impl Manager {
    fn open() -> Result<Self, ServiceError> {
        service::connect()
            .map(Manager)
            .map_err(|error| ServiceError::Manager { code: error.code() })
    }

    fn service(&self, name: &str, access: u32) -> Result<ScService, ServiceError> {
        self.0
            .open(name, access)
            .map_err(|error| open_error(name, error))
    }

    fn status(&self, name: &str) -> Result<ServiceState, ServiceError> {
        let status = self
            .service(name, QUERY)?
            .status()
            .map_err(|error| ServiceError::Status {
                name: name.to_owned(),
                code: error.code(),
            })?;
        Ok(ServiceState::from_raw(status.state))
    }

    fn query(&self, spec: &Entry) -> Result<Service, ServiceError> {
        let name = spec.name.as_str();
        let handle = self.service(name, QUERY)?;
        let status = handle.status().map_err(|error| ServiceError::Status {
            name: name.to_owned(),
            code: error.code(),
        })?;
        let config = handle.config().map_err(|error| ServiceError::Config {
            name: name.to_owned(),
            code: error.code(),
        })?;
        Ok(Service {
            name: name.to_owned(),
            display: config.display,
            state: ServiceState::from_raw(status.state),
            start_type: StartType::from_raw(config.start_type),
            port: spec.port,
            log: spec.log.clone(),
        })
    }

    fn start(&self, name: &str) -> Result<(), ServiceError> {
        let handle = self.service(name, CONTROL)?;
        if let Err(error) = handle.start()
            && error.code() != ALREADY_RUNNING
        {
            return Err(ServiceError::Start {
                name: name.to_owned(),
                code: error.code(),
            });
        }
        self.wait_for(name, ServiceState::Running)
    }

    fn stop(&self, name: &str) -> Result<(), ServiceError> {
        let handle = self.service(name, CONTROL)?;
        if let Err(error) = handle.stop()
            && error.code() != NOT_ACTIVE
        {
            return Err(ServiceError::Stop {
                name: name.to_owned(),
                code: error.code(),
            });
        }
        self.wait_for(name, ServiceState::Stopped)
    }

    fn restart(&self, name: &str) -> Result<(), ServiceError> {
        if self.status(name)? != ServiceState::Stopped {
            self.stop(name)?;
        }
        self.start(name)
    }

    fn wait_for(&self, name: &str, expected: ServiceState) -> Result<(), ServiceError> {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let state = self.status(name)?;
            if state == expected {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(ServiceError::Wait {
                    name: name.to_owned(),
                    state,
                    expected,
                });
            }
            std::thread::sleep(POLL);
        }
    }
}

fn open_error(name: &str, error: crate::sys::Error) -> ServiceError {
    if error.code() == ACCESS_DENIED {
        ServiceError::AccessDenied {
            name: name.to_owned(),
        }
    } else {
        ServiceError::Open {
            name: name.to_owned(),
            code: error.code(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_known_state_is_named() {
        assert_eq!(ServiceState::from_raw(RUNNING), ServiceState::Running);
        assert_eq!(ServiceState::from_raw(STOPPED), ServiceState::Stopped);
        assert_eq!(
            ServiceState::from_raw(START_PENDING),
            ServiceState::StartPending
        );
        assert_eq!(
            ServiceState::from_raw(STOP_PENDING),
            ServiceState::StopPending
        );
        assert_eq!(
            ServiceState::from_raw(PAUSE_PENDING),
            ServiceState::PausePending
        );
        assert_eq!(ServiceState::from_raw(PAUSED), ServiceState::Paused);
        assert_eq!(
            ServiceState::from_raw(CONTINUE_PENDING),
            ServiceState::ContinuePending
        );
        assert_eq!(ServiceState::from_raw(99), ServiceState::Unknown(99));
        assert_eq!(ServiceState::Running.to_string(), "running");
    }

    #[test]
    fn every_known_start_type_is_named() {
        assert_eq!(StartType::from_raw(AUTO_START), StartType::Automatic);
        assert_eq!(StartType::from_raw(DEMAND_START), StartType::Manual);
        assert_eq!(StartType::from_raw(DISABLED), StartType::Disabled);
        assert_eq!(StartType::from_raw(BOOT_START), StartType::Boot);
        assert_eq!(StartType::from_raw(SYSTEM_START), StartType::System);
        assert_eq!(StartType::from_raw(42), StartType::Unknown(42));
        assert_eq!(StartType::Manual.to_string(), "manual");
    }

    #[test]
    fn only_a_stopped_manual_service_needs_a_start() {
        assert!(needs_start(ServiceState::Stopped, StartType::Manual));
        assert!(!needs_start(ServiceState::Running, StartType::Manual));
        assert!(!needs_start(ServiceState::Stopped, StartType::Automatic));
        assert!(!needs_start(ServiceState::Stopped, StartType::Disabled));
    }
}
