//! Programs the panel launches itself: detached process, PID in the state file.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::mem::{size_of, zeroed};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HWND, WAIT_OBJECT_0};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    CREATE_NEW_PROCESS_GROUP, CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DETACHED_PROCESS,
    OpenProcess, PROCESS_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    PROCESS_TERMINATE, QueryFullProcessImageNameW, STARTUPINFOW, TerminateProcess,
    WaitForSingleObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, PostMessageW, WM_CLOSE,
};

use crate::config::{Entry, EntryKind, default_path};

const STATE_FILE: &str = "xboct-panel.state.toml";
const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);
const KILL_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramState {
    Running,
    Stopped,
}

impl fmt::Display for ProgramState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Running => f.write_str("running"),
            Self::Stopped => f.write_str("stopped"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub name: String,
    pub display: String,
    pub state: ProgramState,
    pub port: Option<u16>,
    pub log: Option<PathBuf>,
    pub pid: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProgramError {
    #[error("no command for program '{name}'")]
    NoCommand { name: String },
    #[error("cannot start '{name}': {message}")]
    Spawn { name: String, message: String },
    #[error("cannot stop '{name}': {message}")]
    Stop { name: String, message: String },
    #[error("cannot write {}: {message}", .path.display())]
    State { path: PathBuf, message: String },
}

#[derive(Default, Deserialize, Serialize)]
struct Saved {
    #[serde(default)]
    pids: BTreeMap<String, u32>,
}

pub fn query_all(entries: &[Entry]) -> Vec<Program> {
    let saved = load(&state_path());
    entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Program)
        .map(|entry| program(entry, &saved))
        .collect()
}

pub fn start(entry: &Entry) -> Result<u32, ProgramError> {
    start_with(entry, &state_path())
}

pub fn stop(entry: &Entry) -> Result<(), ProgramError> {
    stop_with(entry, &state_path())
}

pub fn restart(entry: &Entry) -> Result<u32, ProgramError> {
    stop(entry)?;
    start(entry)
}

fn state_path() -> PathBuf {
    default_path().with_file_name(STATE_FILE)
}

fn program(entry: &Entry, saved: &Saved) -> Program {
    let pid = find_pid(entry, saved);
    Program {
        name: entry.name.clone(),
        display: entry.display.clone().unwrap_or_else(|| entry.name.clone()),
        state: match pid {
            Some(_) => ProgramState::Running,
            None => ProgramState::Stopped,
        },
        port: entry.port,
        log: entry.log.clone(),
        pid,
    }
}

fn start_with(entry: &Entry, state: &Path) -> Result<u32, ProgramError> {
    let mut saved = load(state);
    if let Some(pid) = find_pid(entry, &saved) {
        remember(state, &mut saved, &entry.name, pid)?;
        return Ok(pid);
    }
    let pid = spawn(entry)?;
    remember(state, &mut saved, &entry.name, pid)?;
    Ok(pid)
}

fn stop_with(entry: &Entry, state: &Path) -> Result<(), ProgramError> {
    let mut saved = load(state);
    let Some(pid) = find_pid(entry, &saved) else {
        return forget(state, &mut saved, &entry.name);
    };
    let handle = unsafe {
        OpenProcess(
            PROCESS_TERMINATE | PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        )
    };
    if handle.is_null() {
        return forget(state, &mut saved, &entry.name);
    }
    close_windows(pid);
    let stopped = unsafe { WaitForSingleObject(handle, CLOSE_TIMEOUT.as_millis() as u32) };
    if stopped != WAIT_OBJECT_0 {
        unsafe {
            TerminateProcess(handle, 0);
        }
        let killed = unsafe { WaitForSingleObject(handle, KILL_TIMEOUT.as_millis() as u32) };
        if killed != WAIT_OBJECT_0 {
            unsafe {
                CloseHandle(handle);
            }
            return Err(ProgramError::Stop {
                name: entry.name.clone(),
                message: format!("process {pid} is still running"),
            });
        }
    }
    unsafe {
        CloseHandle(handle);
    }
    forget(state, &mut saved, &entry.name)
}

fn remember(state: &Path, saved: &mut Saved, name: &str, pid: u32) -> Result<(), ProgramError> {
    if saved.pids.get(name) == Some(&pid) {
        return Ok(());
    }
    saved.pids.insert(name.to_string(), pid);
    save(state, saved)
}

fn forget(state: &Path, saved: &mut Saved, name: &str) -> Result<(), ProgramError> {
    if saved.pids.remove(name).is_none() {
        return Ok(());
    }
    save(state, saved)
}

fn load(path: &Path) -> Saved {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}

