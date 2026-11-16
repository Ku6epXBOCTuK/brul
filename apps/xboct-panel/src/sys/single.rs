use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SW_RESTORE, SetForegroundWindow, ShowWindow,
};

use super::text::wide;

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("failed to claim the panel instance: error code {code}")]
pub struct InstanceError {
    pub code: u32,
}

pub struct Claim {
    handle: HANDLE,
    running: bool,
}

impl Claim {
    pub fn running(&self) -> bool {
        self.running
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle) };
    }
}

pub fn claim(name: &str) -> Result<Claim, InstanceError> {
    let name = wide(name);
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    let code = unsafe { GetLastError() };
    if handle.is_null() {
        return Err(InstanceError { code });
    }
    Ok(Claim {
        handle,
        running: code == ERROR_ALREADY_EXISTS,
    })
}

pub fn reveal(title: &str) -> bool {
    let title = wide(title);
    let window = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if window.is_null() {
        return false;
    }
    unsafe {
        ShowWindow(window, SW_RESTORE);
        SetForegroundWindow(window);
    }
    true
}
