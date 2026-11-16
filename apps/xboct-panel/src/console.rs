//! Console programs under conhost: hidden window, show and hide, exit code, restart.

use std::collections::HashMap;
use std::mem::{size_of, zeroed};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HWND, LocalFree, WAIT_OBJECT_0};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::Threading::{
    CreateProcessW, GetExitCodeProcess, OpenProcess, PROCESS_INFORMATION,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, ResumeThread,
    STARTF_USESHOWWINDOW, STARTUPINFOW, TerminateProcess, WaitForSingleObject,
};
use windows_sys::Win32::UI::Shell::CommandLineToArgvW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowThreadProcessId, IsWindow, IsWindowVisible, PostMessageW,
    SW_HIDE, SW_SHOWNORMAL, SetForegroundWindow, SetWindowTextW, ShowWindow, WM_CLOSE,
};

use crate::config::{Entry, EntryKind, Restart};
use crate::program::{self, ProgramState};

const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
const CREATE_SUSPENDED: u32 = 0x0000_0004;
const CREATE_UNICODE_ENVIRONMENT: u32 = 0x0000_0400;
const STATUS_CONTROL_C_EXIT: u32 = 0xC000_013A;

const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);
const KILL_TIMEOUT: Duration = Duration::from_secs(5);
const CHILD_TIMEOUT: Duration = Duration::from_secs(3);
const WINDOW_TIMEOUT: Duration = Duration::from_secs(3);
const ADOPT_TIMEOUT: Duration = Duration::from_millis(300);

const RESTART_GAP: Duration = Duration::from_secs(2);
const RESTART_CAP: Duration = Duration::from_secs(60);
const RESTART_MAX: u32 = 5;
const RESTART_RESET: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Console {
    pub name: String,
    pub display: String,
    pub state: ProgramState,
    pub port: Option<u16>,
    pub log: Option<PathBuf>,
    pub pid: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConsoleError {
    #[error("no command for console '{name}'")]
    NoCommand { name: String },
    #[error("cannot start '{name}': {message}")]
    Spawn { name: String, message: String },
    #[error("cannot stop '{name}': {message}")]
    Stop { name: String, message: String },
    #[error("console '{name}' is not running")]
    NotRunning { name: String },
    #[error("cannot find the console window of '{name}'")]
    Window { name: String },
}

#[derive(Default)]
pub struct Consoles {
    tracks: HashMap<String, Track>,
}

#[derive(Default)]
struct Track {
    runner: Option<Runner>,
    manual_stop: bool,
    attempts: u32,
    next_at: Option<Instant>,
    gave_up: bool,
}

struct Runner {
    conhost: u32,
    child: Option<u32>,
    handle: *mut core::ffi::c_void,
    job: *mut core::ffi::c_void,
    window: HWND,
    started: Instant,
}

impl Drop for Runner {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                CloseHandle(self.handle);
            }
            if !self.job.is_null() {
                CloseHandle(self.job);
            }
        }
    }
}

impl Runner {
    fn pid(&self) -> u32 {
        self.child.unwrap_or(self.conhost)
    }

    fn alive(&self) -> bool {
        if self.handle.is_null() {
            return false;
        }
        unsafe { WaitForSingleObject(self.handle, 0) != WAIT_OBJECT_0 }
    }

    fn exit_code(&self) -> Option<u32> {
        if self.handle.is_null() || unsafe { WaitForSingleObject(self.handle, 0) } != WAIT_OBJECT_0
        {
            return None;
        }
        let mut code = 0;
        unsafe {
            GetExitCodeProcess(self.handle, &mut code);
        }
        Some(code)
    }

    fn wait(&self, timeout: Duration) -> bool {
        if self.handle.is_null() {
            return true;
        }
        unsafe { WaitForSingleObject(self.handle, timeout.as_millis() as u32) == WAIT_OBJECT_0 }
    }

    fn terminate(&self) {
        unsafe {
            if !self.job.is_null() {
                TerminateJobObject(self.job, 1);
            }
            if !self.handle.is_null() {
                TerminateProcess(self.handle, 1);
            }
        }
    }