fn save(path: &Path, saved: &Saved) -> Result<(), ProgramError> {
    let text = toml::to_string(saved).map_err(|error| ProgramError::State {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    std::fs::write(path, text).map_err(|error| ProgramError::State {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

fn find_pid(entry: &Entry, saved: &Saved) -> Option<u32> {
    let exe = entry.command.as_deref()?;
    if let Some(&pid) = saved.pids.get(&entry.name)
        && alive(pid)
        && exe_path(pid).is_some_and(|path| same_path(&path, exe))
    {
        return Some(pid);
    }
    find_by_exe(exe)
}

fn alive(pid: u32) -> bool {
    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            0,
            pid,
        )
    };
    if handle.is_null() {
        return false;
    }
    let exited = unsafe { WaitForSingleObject(handle, 0) } == WAIT_OBJECT_0;
    unsafe {
        CloseHandle(handle);
    }
    !exited
}

pub(crate) fn exe_path(pid: u32) -> Option<String> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return None;
    }
    let mut buffer = [0u16; 1024];
    let mut size = buffer.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size) };
    unsafe {
        CloseHandle(handle);
    }
    (ok != 0).then(|| String::from_utf16_lossy(&buffer[..size as usize]))
}

pub(crate) fn find_by_exe(exe: &str) -> Option<u32> {
    let file = Path::new(exe)
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())?;
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() {
        return None;
    }
    let mut entry: PROCESSENTRY32W = unsafe { zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut found = None;
    unsafe {
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let name = String::from_utf16_lossy(
                    &entry.szExeFile[..entry.szExeFile.iter().position(|c| *c == 0).unwrap_or(0)],
                );
                if name.to_lowercase() == file
                    && entry.th32ProcessID != 0
                    && exe_path(entry.th32ProcessID).is_some_and(|path| same_path(&path, exe))
                {
                    found = Some(entry.th32ProcessID);
                    break;
                }
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
    }
    found
}

fn same_path(left: &str, right: &str) -> bool {
    fn normalize(path: &str) -> String {
        path.trim()
            .trim_matches('"')
            .replace('/', "\\")
            .to_lowercase()
    }
    normalize(left) == normalize(right)
}

fn spawn(entry: &Entry) -> Result<u32, ProgramError> {
    let command = entry
        .command
        .as_deref()
        .ok_or_else(|| ProgramError::NoCommand {
            name: entry.name.clone(),
        })?;
    let exe = command.trim().trim_matches('"');
    if exe.is_empty() {
        return Err(ProgramError::NoCommand {
            name: entry.name.clone(),
        });
    }
    let exe_wide = wide(exe);
    let mut cmdline = wide(&command_line(exe, &entry.args));
    let cwd = entry
        .workdir
        .clone()
        .or_else(|| Path::new(exe).parent().map(Path::to_path_buf));
    let cwd_wide = cwd.map(|dir| wide(&dir.to_string_lossy()));
    let environment = environment(&entry.env);
    let mut startup: STARTUPINFOW = unsafe { zeroed() };
    startup.cb = size_of::<STARTUPINFOW>() as u32;
    let mut process: PROCESS_INFORMATION = unsafe { zeroed() };
    let ok = unsafe {
        CreateProcessW(
            exe_wide.as_ptr(),
            cmdline.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_UNICODE_ENVIRONMENT,
            environment.as_ref().map_or(std::ptr::null(), |block| {
                block.as_ptr().cast::<core::ffi::c_void>()
            }),
            cwd_wide
                .as_ref()
                .map_or(std::ptr::null(), |wide| wide.as_ptr()),
            &startup,
            &mut process,
        )
    };
    if ok == 0 {
        let error = unsafe { GetLastError() };
        return Err(ProgramError::Spawn {
            name: entry.name.clone(),
            message: format!("CreateProcessW failed (error {error}): {exe}"),
        });
    }
    unsafe {
        CloseHandle(process.hThread);
        CloseHandle(process.hProcess);
    }
    Ok(process.dwProcessId)
}

pub(crate) fn command_line(exe: &str, args: &[String]) -> String {
    let mut line = quote(exe);
    for arg in args {
        line.push(' ');
        line.push_str(&quote(arg));
    }
    line
}

pub(crate) fn quote(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        return arg.to_string();
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    let mut backslashes = 0;
    for c in arg.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.extend(std::iter::repeat_n('\\', backslashes));
                out.push(c);
                backslashes = 0;
            }
        }
    }
    out.extend(std::iter::repeat_n('\\', backslashes * 2));
    out.push('"');
    out
}

pub(crate) fn environment(overrides: &BTreeMap<String, String>) -> Option<Vec<u16>> {
    if overrides.is_empty() {
        return None;
    }
    let replaced: BTreeSet<String> = overrides.keys().map(|key| key.to_uppercase()).collect();
    let mut pairs: Vec<(String, String)> = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .filter(|(key, _)| !replaced.contains(&key.to_uppercase()))
        .collect();
    pairs.extend(
        overrides
            .iter()
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    pairs.sort_by_key(|(key, _)| key.to_uppercase());
    let mut block = Vec::new();
    for (key, value) in pairs {
        block.extend(format!("{key}={value}").encode_utf16());
        block.push(0);
    }
    block.push(0);
    Some(block)
}

struct CloseRequest {
    pid: u32,
    windows: Vec<HWND>,
}

fn close_windows(pid: u32) {
    let mut request = CloseRequest {
        pid,
        windows: Vec::new(),
    };
    unsafe {
        EnumWindows(Some(collect_windows), &mut request as *mut _ as isize);
    }
    for window in request.windows {
        unsafe {
            PostMessageW(window, WM_CLOSE, 0, 0);
        }
    }
}

unsafe extern "system" fn collect_windows(window: HWND, lparam: isize) -> i32 {
    let request = unsafe { &mut *(lparam as *mut CloseRequest) };
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut pid);
    }
    if pid == request.pid {
        request.windows.push(window);
    }
    1
}

