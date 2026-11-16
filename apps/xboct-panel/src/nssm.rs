//! Installing and updating a service through nssm.

use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

use crate::{Entry, StartMode};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NssmError {
    #[error("entry {name} is a service but has no command")]
    NoCommand { name: String },
    #[error("failed to run nssm: {message}")]
    Run { message: String },
    #[error("nssm failed: {message}")]
    Failed { message: String },
}

pub fn apply(nssm: &Path, entry: &Entry, exists: bool) -> Result<(), NssmError> {
    for command in commands(entry, exists)? {
        run(nssm, &command)?;
    }
    Ok(())
}

fn commands(entry: &Entry, exists: bool) -> Result<Vec<Vec<String>>, NssmError> {
    let Some(command) = entry
        .command
        .as_deref()
        .filter(|command| !command.trim().is_empty())
    else {
        return Err(NssmError::NoCommand {
            name: entry.name.clone(),
        });
    };
    let name = entry.name.as_str();
    let set = |key: &str, values: Vec<String>| {
        let mut command = vec![String::from("set"), name.to_owned(), key.to_owned()];
        command.extend(values);
        command
    };
    let mut commands = if exists {
        vec![set("Application", vec![command.to_owned()])]
    } else {
        vec![vec![
            String::from("install"),
            name.to_owned(),
            command.to_owned(),
        ]]
    };
    if !entry.args.is_empty() {
        commands.push(set("AppParameters", vec![entry.args.join(" ")]));
    }
    if let Some(workdir) = &entry.workdir {
        commands.push(set("AppDirectory", vec![workdir.display().to_string()]));
    }
    if !entry.env.is_empty() {
        let pairs = entry
            .env
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        commands.push(set("AppEnvironmentExtra", pairs));
    }
    if let Some(start) = entry.start {
        let mode = match start {
            StartMode::Auto => "SERVICE_AUTO_START",
            StartMode::Manual => "SERVICE_DEMAND_START",
        };
        commands.push(set("Start", vec![String::from(mode)]));
    }
    Ok(commands)
}

fn run(nssm: &Path, args: &[String]) -> Result<(), NssmError> {
    let output = Command::new(nssm)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| NssmError::Run {
            message: error.to_string(),
        })?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let message = [stderr.as_ref(), stdout.as_ref()]
        .iter()
        .map(|text| text.trim())
        .find(|text| !text.is_empty())
        .map_or_else(
            || format!("nssm {} exited with {}", args.join(" "), output.status),
            String::from,
        );
    Err(NssmError::Failed { message })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn spec() -> Entry {
        Entry {
            name: String::from("caps-lang"),
            command: Some(String::from("C:/tools/caps-lang.exe")),
            args: vec![String::from("--flag"), String::from("value")],
            workdir: Some(String::from("C:/tools").into()),
            env: BTreeMap::from([(String::from("LANG"), String::from("en"))]),
            start: Some(StartMode::Auto),
            ..Entry::default()
        }
    }

    #[test]
    fn a_new_service_is_installed_and_configured() {
        let commands = commands(&spec(), false).expect("commands");

        assert_eq!(
            commands,
            vec![
                vec!["install", "caps-lang", "C:/tools/caps-lang.exe"],
                vec!["set", "caps-lang", "AppParameters", "--flag value"],
                vec!["set", "caps-lang", "AppDirectory", "C:/tools"],
                vec!["set", "caps-lang", "AppEnvironmentExtra", "LANG=en"],
                vec!["set", "caps-lang", "Start", "SERVICE_AUTO_START"],
            ]
        );
    }

    #[test]
    fn an_existing_service_has_its_application_replaced() {
        let commands = commands(&spec(), true).expect("commands");

        assert_eq!(
            commands[0],
            vec!["set", "caps-lang", "Application", "C:/tools/caps-lang.exe"]
        );
        assert_eq!(commands.len(), 5);
    }

    #[test]
    fn a_service_without_a_command_is_rejected() {
        let entry = Entry {
            name: String::from("empty"),
            ..Entry::default()
        };

        let failure = commands(&entry, false).expect_err("no command");

        assert_eq!(
            failure,
            NssmError::NoCommand {
                name: String::from("empty")
            }
        );
    }
}