    fn stop(&self) -> Result<(), String> {
        if !self.window.is_null() {
            unsafe {
                PostMessageW(self.window, WM_CLOSE, 0, 0);
            }
        }
        if self.wait(CLOSE_TIMEOUT) {
            return Ok(());
        }
        self.terminate();
        if self.wait(KILL_TIMEOUT) {
            return Ok(());
        }
        Err(format!("process {} is still running", self.pid()))
    }

    fn window(&mut self) -> Result<HWND, ()> {
        if !self.window.is_null() && unsafe { IsWindow(self.window) } != 0 {
            return Ok(self.window);
        }
        let found = find_window(self.conhost, ADOPT_TIMEOUT);
        if found.is_null() {
            return Err(());
        }
        self.window = found;
        Ok(found)
    }

    fn toggle(&mut self) -> Result<bool, ()> {
        let window = self.window()?;
        let visible = unsafe { IsWindowVisible(window) } != 0;
        unsafe {
            if visible {
                ShowWindow(window, SW_HIDE);
            } else {
                ShowWindow(window, SW_SHOWNORMAL);
                SetForegroundWindow(window);
            }
        }
        Ok(!visible)
    }
}

impl Consoles {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn query(&mut self, entries: &[Entry]) -> Vec<Console> {
        let now = Instant::now();
        let mut rows = Vec::new();
        for entry in entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::Console)
        {
            let track = self.tracks.entry(entry.name.clone()).or_default();
            supervise(track, entry, now);
            rows.push(Console {
                name: entry.name.clone(),
                display: entry.display.clone().unwrap_or_else(|| entry.name.clone()),
                state: match track.runner.as_ref() {
                    Some(runner) if runner.alive() => ProgramState::Running,
                    _ => ProgramState::Stopped,
                },
                port: entry.port,
                log: entry.log.clone(),
                pid: track.runner.as_ref().map(Runner::pid),
            });
        }
        rows
    }

    pub fn start(&mut self, entry: &Entry) -> Result<(), ConsoleError> {
        let track = self.tracks.entry(entry.name.clone()).or_default();
        track.manual_stop = false;
        track.attempts = 0;
        track.gave_up = false;
        track.next_at = None;
        if track.runner.as_ref().is_some_and(Runner::alive) {
            return Ok(());
        }
        if let Some(pid) = running_pid(entry)
            && let Some(runner) = adopt(entry, pid)
        {
            track.runner = Some(runner);
            return Ok(());
        }
        track.runner = Some(spawn(entry)?);
        Ok(())
    }

    pub fn stop(&mut self, entry: &Entry) -> Result<(), ConsoleError> {
        let track = self.tracks.entry(entry.name.clone()).or_default();
        track.manual_stop = true;
        track.attempts = 0;
        track.gave_up = false;
        track.next_at = None;
        if let Some(runner) = track.runner.take() {
            return runner.stop().map_err(|message| ConsoleError::Stop {
                name: entry.name.clone(),
                message,
            });
        }
        if let Some(pid) = running_pid(entry) {
            kill(pid).map_err(|message| ConsoleError::Stop {
                name: entry.name.clone(),
                message,
            })?;
        }
        Ok(())
    }

    pub fn restart(&mut self, entry: &Entry) -> Result<(), ConsoleError> {
        self.stop(entry)?;
        self.start(entry)
    }

    pub fn toggle(&mut self, entry: &Entry) -> Result<bool, ConsoleError> {
        let track = self.tracks.entry(entry.name.clone()).or_default();
        let runner = track
            .runner
            .as_mut()
            .ok_or_else(|| ConsoleError::NotRunning {
                name: entry.name.clone(),
            })?;
        runner.toggle().map_err(|()| ConsoleError::Window {
            name: entry.name.clone(),
        })
    }
}

