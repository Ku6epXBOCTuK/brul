use windows_sys::Win32::Foundation::GetLastError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("windows error {code}")]
pub(crate) struct Error {
    code: u32,
}

impl Error {
    pub(crate) fn code(self) -> u32 {
        self.code
    }

    pub(crate) fn last() -> Self {
        Self {
            code: unsafe { GetLastError() },
        }
    }
}
