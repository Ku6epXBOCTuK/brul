//! Windows services from the SCM: state, start type and display name.

use std::fmt;
use std::mem::{size_of, zeroed};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_INSUFFICIENT_BUFFER, ERROR_SERVICE_ALREADY_RUNNING,
    ERROR_SERVICE_NOT_ACTIVE, GetLastError,
};
use windows_sys::Win32::System::Services::{
    CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW, QUERY_SERVICE_CONFIGW,
    QueryServiceConfigW, QueryServiceStatusEx, SC_HANDLE, SC_MANAGER_CONNECT,
    SC_STATUS_PROCESS_INFO, SERVICE_AUTO_START, SERVICE_BOOT_START, SERVICE_CONTINUE_PENDING,
    SERVICE_CONTROL_STOP, SERVICE_DEMAND_START, SERVICE_DISABLED, SERVICE_PAUSE_PENDING,
    SERVICE_PAUSED, SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_START,
    SERVICE_START_PENDING, SERVICE_STATUS, SERVICE_STATUS_PROCESS, SERVICE_STOP,
    SERVICE_STOP_PENDING, SERVICE_STOPPED, SERVICE_SYSTEM_START, StartServiceW,
};

use crate::config::ServiceSpec;

const QUERY_ACCESS: u32 = SERVICE_QUERY_CONFIG | SERVICE_QUERY_STATUS;
const CONTROL_ACCESS: u32 = QUERY_ACCESS | SERVICE_START | SERVICE_STOP;

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
            SERVICE_STOPPED => Self::Stopped,
            SERVICE_START_PENDING => Self::StartPending,
            SERVICE_STOP_PENDING => Self::StopPending,
            SERVICE_RUNNING => Self::Running,
            SERVICE_CONTINUE_PENDING => Self::ContinuePending,
            SERVICE_PAUSE_PENDING => Self::PausePending,
            SERVICE_PAUSED => Self::Paused,
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
            SERVICE_BOOT_START => Self::Boot,
            SERVICE_SYSTEM_START => Self::System,
            SERVICE_AUTO_START => Self::Automatic,
            SERVICE_DEMAND_START => Self::Manual,
            SERVICE_DISABLED => Self::Disabled,
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