fn supervise(track: &mut Track, entry: &Entry, now: Instant) {
    if let Some(runner) = track.runner.as_ref() {
        if let Some(code) = runner.exit_code() {
            let crashed = !track.manual_stop && code != 0 && !is_user_stop(code);
            track.runner = None;
            if crashed {
                track.attempts += 1;
                if track.attempts > RESTART_MAX {
                    track.gave_up = true;
                } else {
                    track.next_at = Some(now + backoff(track.attempts));
                }
            } else {
                track.attempts = 0;
            }
            return;
        }
        if now.duration_since(runner.started) >= RESTART_RESET {
            track.attempts = 0;
        }
        return;
    }
    if !track.manual_stop
        && let Some(pid) = running_pid(entry)
        && let Some(runner) = adopt(entry, pid)
    {
        track.runner = Some(runner);
        return;
    }
    if track.manual_stop || track.gave_up || entry.restart != Restart::OnCrash {
        return;
    }
    let Some(due) = track.next_at else {
        return;
    };
    if now < due {
        return;
    }
    match spawn(entry) {
        Ok(runner) => {
            track.runner = Some(runner);
            track.next_at = None;
        }
        Err(_) => track.next_at = Some(now + backoff(track.attempts.max(1))),
    }
}

fn backoff(attempt: u32) -> Duration {
    let factor = 1u32 << attempt.saturating_sub(1).min(5);
    RESTART_GAP.saturating_mul(factor).min(RESTART_CAP)
}

fn is_user_stop(code: u32) -> bool {
    code == STATUS_CONTROL_C_EXIT
}

fn running_pid(entry: &Entry) -> Option<u32> {
    let command = entry.command.as_deref()?;
    let (program, _) = split_command(command).ok()?;
    program::find_by_exe(&program)
}

fn spawn(entry: &Entry) -> Result<Runner, ConsoleError> {
    let command = entry
        .command
        .as_deref()
        .ok_or_else(|| ConsoleError::NoCommand {
            name: entry.name.clone(),
        })?;
    let (program, mut args) = split_command(command).map_err(|message| ConsoleError::Spawn {
        name: entry.name.clone(),
        message,
    })?;
    args.extend(entry.args.iter().cloned());
    let conhost = conhost_path();
    let line = format!(
        "{} {}",
        program::quote(&conhost),
        program::command_line(&program, &args)
    );
    let app = program::wide(&conhost);
    let mut cmdline = program::wide(&line);
    let cwd = entry
        .workdir
        .clone()
        .or_else(|| Path::new(&program).parent().map(Path::to_path_buf));
    let cwd_wide = cwd.map(|dir| program::wide(&dir.to_string_lossy()));
    let environment = program::environment(&entry.env);
    let mut startup: STARTUPINFOW = unsafe { zeroed() };
    startup.cb = size_of::<STARTUPINFOW>() as u32;
    startup.dwFlags = STARTF_USESHOWWINDOW;
    startup.wShowWindow = SW_HIDE as u16;
    let mut process: PROCESS_INFORMATION = unsafe { zeroed() };
    let ok = unsafe {
        CreateProcessW(
            app.as_ptr(),
            cmdline.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            CREATE_NEW_CONSOLE | CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
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
        return Err(ConsoleError::Spawn {
            name: entry.name.clone(),
            message: format!("CreateProcessW failed (error {error}): {program}"),
        });
    }

    let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    let mut failure = String::new();
    if job.is_null() {
        failure = String::from("cannot create a job object");
    } else {
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
            failure = String::from("cannot configure the job object");
        } else if unsafe { AssignProcessToJobObject(job, process.hProcess) } == 0 {
            failure = String::from("cannot put conhost into the job object");
        }
    }
    if !failure.is_empty() {
        unsafe {
            TerminateProcess(process.hProcess, 1);
            CloseHandle(process.hThread);
            CloseHandle(process.hProcess);
            if !job.is_null() {
                CloseHandle(job);
            }
        }
        return Err(ConsoleError::Spawn {
            name: entry.name.clone(),
            message: failure,
        });
    }

    unsafe {
        ResumeThread(process.hThread);
        CloseHandle(process.hThread);
    }

    let child = find_child(process.dwProcessId, CHILD_TIMEOUT);
    let handle = match child.and_then(open_handle) {
        Some(handle) => {
            unsafe {
                CloseHandle(process.hProcess);
            }
            handle
        }
        None => process.hProcess,
    };
    let window = find_window(process.dwProcessId, WINDOW_TIMEOUT);
    if window.is_null() {
        unsafe {
            TerminateJobObject(job, 1);
            CloseHandle(handle);
            CloseHandle(job);
        }
        return Err(ConsoleError::Spawn {
            name: entry.name.clone(),
            message: String::from("console window not found, is it a console program?"),
        });
    }

    let title = program::wide(&entry.name);
    unsafe {
        SetWindowTextW(window, title.as_ptr());
        if entry.hidden {
            ShowWindow(window, SW_HIDE);
        } else {
            ShowWindow(window, SW_SHOWNORMAL);
        }
    }

    Ok(Runner {
        conhost: process.dwProcessId,
        child,
        handle,
        job,
        window,
        started: Instant::now(),
    })
}

