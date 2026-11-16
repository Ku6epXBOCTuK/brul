use std::mem::size_of;

use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_INSUFFICIENT_BUFFER, ERROR_SERVICE_ALREADY_RUNNING,
    ERROR_SERVICE_NOT_ACTIVE,
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

use super::{
    Error,
    text::{from_pointer, wide},
};

pub(crate) const ACCESS_DENIED: u32 = ERROR_ACCESS_DENIED;
pub(crate) const ALREADY_RUNNING: u32 = ERROR_SERVICE_ALREADY_RUNNING;
pub(crate) const NOT_ACTIVE: u32 = ERROR_SERVICE_NOT_ACTIVE;
// NOTE: windows-sys does not generate ERROR_SERVICE_DOES_NOT_EXIST
pub(crate) const MISSING: u32 = 1060;

pub(crate) const QUERY: u32 = SERVICE_QUERY_CONFIG | SERVICE_QUERY_STATUS;
pub(crate) const CONTROL: u32 = QUERY | SERVICE_START | SERVICE_STOP;

pub(crate) const STOPPED: u32 = SERVICE_STOPPED;
pub(crate) const START_PENDING: u32 = SERVICE_START_PENDING;
pub(crate) const STOP_PENDING: u32 = SERVICE_STOP_PENDING;
pub(crate) const RUNNING: u32 = SERVICE_RUNNING;
pub(crate) const CONTINUE_PENDING: u32 = SERVICE_CONTINUE_PENDING;
pub(crate) const PAUSE_PENDING: u32 = SERVICE_PAUSE_PENDING;
pub(crate) const PAUSED: u32 = SERVICE_PAUSED;

pub(crate) const BOOT_START: u32 = SERVICE_BOOT_START;
pub(crate) const SYSTEM_START: u32 = SERVICE_SYSTEM_START;
pub(crate) const AUTO_START: u32 = SERVICE_AUTO_START;
pub(crate) const DEMAND_START: u32 = SERVICE_DEMAND_START;
pub(crate) const DISABLED: u32 = SERVICE_DISABLED;

pub(crate) struct Status {
    pub(crate) state: u32,
}

pub(crate) struct Config {
    pub(crate) display: String,
    pub(crate) start_type: u32,
}

pub(crate) struct ScManager(SC_HANDLE);

pub(crate) fn connect() -> Result<ScManager, Error> {
    let handle = unsafe { OpenSCManagerW(std::ptr::null(), std::ptr::null(), SC_MANAGER_CONNECT) };
    if handle.is_null() {
        return Err(Error::last());
    }
    Ok(ScManager(handle))
}

impl ScManager {
    pub(crate) fn open(&self, name: &str, access: u32) -> Result<ScService, Error> {
        let name = wide(name);
        let handle = unsafe { OpenServiceW(self.0, name.as_ptr(), access) };
        if handle.is_null() {
            return Err(Error::last());
        }
        Ok(ScService(handle))
    }
}

impl Drop for ScManager {
    fn drop(&mut self) {
        unsafe {
            CloseServiceHandle(self.0);
        }
    }
}

pub(crate) struct ScService(SC_HANDLE);

impl ScService {
    pub(crate) fn status(&self) -> Result<Status, Error> {
        let mut status: SERVICE_STATUS_PROCESS = unsafe { std::mem::zeroed() };
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
            return Err(Error::last());
        }
        Ok(Status {
            state: status.dwCurrentState,
        })
    }

    pub(crate) fn config(&self) -> Result<Config, Error> {
        let slot = size_of::<QUERY_SERVICE_CONFIGW>();
        let mut buffer = vec![QUERY_SERVICE_CONFIGW::default(); 1];
        let mut needed = 0;
        let done =
            unsafe { QueryServiceConfigW(self.0, buffer.as_mut_ptr(), slot as u32, &mut needed) };
        if done == 0 {
            let error = Error::last();
            if error.code() != ERROR_INSUFFICIENT_BUFFER {
                return Err(error);
            }
            let slots = (needed as usize).div_ceil(slot).max(1);
            buffer = vec![QUERY_SERVICE_CONFIGW::default(); slots];
            let done = unsafe {
                QueryServiceConfigW(
                    self.0,
                    buffer.as_mut_ptr(),
                    (slots * slot) as u32,
                    &mut needed,
                )
            };
            if done == 0 {
                return Err(Error::last());
            }
        }
        Ok(Config {
            display: from_pointer(buffer[0].lpDisplayName),
            start_type: buffer[0].dwStartType,
        })
    }

    pub(crate) fn start(&self) -> Result<(), Error> {
        if unsafe { StartServiceW(self.0, 0, std::ptr::null()) } == 0 {
            return Err(Error::last());
        }
        Ok(())
    }

    pub(crate) fn stop(&self) -> Result<(), Error> {
        let mut status: SERVICE_STATUS = unsafe { std::mem::zeroed() };
        if unsafe { ControlService(self.0, SERVICE_CONTROL_STOP, &mut status) } == 0 {
            return Err(Error::last());
        }
        Ok(())
    }
}

impl Drop for ScService {
    fn drop(&mut self) {
        unsafe {
            CloseServiceHandle(self.0);
        }
    }
}
