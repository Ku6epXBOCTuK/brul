//! Programs the panel launches itself: detached process, PID in the state file.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::{Entry, EntryKind, default_path};
use crate::sys::{self, Handle, process, window};

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
    let Some(handle) = Handle::open(pid) else {
        return forget(state, &mut saved, &entry.name);
    };
    window::close_all(pid);
    if !handle.wait(CLOSE_TIMEOUT) {
        handle.terminate();
        if !handle.wait(KILL_TIMEOUT) {
            return Err(ProgramError::Stop {
                name: entry.name.clone(),
                message: format!("process {pid} is still running"),
            });
        }
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
        && process::alive(pid)
        && process::exe_path(pid).is_some_and(|path| process::same_path(&path, exe))
    {
        return Some(pid);
    }
    process::find_by_exe(exe)
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
    let workdir = entry
        .workdir
        .clone()
        .or_else(|| Path::new(exe).parent().map(Path::to_path_buf));
    let (pid, process) = sys::spawn_detached(exe, &entry.args, workdir.as_deref(), &entry.env)
        .map_err(|error| ProgramError::Spawn {
            name: entry.name.clone(),
            message: format!("{error}: {exe}"),
        })?;
    drop(process);
    Ok(pid)
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
    fn a_program_is_started_once_and_stopped() {
        let state = temp_state("lifecycle");
        let entry = ping("lifecycle");

        let pid = start_with(&entry, &state).expect("start");
        assert!(process::alive(pid));
        assert_eq!(start_with(&entry, &state).expect("second start"), pid);
        let running = program(&entry, &load(&state));
        assert_eq!(running.state, ProgramState::Running);
        assert_eq!(running.pid, Some(pid));

        stop_with(&entry, &state).expect("stop");

        assert!(!process::alive(pid));
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
}