fn adopt(entry: &Entry, pid: u32) -> Option<Runner> {
    let handle = open_handle(pid)?;
    let conhost = parent_of(pid).unwrap_or(0);
    let window = if conhost == 0 {
        std::ptr::null_mut()
    } else {
        find_window(conhost, ADOPT_TIMEOUT)
    };
    if !window.is_null() {
        let title = program::wide(&entry.name);
        unsafe {
            SetWindowTextW(window, title.as_ptr());
        }
    }
    Some(Runner {
        conhost,
        child: Some(pid),
        handle,
        job: std::ptr::null_mut(),
        window,
        started: Instant::now(),
    })
}

fn kill(pid: u32) -> Result<(), String> {
    let Some(handle) = open_handle(pid) else {
        return Ok(());
    };
    let handle = Handle(handle);
    if unsafe { WaitForSingleObject(handle.0, 0) } == WAIT_OBJECT_0 {
        return Ok(());
    }
    unsafe {
        TerminateProcess(handle.0, 1);
    }
    if unsafe { WaitForSingleObject(handle.0, KILL_TIMEOUT.as_millis() as u32) } == WAIT_OBJECT_0 {
        return Ok(());
    }
    Err(format!("process {pid} is still running"))
}

struct Handle(*mut core::ffi::c_void);

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

fn open_handle(pid: u32) -> Option<*mut core::ffi::c_void> {
    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE,
            0,
            pid,
        )
    };
    (!handle.is_null()).then_some(handle)
}

fn conhost_path() -> String {
    std::env::var("SystemRoot").unwrap_or_else(|_| String::from(r"C:\Windows"))
        + r"\System32\conhost.exe"
}

fn split_command(command: &str) -> Result<(String, Vec<String>), String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err(String::from("command is empty"));
    }
    let wide = program::wide(trimmed);
    let mut argc = 0;
    let argv = unsafe { CommandLineToArgvW(wide.as_ptr(), &mut argc) };
    if argv.is_null() || argc == 0 {
        return Err(format!("cannot parse command {command:?}"));
    }
    let mut parts = Vec::with_capacity(argc as usize);
    for index in 0..argc as usize {
        let ptr = unsafe { *argv.add(index) };
        let len = unsafe { wide_len(ptr) };
        parts.push(String::from_utf16_lossy(unsafe {
            std::slice::from_raw_parts(ptr, len)
        }));
    }
    unsafe {
        LocalFree(argv as *mut core::ffi::c_void);
    }
    let program = parts.remove(0);
    Ok((program, parts))
}

unsafe fn wide_len(ptr: *const u16) -> usize {
    let mut len = 0;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    len
}

