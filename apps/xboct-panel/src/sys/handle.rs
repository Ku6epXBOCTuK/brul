use std::mem::{size_of, zeroed};
use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject,
};

use super::Error;

pub(crate) struct Handle(*mut core::ffi::c_void);

impl Handle {
    pub(crate) fn open(pid: u32) -> Option<Self> {
        let handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE,
                0,
                pid,
            )
        };
        (!handle.is_null()).then_some(Self(handle))
    }

    pub(super) fn from_raw(handle: *mut core::ffi::c_void) -> Self {
        Self(handle)
    }

    pub(crate) fn is_alive(&self) -> bool {
        unsafe { WaitForSingleObject(self.0, 0) != WAIT_OBJECT_0 }
    }

    pub(crate) fn exit_code(&self) -> Option<u32> {
        if !self.is_alive() {
            let mut code = 0;
            unsafe {
                GetExitCodeProcess(self.0, &mut code);
            }
            return Some(code);
        }
        None
    }

    pub(crate) fn wait(&self, timeout: Duration) -> bool {
        unsafe { WaitForSingleObject(self.0, timeout.as_millis() as u32) == WAIT_OBJECT_0 }
    }

    pub(crate) fn terminate(&self) {
        unsafe {
            TerminateProcess(self.0, 1);
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub(crate) struct Job(*mut core::ffi::c_void);

impl Job {
    pub(crate) fn new() -> Result<Self, Error> {
        let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job.is_null() {
            return Err(Error::last());
        }
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if configured == 0 {
            let error = Error::last();
            unsafe {
                CloseHandle(job);
            }
            return Err(error);
        }
        Ok(Self(job))
    }

    pub(crate) fn assign(&self, process: &Handle) -> Result<(), Error> {
        if unsafe { AssignProcessToJobObject(self.0, process.0) } == 0 {
            return Err(Error::last());
        }
        Ok(())
    }

    pub(crate) fn terminate(&self) {
        unsafe {
            TerminateJobObject(self.0, 1);
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
