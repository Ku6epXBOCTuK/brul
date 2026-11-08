//! Запуск браузера без shell.
//!
//! [`Launcher`] собирает [`CommandSpec`] из пути, флагов и URL, а [`Runner`]
//! выполняет его. Системный runner запускает процесс напрямую; тестовый runner
//! записывает команду и ничего не запускает.

use std::fmt;
use std::process::Command;

use crate::config::BrowserEntry;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LaunchError {
    EmptyPath,
    EmptyUrl,
    Spawn(String),
}

impl fmt::Display for LaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPath => formatter.write_str("у браузера пустой путь"),
            Self::EmptyUrl => formatter.write_str("пустой URL"),
            Self::Spawn(message) => write!(formatter, "не удалось запустить браузер: {message}"),
        }
    }
}

impl std::error::Error for LaunchError {}

pub trait Runner {
    fn run(&self, command: &CommandSpec) -> Result<(), LaunchError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, command: &CommandSpec) -> Result<(), LaunchError> {
        Command::new(&command.program)
            .args(&command.args)
            .spawn()
            .map(|_| ())
            .map_err(|error| LaunchError::Spawn(error.to_string()))
    }
}

pub struct Launcher<R = SystemRunner> {
    runner: R,
}

impl Launcher<SystemRunner> {
    pub fn new() -> Self {
        Self {
            runner: SystemRunner,
        }
    }
}

impl Default for Launcher<SystemRunner> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: Runner> Launcher<R> {
    pub fn with_runner(runner: R) -> Self {
        Self { runner }
    }

    pub fn launch(&self, browser: &BrowserEntry, url: &str) -> Result<(), LaunchError> {
        if browser.path.trim().is_empty() {
            return Err(LaunchError::EmptyPath);
        }
        if url.trim().is_empty() {
            return Err(LaunchError::EmptyUrl);
        }

        self.runner.run(&command_for(browser, url))
    }
}

pub fn command_for(browser: &BrowserEntry, url: &str) -> CommandSpec {
    let mut args = split_flags(&browser.flags);

    #[cfg(target_os = "macos")]
    if browser.path.ends_with(".app") {
        let mut open_args = vec!["-a".to_string(), browser.path.clone()];
        if !args.is_empty() {
            open_args.push("--args".to_string());
            open_args.extend(args);
        }
        open_args.push(url.to_string());
        return CommandSpec {
            program: "open".to_string(),
            args: open_args,
        };
    }

    args.push(url.to_string());

    CommandSpec {
        program: browser.path.clone(),
        args,
    }
}

pub fn split_flags(flags: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;

    for character in flags.chars() {
        match quote {
            Some(open) if character == open => quote = None,
            Some(_) => current.push(character),
            None if character == '"' || character == '\'' => quote = Some(character),
            None if character.is_whitespace() => {
                if !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                }
            }
            None => current.push(character),
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}