pub fn query_all(specs: &[ServiceSpec]) -> Result<Vec<Service>, ServiceError> {
    let manager = Manager::open()?;
    specs.iter().map(|spec| manager.query(spec)).collect()
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

pub fn start_manual(specs: &[ServiceSpec]) -> Result<Started, ServiceError> {
    let manager = Manager::open()?;
    let mut report = Started::default();
    for spec in specs {
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

struct Manager(SC_HANDLE);

impl Manager {
    fn open() -> Result<Self, ServiceError> {
        let handle =
            unsafe { OpenSCManagerW(std::ptr::null(), std::ptr::null(), SC_MANAGER_CONNECT) };
        if handle.is_null() {
            return Err(ServiceError::Manager { code: last_error() });
        }
        Ok(Self(handle))
    }

    fn service(&self, name: &str, access: u32) -> Result<ServiceHandle, ServiceError> {
        let wide = wide(name);
        let handle = unsafe { OpenServiceW(self.0, wide.as_ptr(), access) };
        if handle.is_null() {
            return Err(open_error(name));
        }
        Ok(ServiceHandle(handle))
    }

    fn query(&self, spec: &ServiceSpec) -> Result<Service, ServiceError> {
        let name = spec.name.as_str();
        let handle = self.service(name, QUERY_ACCESS)?;
        let status = handle.status(name)?;
        let config = handle.config(name)?;
        Ok(Service {
            name: name.to_owned(),
            display: wide_string(config.lpDisplayName),
            state: ServiceState::from_raw(status.dwCurrentState),
            start_type: StartType::from_raw(config.dwStartType),
            port: spec.port,
            log: spec.log.clone(),
        })
    }

    fn start(&self, name: &str) -> Result<(), ServiceError> {
        let handle = self.service(name, CONTROL_ACCESS)?;
        let done = unsafe { StartServiceW(handle.0, 0, std::ptr::null()) };
        if done == 0 {
            let code = last_error();
            if code != ERROR_SERVICE_ALREADY_RUNNING {
                return Err(ServiceError::Start {
                    name: name.to_owned(),
                    code,
                });
            }
        }
        handle.wait_for(name, ServiceState::Running)
    }

    fn stop(&self, name: &str) -> Result<(), ServiceError> {
        let handle = self.service(name, CONTROL_ACCESS)?;
        let mut status: SERVICE_STATUS = unsafe { zeroed() };
        let done = unsafe { ControlService(handle.0, SERVICE_CONTROL_STOP, &mut status) };
        if done == 0 {
            let code = last_error();
            if code != ERROR_SERVICE_NOT_ACTIVE {
                return Err(ServiceError::Stop {
                    name: name.to_owned(),
                    code,
                });
            }
        }
        handle.wait_for(name, ServiceState::Stopped)
    }

    fn restart(&self, name: &str) -> Result<(), ServiceError> {
        let state = self.service(name, CONTROL_ACCESS)?.status(name)?;
        if ServiceState::from_raw(state.dwCurrentState) != ServiceState::Stopped {
            self.stop(name)?;
        }
        self.start(name)
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        unsafe { CloseServiceHandle(self.0) };
    }
}

struct ServiceHandle(SC_HANDLE);

impl ServiceHandle {
    fn status(&self, name: &str) -> Result<SERVICE_STATUS_PROCESS, ServiceError> {
        let mut status: SERVICE_STATUS_PROCESS = unsafe { zeroed() };
        let mut needed = 0;
        let size = size_of::<SERVICE_STATUS_PROCESS>() as u32;
        let done = unsafe {
            QueryServiceStatusEx(
                self.0,
                SC_STATUS_PROCESS_INFO,
                &mut status as *mut SERVICE_STATUS_PROCESS as *mut u8,
                size,
                &mut needed,
            )
        };
        if done == 0 {
            return Err(ServiceError::Status {
                name: name.to_owned(),
                code: last_error(),
            });
        }
        Ok(status)
    }

    fn config(&self, name: &str) -> Result<QUERY_SERVICE_CONFIGW, ServiceError> {
        let slot = size_of::<QUERY_SERVICE_CONFIGW>();
        let mut buffer = vec![QUERY_SERVICE_CONFIGW::default(); 1];
        let mut needed = 0;
        let size = slot as u32;
        let done = unsafe { QueryServiceConfigW(self.0, buffer.as_mut_ptr(), size, &mut needed) };
        if done != 0 {
            return Ok(buffer[0]);
        }
        if last_error() != ERROR_INSUFFICIENT_BUFFER {
            return Err(ServiceError::Config {
                name: name.to_owned(),
                code: last_error(),
            });
        }

        let slots = (needed as usize).div_ceil(slot).max(1);
        buffer = vec![QUERY_SERVICE_CONFIGW::default(); slots];
        let size = (slots * slot) as u32;
        let done = unsafe { QueryServiceConfigW(self.0, buffer.as_mut_ptr(), size, &mut needed) };
        if done == 0 {
            return Err(ServiceError::Config {
                name: name.to_owned(),
                code: last_error(),
            });
        }
        Ok(buffer[0])
    }

    fn wait_for(&self, name: &str, expected: ServiceState) -> Result<(), ServiceError> {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let state = ServiceState::from_raw(self.status(name)?.dwCurrentState);
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

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        unsafe { CloseServiceHandle(self.0) };
    }
}

fn last_error() -> u32 {
    unsafe { GetLastError() }
}

fn open_error(name: &str) -> ServiceError {
    let code = last_error();
    if code == ERROR_ACCESS_DENIED {
        ServiceError::AccessDenied {
            name: name.to_owned(),
        }
    } else {
        ServiceError::Open {
            name: name.to_owned(),
            code,
        }
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_string(pointer: *const u16) -> String {
    if pointer.is_null() {
        return String::new();
    }
    let mut length = 0;
    while length < 1024 && unsafe { *pointer.add(length) } != 0 {
        length += 1;
    }
    let units = unsafe { std::slice::from_raw_parts(pointer, length) };
    String::from_utf16_lossy(units)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_known_state_is_named() {
        assert_eq!(
            ServiceState::from_raw(SERVICE_RUNNING),
            ServiceState::Running
        );
        assert_eq!(
            ServiceState::from_raw(SERVICE_STOPPED),
            ServiceState::Stopped
        );
        assert_eq!(
            ServiceState::from_raw(SERVICE_START_PENDING),
            ServiceState::StartPending
        );
        assert_eq!(
            ServiceState::from_raw(SERVICE_STOP_PENDING),
            ServiceState::StopPending
        );
        assert_eq!(
            ServiceState::from_raw(SERVICE_PAUSE_PENDING),
            ServiceState::PausePending
        );
        assert_eq!(ServiceState::from_raw(SERVICE_PAUSED), ServiceState::Paused);
        assert_eq!(
            ServiceState::from_raw(SERVICE_CONTINUE_PENDING),
            ServiceState::ContinuePending
        );
        assert_eq!(ServiceState::from_raw(99), ServiceState::Unknown(99));
        assert_eq!(ServiceState::Running.to_string(), "running");
    }

    #[test]
    fn every_known_start_type_is_named() {
        assert_eq!(
            StartType::from_raw(SERVICE_AUTO_START),
            StartType::Automatic
        );
        assert_eq!(StartType::from_raw(SERVICE_DEMAND_START), StartType::Manual);
        assert_eq!(StartType::from_raw(SERVICE_DISABLED), StartType::Disabled);
        assert_eq!(StartType::from_raw(SERVICE_BOOT_START), StartType::Boot);
        assert_eq!(StartType::from_raw(SERVICE_SYSTEM_START), StartType::System);
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

    #[test]
    fn a_wide_string_is_read_until_the_terminator() {
        let text = wide("Services");
        assert_eq!(wide_string(text.as_ptr()), "Services");
        assert_eq!(wide_string(std::ptr::null()), "");
    }
}
