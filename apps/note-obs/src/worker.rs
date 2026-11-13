//! Фоновый воркер: OBS трогается из отдельного потока, статус уходит в DSL.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use brul::Handle;
use brul::logging::{error, info};

use crate::afk::{self, Action};
use crate::config::Config;
use crate::obs;

const POLL: Duration = Duration::from_millis(200);
const SAVE_DELAY: Duration = Duration::from_millis(700);

pub enum Command {
    Show(String),
    Clear,
    Afk(bool),
    AfkText(String),
}

#[derive(Clone)]
pub struct Worker {
    commands: Sender<Command>,
    shared: Arc<Mutex<afk::Snapshot>>,
}

pub struct Job {
    config: Config,
    path: PathBuf,
    inbox: Receiver<Command>,
    afk: afk::Afk,
    shared: Arc<Mutex<afk::Snapshot>>,
}

pub fn worker(config: Config, path: PathBuf) -> (Worker, Job) {
    let (commands, inbox) = mpsc::channel();
    let afk = afk::Afk::new(
        config.afk_enabled,
        config.afk_text.clone(),
        config.afk_after_secs,
    );
    let shared = Arc::new(Mutex::new(afk.snapshot(afk::idle_secs())));
    (
        Worker {
            commands,
            shared: Arc::clone(&shared),
        },
        Job {
            config,
            path,
            inbox,
            afk,
            shared,
        },
    )
}

fn guard(shared: &Mutex<afk::Snapshot>) -> MutexGuard<'_, afk::Snapshot> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Worker {
    pub fn show(&self, text: String) -> Result<(), String> {
        self.send(Command::Show(text))
    }

    pub fn clear(&self) -> Result<(), String> {
        self.send(Command::Clear)
    }

    pub fn afk(&self, enabled: bool) -> Result<(), String> {
        self.send(Command::Afk(enabled))
    }

    pub fn afk_text(&self, text: String) -> Result<(), String> {
        self.send(Command::AfkText(text))
    }

    pub fn afk_note(&self) -> String {
        afk::describe(*guard(&self.shared))
    }

    fn send(&self, command: Command) -> Result<(), String> {
        self.commands
            .send(command)
            .map_err(|_| String::from("OBS worker stopped"))
    }
}

impl Job {
    pub fn start(self, handle: Handle) {
        let Self {
            mut config,
            path,
            inbox,
            mut afk,
            shared,
        } = self;
        thread::spawn(move || {
            let mut save_at: Option<Instant> = None;
            loop {
                match inbox.recv_timeout(POLL) {
                    Ok(Command::Show(text)) => {
                        afk.busy();
                        write(&config, &text, "в эфире", &handle);
                    }
                    Ok(Command::Clear) => {
                        afk.free();
                        write(&config, "", "idle", &handle);
                    }
                    Ok(Command::Afk(enabled)) => {
                        config.afk_enabled = enabled;
                        afk.set(enabled, config.afk_text.clone());
                        save_at = Some(Instant::now() + SAVE_DELAY);
                    }
                    Ok(Command::AfkText(text)) => {
                        config.afk_text = text.clone();
                        afk.set(config.afk_enabled, text);
                        save_at = Some(Instant::now() + SAVE_DELAY);
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                let idle = afk::idle_secs();
                match afk.poll(idle, Instant::now()) {
                    Some(Action::Away) => write(&config, afk.text(), "отошёл", &handle),
                    Some(Action::Back) => write(&config, "", "idle", &handle),
                    None => {}
                }
                *guard(&shared) = afk.snapshot(idle);
                let due = save_at.is_some_and(|at| Instant::now() >= at);
                if due {
                    save_at = None;
                    if let Err(failure) =
                        Config::save_afk(&path, config.afk_enabled, &config.afk_text)
                    {
                        error!(%failure, "afk not saved");
                    }
                }
            }
            info!("OBS worker stopped");
        });
    }
}

fn write(config: &Config, text: &str, status: &str, handle: &Handle) {
    match obs::write_text(config, text) {
        Ok(()) => {
            info!(len = text.len(), "text written");
            handle.set("live", !text.is_empty());
            handle.set("status", status);
        }
        Err(failure) => {
            error!(%failure, "text not written");
            handle.set("status", failure.to_string());
        }
    }
}
