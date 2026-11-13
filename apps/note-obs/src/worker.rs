//! Фоновый воркер: OBS трогается из отдельного потока, статус уходит в DSL.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use brul::Handle;
use brul::logging::{error, info};

use crate::afk::{self, Action};
use crate::config::Config;
use crate::obs;

const POLL: Duration = Duration::from_millis(200);

pub enum Command {
    Show(String),
    Clear,
}

#[derive(Clone)]
pub struct Worker {
    commands: Sender<Command>,
}

pub struct Job {
    config: Config,
    inbox: Receiver<Command>,
}

pub fn worker(config: Config) -> (Worker, Job) {
    let (commands, inbox) = mpsc::channel();
    (Worker { commands }, Job { config, inbox })
}

impl Worker {
    pub fn show(&self, text: String) -> Result<(), String> {
        self.send(Command::Show(text))
    }

    pub fn clear(&self) -> Result<(), String> {
        self.send(Command::Clear)
    }

    fn send(&self, command: Command) -> Result<(), String> {
        self.commands
            .send(command)
            .map_err(|_| String::from("OBS worker stopped"))
    }
}

impl Job {
    pub fn start(self, handle: Handle) {
        let Self { config, inbox } = self;
        thread::spawn(move || {
            let mut afk = afk::Afk::new(
                config.afk_enabled,
                config.afk_text.clone(),
                config.afk_after_secs,
            );
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
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                match afk.poll(afk::idle_secs(), Instant::now()) {
                    Some(Action::Away) => write(&config, afk.text(), "отошёл", &handle),
                    Some(Action::Back) => write(&config, "", "idle", &handle),
                    None => {}
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
            handle.set("status", status);
        }
        Err(failure) => {
            error!(%failure, "text not written");
            handle.set("status", failure.to_string());
        }
    }
}
