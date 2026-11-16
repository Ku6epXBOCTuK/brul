use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowThreadProcessId, IsWindow, IsWindowVisible, PostMessageW,
    SW_HIDE, SW_SHOWNORMAL, SetForegroundWindow, SetWindowTextW, ShowWindow, WM_CLOSE,
};

use super::{process, text::wide};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Window(HWND);

impl Window {
    pub(crate) fn is_valid(self) -> bool {
        (unsafe { IsWindow(self.0) }) != 0
    }

    pub(crate) fn is_visible(self) -> bool {
        (unsafe { IsWindowVisible(self.0) }) != 0
    }

    pub(crate) fn show(self) {
        unsafe {
            ShowWindow(self.0, SW_SHOWNORMAL);
            SetForegroundWindow(self.0);
        }
    }

    pub(crate) fn hide(self) {
        unsafe {
            ShowWindow(self.0, SW_HIDE);
        }
    }

    pub(crate) fn set_title(self, title: &str) {
        let title = wide(title);
        unsafe {
            SetWindowTextW(self.0, title.as_ptr());
        }
    }

    pub(crate) fn close(self) {
        unsafe {
            PostMessageW(self.0, WM_CLOSE, 0, 0);
        }
    }
}

struct Search {
    conhost: u32,
    found: HWND,
}

pub(crate) fn find_console(conhost: u32, timeout: Duration) -> Option<Window> {
    let deadline = Instant::now() + timeout;
    loop {
        let mut search = Search {
            conhost,
            found: std::ptr::null_mut(),
        };
        unsafe {
            EnumWindows(Some(find_console_window), &mut search as *mut _ as isize);
        }
        if !search.found.is_null() {
            return Some(Window(search.found));
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

unsafe extern "system" fn find_console_window(window: HWND, lparam: isize) -> i32 {
    let search = unsafe { &mut *(lparam as *mut Search) };
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut pid);
    }
    if pid == 0 {
        return 1;
    }
    let mut buffer = [0u16; 64];
    let length = unsafe { GetClassNameW(window, buffer.as_mut_ptr(), 64) };
    let class = String::from_utf16_lossy(&buffer[..length.max(0) as usize]);
    if class == "ConsoleWindowClass" && belongs_to(pid, search.conhost) {
        search.found = window;
        return 0;
    }
    1
}

fn belongs_to(window_pid: u32, conhost: u32) -> bool {
    window_pid == conhost || process::parent_of(window_pid).is_some_and(|parent| parent == conhost)
}

pub(crate) fn close_all(pid: u32) {
    struct Request {
        pid: u32,
        windows: Vec<HWND>,
    }

    unsafe extern "system" fn collect(window: HWND, lparam: isize) -> i32 {
        let request = unsafe { &mut *(lparam as *mut Request) };
        let mut pid: u32 = 0;
        unsafe {
            GetWindowThreadProcessId(window, &mut pid);
        }
        if pid == request.pid {
            request.windows.push(window);
        }
        1
    }

    let mut request = Request {
        pid,
        windows: Vec::new(),
    };
    unsafe {
        EnumWindows(Some(collect), &mut request as *mut _ as isize);
    }
    for window in request.windows {
        unsafe {
            PostMessageW(window, WM_CLOSE, 0, 0);
        }
    }
}