pub(crate) fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ping_exe(name: &str) -> PathBuf {
        let source = std::env::var("SystemRoot").expect("SystemRoot") + r"\System32\ping.exe";
        let dir = std::env::temp_dir().join(format!("xboct-panel-test-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let exe = dir.join("ping.exe");
        std::fs::copy(source, &exe).expect("ping copy");
        exe
    }

    fn ping(name: &str) -> Entry {
        Entry {
            name: String::from("sample-ping"),
            kind: EntryKind::Program,
            command: Some(ping_exe(name).to_string_lossy().into_owned()),
            args: vec![String::from("-t"), String::from("127.0.0.1")],
            ..Entry::default()
        }
    }

    fn cleanup(entry: &Entry) {
        if let Some(exe) = entry.command.as_deref()
            && let Some(dir) = Path::new(exe).parent()
        {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    fn temp_state(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("xboct-panel-state-{name}.toml"));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn a_state_file_round_trips() {
        let path = temp_state("roundtrip");
        let mut saved = Saved::default();
        saved.pids.insert(String::from("one"), 42);

        save(&path, &saved).expect("save");
        let loaded = load(&path);

        assert_eq!(loaded.pids.get("one"), Some(&42));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_broken_state_file_is_an_empty_one() {
        let path = temp_state("broken");
        std::fs::write(&path, "not toml").expect("state file");

        assert!(load(&path).pids.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn paths_compare_ignoring_case_and_quotes() {
        assert!(same_path(
            r#""C:\Program Files\Deskflow\deskflow-core.exe""#,
            r"c:/program files/deskflow/deskflow-core.exe"
        ));
        assert!(!same_path(r"C:\one\app.exe", r"C:\two\app.exe"));
    }

    #[test]
    fn a_program_is_started_once_and_stopped() {
        let state = temp_state("lifecycle");
        let entry = ping("lifecycle");

        let pid = start_with(&entry, &state).expect("start");
        assert!(alive(pid));
        assert_eq!(start_with(&entry, &state).expect("second start"), pid);
        let running = program(&entry, &load(&state));
        assert_eq!(running.state, ProgramState::Running);
        assert_eq!(running.pid, Some(pid));

        stop_with(&entry, &state).expect("stop");

        assert!(!alive(pid));
        assert_eq!(program(&entry, &load(&state)).state, ProgramState::Stopped);
        let _ = std::fs::remove_file(&state);
        cleanup(&entry);
    }

    #[test]
    fn a_running_program_is_found_by_its_exe_path() {
        let entry = ping("found");
        let child = std::process::Command::new(entry.command.as_deref().expect("command"))
            .args(&entry.args)
            .spawn()
            .expect("ping");

        let found = find_pid(&entry, &Saved::default());

        assert_eq!(found, Some(child.id()));
        let mut child = child;
        let _ = child.kill();
        let _ = child.wait();
        cleanup(&entry);
    }

    #[test]
    fn a_missing_exe_fails_to_start() {
        let state = temp_state("missing");
        let entry = Entry {
            name: String::from("missing"),
            kind: EntryKind::Program,
            command: Some(String::from(r"C:\no-such-dir\no-such-app.exe")),
            ..Entry::default()
        };

        let failure = start_with(&entry, &state).expect_err("start");

        assert!(failure.to_string().contains("no-such-app"), "{failure}");
        let _ = std::fs::remove_file(&state);
    }

    #[test]
    fn a_program_row_carries_its_fields() {
        let entry = Entry {
            display: Some(String::from("Sample")),
            port: Some(8080),
            log: Some(PathBuf::from(r"C:\logs\sample.log")),
            ..ping("row")
        };

        let row = program(&entry, &Saved::default());

        assert_eq!(row.name, "sample-ping");
        assert_eq!(row.display, "Sample");
        assert_eq!(row.state, ProgramState::Stopped);
        assert_eq!(row.port, Some(8080));
        assert_eq!(row.log, entry.log);
        cleanup(&entry);
    }

    #[test]
    fn an_environment_block_overrides_and_sorts() {
        let mut overrides = BTreeMap::new();
        overrides.insert(String::from("XBOCT_PANEL_TEST"), String::from("1"));

        let block = environment(&overrides).expect("block");
        let text = String::from_utf16_lossy(&block);

        assert!(text.contains("XBOCT_PANEL_TEST=1\0"), "{text}");
        assert!(text.ends_with("\0\0"), "{text}");
        assert!(environment(&BTreeMap::new()).is_none());
    }
}
