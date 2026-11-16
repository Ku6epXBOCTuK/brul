//! Scheduled task that starts the panel at logon.

use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};

const TASK: &str = "xboct-panel";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const LEGACY_RUN: [&str; 2] = ["nobs", "tray-run"];
const LEGACY_SHORTCUT: &str = "caps-lang.lnk";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Removed {
    pub shortcut: bool,
    pub run: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AutostartError {
    #[error("cannot find the panel path: {message}")]
    Path { message: String },
    #[error("failed to run schtasks: {message}")]
    Command { message: String },
    #[error("schtasks failed: {message}")]
    Failed { message: String },
    #[error("failed to update the registry: {message}")]
    Registry { message: String },
}

pub fn install(exe: &Path) -> Result<Removed, AutostartError> {
    schtasks(&create_args(exe))?;
    remove_legacy()
}

pub fn uninstall() -> Result<(), AutostartError> {
    schtasks(&[
        String::from("/Delete"),
        String::from("/F"),
        String::from("/TN"),
        String::from(TASK),
    ])
}

fn create_args(exe: &Path) -> Vec<String> {
    [
        "/Create".to_owned(),
        "/F".to_owned(),
        "/TN".to_owned(),
        TASK.to_owned(),
        "/TR".to_owned(),
        format!("\"{}\" --autostart", exe.display()),
        "/SC".to_owned(),
        "ONLOGON".to_owned(),
        "/RL".to_owned(),
        "HIGHEST".to_owned(),
    ]
    .to_vec()
}

fn schtasks(args: &[String]) -> Result<(), AutostartError> {
    let output = Command::new("schtasks")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| AutostartError::Command {
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
            || format!("schtasks {} exited with {}", args.join(" "), output.status),
            String::from,
        );
    Err(AutostartError::Failed { message })
}

fn remove_legacy() -> Result<Removed, AutostartError> {
    let mut removed = Removed::default();
    if let Some(path) = startup_shortcut() {
        removed.shortcut = std::fs::remove_file(path).is_ok();
    }
    for name in LEGACY_RUN {
        if delete_run_value(name)? {
            removed.run.push(String::from(name));
        }
    }
    Ok(removed)
}

fn startup_shortcut() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(appdata)
            .join(r"Microsoft\Windows\Start Menu\Programs\Startup")
            .join(LEGACY_SHORTCUT),
    )
}

fn delete_run_value(name: &str) -> Result<bool, AutostartError> {
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)
        .map_err(registry_error)?;
    match key.delete_value(name) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(registry_error(error)),
    }
}

fn registry_error(error: io::Error) -> AutostartError {
    AutostartError::Registry {
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_task_starts_the_given_exe_at_logon_with_highest_rights() {
        assert_eq!(
            create_args(Path::new(r"C:\Tools\xboct-panel\xboct-panel.exe")),
            vec![
                "/Create",
                "/F",
                "/TN",
                "xboct-panel",
                "/TR",
                r#""C:\Tools\xboct-panel\xboct-panel.exe" --autostart"#,
                "/SC",
                "ONLOGON",
                "/RL",
                "HIGHEST",
            ]
        );
    }
}
