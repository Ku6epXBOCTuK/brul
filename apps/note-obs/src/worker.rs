//! Фоновый воркер: OBS трогается из отдельного потока, статус уходит в DSL.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use brul::Handle;
use brul::logging::{error, info};

use crate::config::Config;
use crate::obs;

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
            for command in inbox {
                let (text, idle) = match command {
                    Command::Show(text) => (text, "в эфире"),
                    Command::Clear => (String::new(), "idle"),
                };
                match obs::write_text(&config, &text) {
                    Ok(()) => {
                        info!(len = text.len(), "text written");
                        handle.set("status", idle);
                    }
                    Err(failure) => {
                        error!(%failure, "text not written");
                        handle.set("status", failure.to_string());
                    }
                }
            }
        });
    }
}