fn find_child(parent: u32, timeout: Duration) -> Option<u32> {
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

fn child_of(parent: u32) -> Option<u32> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() {
        return None;
    }
    let mut found = None;
    let mut entry: PROCESSENTRY32W = unsafe { zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    unsafe {
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                if entry.th32ParentProcessID == parent && entry.th32ProcessID != 0 {
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

fn parent_of(pid: u32) -> Option<u32> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() {
        return None;
    }
    let mut found = None;
    let mut entry: PROCESSENTRY32W = unsafe { zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    unsafe {
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                if entry.th32ProcessID == pid {
                    found = Some(entry.th32ParentProcessID);
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

struct WindowSearch {
    conhost: u32,
    found: HWND,
}

fn find_window(conhost: u32, timeout: Duration) -> HWND {
    let deadline = Instant::now() + timeout;
    loop {
        let mut search = WindowSearch {
            conhost,
            found: std::ptr::null_mut(),
        };
        unsafe {
            EnumWindows(Some(find_console_window), &mut search as *mut _ as isize);
        }
        if !search.found.is_null() {
            return search.found;
        }
        if Instant::now() >= deadline {
            return std::ptr::null_mut();
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

unsafe extern "system" fn find_console_window(window: HWND, lparam: isize) -> i32 {
    let search = unsafe { &mut *(lparam as *mut WindowSearch) };
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut pid);
    }
    if pid == 0 {
        return 1;
    }
    let mut buffer = [0u16; 64];
    let len = unsafe { GetClassNameW(window, buffer.as_mut_ptr(), 64) };
    let class = String::from_utf16_lossy(&buffer[..len.max(0) as usize]);
    if class == "ConsoleWindowClass" && belongs_to(pid, search.conhost) {
        search.found = window;
        return 0;
    }
    1
}

fn belongs_to(window_pid: u32, conhost: u32) -> bool {
    window_pid == conhost || parent_of(window_pid).is_some_and(|parent| parent == conhost)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quoted_command_with_spaces_is_split() {
        let (program, args) =
            split_command(r#""C:\Program Files\Deskflow\deskflow-core.exe" server -s "a b.conf""#)
                .expect("command");

        assert_eq!(program, r"C:\Program Files\Deskflow\deskflow-core.exe");
        assert_eq!(
            args,
            vec![
                String::from("server"),
                String::from("-s"),
                String::from("a b.conf")
            ]
        );
    }

    #[test]
    fn a_blank_command_is_rejected() {
        assert!(split_command("   ").is_err());
    }

    #[test]
    fn the_restart_delay_grows_to_its_cap() {
        assert_eq!(backoff(1), Duration::from_secs(2));
        assert_eq!(backoff(2), Duration::from_secs(4));
        assert_eq!(backoff(3), Duration::from_secs(8));
        assert_eq!(backoff(5), Duration::from_secs(32));
        assert_eq!(backoff(6), RESTART_CAP);
        assert_eq!(backoff(60), RESTART_CAP);
    }

    #[test]
    fn a_control_c_exit_is_a_user_stop() {
        assert!(is_user_stop(STATUS_CONTROL_C_EXIT));
        assert!(!is_user_stop(1));
        assert!(!is_user_stop(0));
    }

    fn ping_console(name: &str) -> (Entry, PathBuf) {
        let source = std::env::var("SystemRoot").expect("SystemRoot") + r"\System32\ping.exe";
        let dir = std::env::temp_dir().join(format!("xboct-panel-console-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let exe = dir.join("ping.exe");
        std::fs::copy(source, &exe).expect("ping copy");
        let entry = Entry {
            name: String::from("sample-console"),
            kind: EntryKind::Console,
            command: Some(format!("\"{}\" -t 127.0.0.1", exe.display())),
            hidden: true,
            ..Entry::default()
        };
        (entry, dir)
    }

    #[test]
    fn a_console_starts_hidden_and_is_stopped() {
        let (entry, dir) = ping_console("lifecycle");
        let mut consoles = Consoles::new();

        consoles.start(&entry).expect("start");
        let rows = consoles.query(std::slice::from_ref(&entry));
        assert_eq!(rows[0].state, ProgramState::Running);
        assert_eq!(
            rows[0].pid,
            consoles.tracks[&entry.name]
                .runner
                .as_ref()
                .map(Runner::pid)
        );

        consoles.stop(&entry).expect("stop");
        assert_eq!(
            consoles.query(std::slice::from_ref(&entry))[0].state,
            ProgramState::Stopped
        );
        assert!(consoles.tracks[&entry.name].manual_stop);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_console_window_is_shown_and_hidden() {
        let (entry, dir) = ping_console("window");
        let mut consoles = Consoles::new();
        consoles.start(&entry).expect("start");

        assert!(consoles.toggle(&entry).expect("show"));
        assert!(!consoles.toggle(&entry).expect("hide"));
        consoles.stop(&entry).expect("stop");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_adopted_console_is_found_by_its_command() {
        let (entry, dir) = ping_console("adopted");
        let (program, args) =
            split_command(entry.command.as_deref().expect("command")).expect("split");
        let mut child = std::process::Command::new(program)
            .args(args)
            .spawn()
            .expect("ping");
        let pid = child.id();

        assert!(adopt(&entry, pid).is_some());
        assert!(running_pid(&entry).is_some());

        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(dir);
    }
}
