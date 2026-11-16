use std::mem::{size_of, zeroed};
use std::path::Path;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};

use super::Handle;

struct Seen {
    pid: u32,
    parent: u32,
    name: String,
}

fn snapshot() -> Vec<Seen> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() {
        return Vec::new();
    }
    let mut seen = Vec::new();
    let mut entry: PROCESSENTRY32W = unsafe { zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    unsafe {
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let length = entry
                    .szExeFile
                    .iter()
                    .position(|unit| *unit == 0)
                    .unwrap_or(0);
                seen.push(Seen {
                    pid: entry.th32ProcessID,
                    parent: entry.th32ParentProcessID,
                    name: String::from_utf16_lossy(&entry.szExeFile[..length]),
                });
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
    }
    seen
}

pub(crate) fn exe_path(pid: u32) -> Option<String> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return None;
    }
    let mut buffer = [0u16; 1024];
    let mut size = buffer.len() as u32;
    let done = unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size) };
    unsafe {
        CloseHandle(handle);
    }
    (done != 0).then(|| String::from_utf16_lossy(&buffer[..size as usize]))
}

pub(crate) fn find_by_exe(exe: &str) -> Option<u32> {
    let file = Path::new(exe)
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())?;
    snapshot()
        .into_iter()
        .find(|seen| {
            seen.pid != 0
                && seen.name.to_lowercase() == file
                && exe_path(seen.pid).is_some_and(|path| same_path(&path, exe))
        })
        .map(|seen| seen.pid)
}

pub(crate) fn child_of(parent: u32) -> Option<u32> {
    snapshot()
        .into_iter()
        .find(|seen| seen.parent == parent && seen.pid != 0)
        .map(|seen| seen.pid)
}

pub(crate) fn parent_of(pid: u32) -> Option<u32> {
    snapshot()
        .into_iter()
        .find(|seen| seen.pid == pid)
        .map(|seen| seen.parent)
}

pub(crate) fn wait_for_child(parent: u32, timeout: Duration) -> Option<u32> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(child) = child_of(parent) {
            return Some(child);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub(crate) fn same_path(left: &str, right: &str) -> bool {
    fn normalize(path: &str) -> String {
        path.trim()
            .trim_matches('"')
            .replace('/', "\\")
            .to_lowercase()
    }
    normalize(left) == normalize(right)
}

pub(crate) fn alive(pid: u32) -> bool {
    Handle::open(pid).is_some_and(|handle| handle.is_alive())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_compare_ignoring_case_and_quotes() {
        assert!(same_path(
            r#""C:\Program Files\Deskflow\deskflow-core.exe""#,
            r"c:/program files/deskflow/deskflow-core.exe"
        ));
        assert!(!same_path(r"C:\one\app.exe", r"C:\two\app.exe"));
    }

    #[test]
    fn the_current_process_is_found_by_its_exe() {
        let exe = std::env::current_exe().expect("current exe");
        assert_eq!(
            find_by_exe(&exe.to_string_lossy()),
            Some(std::process::id())
        );
        assert_eq!(
            exe_path(std::process::id()).as_deref(),
            Some(exe.to_string_lossy().as_ref())
        );
    }
}
