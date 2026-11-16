//! Console programs under conhost: hidden window, show and hide, exit code, restart.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::config::{Entry, EntryKind, Restart};
use crate::program::ProgramState;
use crate::sys::{Handle, Job, command, process, spawn_console, window};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("process {pid} is still running")]
pub struct StopFailure {
    pid: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("no console window")]
struct NoWindow;

#[derive(Debug, thiserror::Error)]
pub enum ConsoleError {
    #[error("no command for console '{name}'")]
    NoCommand { name: String },
    #[error("cannot start '{name}': {message}")]
    Spawn { name: String, message: String },
    #[error("cannot stop '{name}': {source}")]
    Stop { name: String, source: StopFailure },
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
    handle: Handle,
    job: Option<Job>,
    window: Option<window::Window>,
    started: Instant,
}

impl Runner {
    fn pid(&self) -> u32 {
        self.child.unwrap_or(self.conhost)
    }

    fn alive(&self) -> bool {
        self.handle.is_alive()
    }

    fn exit_code(&self) -> Option<u32> {
        self.handle.exit_code()
    }

    fn stop(&self) -> Result<(), StopFailure> {
        if let Some(window) = self.window {
            window.close();
        }
        if self.handle.wait(CLOSE_TIMEOUT) {
            return Ok(());
        }
        if let Some(job) = &self.job {
            job.terminate();
        }
        self.handle.terminate();
        if self.handle.wait(KILL_TIMEOUT) {
            return Ok(());
        }
        Err(StopFailure { pid: self.pid() })
    }

    fn window(&mut self) -> Result<window::Window, NoWindow> {
        if let Some(window) = self.window
            && window.is_valid()
        {
            return Ok(window);
        }
        let found = window::find_console(self.conhost, ADOPT_TIMEOUT).ok_or(NoWindow)?;
        self.window = Some(found);
        Ok(found)
    }

    fn toggle(&mut self) -> Result<bool, NoWindow> {
        let window = self.window()?;
        if window.is_visible() {
            window.hide();
            Ok(false)
        } else {
            window.show();
            Ok(true)
        }
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
            return runner.stop().map_err(|source| ConsoleError::Stop {
                name: entry.name.clone(),
                source,
            });
        }
        if let Some(pid) = running_pid(entry) {
            kill(pid).map_err(|source| ConsoleError::Stop {
                name: entry.name.clone(),
                source,
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
        runner.toggle().map_err(|_| ConsoleError::Window {
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
    let (program, _) = command::split(command).ok()?;
    process::find_by_exe(&program)
}

fn spawn(entry: &Entry) -> Result<Runner, ConsoleError> {
    let command = entry
        .command
        .as_deref()
        .ok_or_else(|| ConsoleError::NoCommand {
            name: entry.name.clone(),
        })?;
    let (program, mut args) = command::split(command).map_err(|failure| ConsoleError::Spawn {
        name: entry.name.clone(),
        message: failure.to_string(),
    })?;
    args.extend(entry.args.iter().cloned());
    let workdir = entry
        .workdir
        .clone()
        .or_else(|| Path::new(&program).parent().map(Path::to_path_buf));
    let failure = |message: String| ConsoleError::Spawn {
        name: entry.name.clone(),
        message,
    };
    let spawned = spawn_console(&program, &args, workdir.as_deref(), &entry.env)
        .map_err(|error| failure(format!("{error}: {program}")))?;
    let job =
        Job::new().map_err(|error| failure(format!("cannot create a job object: {error}")))?;
    if let Err(error) = job.assign(spawned.process()) {
        spawned.process().terminate();
        return Err(failure(format!(
            "cannot put conhost into the job object: {error}"
        )));
    }
    let conhost = spawned.pid();
    let conhost_handle = spawned.resume();
    let child = process::wait_for_child(conhost, CHILD_TIMEOUT);
    let handle = match child.and_then(Handle::open) {
        Some(child_handle) => child_handle,
        None => conhost_handle,
    };
    let Some(window) = window::find_console(conhost, WINDOW_TIMEOUT) else {
        job.terminate();
        handle.terminate();
        return Err(failure(String::from(
            "console window not found, is it a console program?",
        )));
    };
    window.set_title(&entry.name);
    if entry.hidden {
        window.hide();
    } else {
        window.show();
    }

    Ok(Runner {
        conhost,
        child,
        handle,
        job: Some(job),
        window: Some(window),
        started: Instant::now(),
    })
}

fn adopt(entry: &Entry, pid: u32) -> Option<Runner> {
    let handle = Handle::open(pid)?;
    let conhost = process::parent_of(pid).unwrap_or(0);
    let window = if conhost == 0 {
        None
    } else {
        window::find_console(conhost, ADOPT_TIMEOUT)
    };
    if let Some(window) = window {
        window.set_title(&entry.name);
    }
    Some(Runner {
        conhost,
        child: Some(pid),
        handle,
        job: None,
        window,
        started: Instant::now(),
    })
}

fn kill(pid: u32) -> Result<(), StopFailure> {
    let Some(handle) = Handle::open(pid) else {
        return Ok(());
    };
    if !handle.is_alive() {
        return Ok(());
    }
    handle.terminate();
    if handle.wait(KILL_TIMEOUT) {
        return Ok(());
    }
    Err(StopFailure { pid })
}

#[cfg(test)]
mod tests {
    use super::*;

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
            command::split(entry.command.as_deref().expect("command")).expect("split");
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
