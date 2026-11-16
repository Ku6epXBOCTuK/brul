//! Panel config: which services, programs and console commands to show.

use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use brul::logging::warn;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub nssm: Option<PathBuf>,
    pub entries: Vec<Entry>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Entry {
    pub name: String,
    pub kind: EntryKind,
    pub display: Option<String>,
    pub port: Option<u16>,
    pub log: Option<PathBuf>,
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    pub workdir: Option<PathBuf>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "is_true")]
    pub hidden: bool,
    #[serde(skip_serializing_if = "is_never")]
    pub restart: Restart,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<StartMode>,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            name: String::new(),
            kind: EntryKind::Service,
            display: None,
            port: None,
            log: None,
            command: None,
            args: Vec::new(),
            workdir: None,
            env: BTreeMap::new(),
            hidden: true,
            restart: Restart::Never,
            start: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    #[default]
    Service,
    Program,
    Console,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Restart {
    #[default]
    Never,
    OnCrash,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StartMode {
    Auto,
    Manual,
}

fn is_true(value: &bool) -> bool {
    *value
}

fn is_never(value: &Restart) -> bool {
    *value == Restart::Never
}

impl fmt::Display for EntryKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Service => "service",
            Self::Program => "program",
            Self::Console => "console",
        };
        formatter.write_str(label)
    }
}

impl fmt::Display for Restart {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Never => "never",
            Self::OnCrash => "on-crash",
        };
        formatter.write_str(label)
    }
}

impl fmt::Display for StartMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
        };
        formatter.write_str(label)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read {}: {message}", .path.display())]
    Read { path: PathBuf, message: String },
    #[error("failed to parse {}: {message}", .path.display())]
    Parse { path: PathBuf, message: String },
    #[error("failed to write {}: {message}", .path.display())]
    Write { path: PathBuf, message: String },
    #[error("an entry named {name:?} already exists")]
    Duplicate { name: String },
    #[error("invalid config: {message}")]
    Invalid { message: String },
}

impl Config {
    pub fn parse(source: &str, path: &Path) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(source).map_err(|error| ConfigError::Parse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        config.validate(path)?;
        Ok(config)
    }

    pub fn validate(&self, path: &Path) -> Result<(), ConfigError> {
        self.verify().map_err(|error| match error {
            ConfigError::Invalid { message } => ConfigError::Parse {
                path: path.to_path_buf(),
                message,
            },
            error => error,
        })
    }

    pub fn verify(&self) -> Result<(), ConfigError> {
        let mut seen = HashSet::new();
        for (index, entry) in self.entries.iter().enumerate() {
            if entry.name.trim().is_empty() {
                return Err(ConfigError::Invalid {
                    message: format!("entries[{index}] has no name"),
                });
            }
            if !seen.insert(entry.name.as_str()) {
                return Err(ConfigError::Invalid {
                    message: format!("entries[{index}] repeats name {:?}", entry.name),
                });
            }
            if entry.kind != EntryKind::Service
                && entry
                    .command
                    .as_deref()
                    .is_none_or(|command| command.trim().is_empty())
            {
                return Err(ConfigError::Invalid {
                    message: format!("entries[{index}] has no command"),
                });
            }
        }
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let source = std::fs::read_to_string(path.as_ref()).map_err(|error| ConfigError::Read {
            path: path.as_ref().to_path_buf(),
            message: error.to_string(),
        })?;

        Self::parse(&source, path.as_ref())
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let path = path.as_ref();
        let source = toml::to_string_pretty(self).map_err(|error| ConfigError::Write {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        std::fs::write(path, source).map_err(|error| ConfigError::Write {
            path: path.to_path_buf(),
            message: error.to_string(),
        })
    }

    pub fn upsert(&mut self, original: &str, entry: Entry) -> Result<(), ConfigError> {
        if self
            .entries
            .iter()
            .any(|other| other.name == entry.name && other.name != original)
        {
            return Err(ConfigError::Duplicate { name: entry.name });
        }
        match self.entries.iter_mut().find(|other| other.name == original) {
            Some(slot) => *slot = entry,
            None => self.entries.push(entry),
        }
        Ok(())
    }

    pub fn load_or_create(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        match std::fs::read_to_string(path) {
            Ok(source) => match Self::parse(&source, path) {
                Ok(config) => config,
                Err(error) => {
                    warn!(%error, "config is unusable, using the defaults");
                    Self::default()
                }
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let config = Self::default();
                if let Err(error) = config.save(path) {
                    warn!(%error, "config is not written");
                }
                config
            }
            Err(error) => {
                warn!(path = %path.display(), %error, "config is unreadable, using the defaults");
                Self::default()
            }
        }
    }
}

const CONFIG_FILE: &str = "xboct-panel.toml";

pub fn default_path() -> PathBuf {
    if let Ok(path) = std::env::var("XBOCT_PANEL_CONFIG")
        && !path.is_empty()
    {
        return PathBuf::from(path);
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(CONFIG_FILE)))
        .unwrap_or_else(|| PathBuf::from(CONFIG_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("xboct-panel-config-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir.join(CONFIG_FILE)
    }

    #[test]
    fn a_service_with_port_and_log_is_read() {
        let source = r#"
            [[entries]]
            kind = "service"
            name = "sample-service"
            port = 8080
            log = 'C:\Logs\sample-service.log'
        "#;
        let config = Config::parse(source, Path::new(CONFIG_FILE)).expect("config");
        let service = &config.entries[0];
        assert_eq!(service.kind, EntryKind::Service);
        assert_eq!(service.name, "sample-service");
        assert_eq!(service.port, Some(8080));
        assert_eq!(
            service.log.as_deref(),
            Some(Path::new(r"C:\Logs\sample-service.log"))
        );
    }

    #[test]
    fn a_program_carries_command_arguments_workdir_and_env() {
        let source = r#"
            [[entries]]
            kind = "program"
            name = "nobs"
            command = 'C:\bin\nobs.exe'
            args = ["--tray"]
            workdir = 'C:\bin'
            env = { OBS_PORT = "4455" }
        "#;
        let config = Config::parse(source, Path::new(CONFIG_FILE)).expect("config");
        let program = &config.entries[0];
        assert_eq!(program.kind, EntryKind::Program);
        assert_eq!(program.command.as_deref(), Some(r"C:\bin\nobs.exe"));
        assert_eq!(program.args, ["--tray"]);
        assert_eq!(program.workdir.as_deref(), Some(Path::new(r"C:\bin")));
        assert_eq!(
            program.env.get("OBS_PORT").map(String::as_str),
            Some("4455")
        );
    }

    #[test]
    fn a_console_is_hidden_and_can_restart_on_crash() {
        let source = r#"
            [[entries]]
            kind = "console"
            name = "deskflow-server"
            command = 'deskflow-core server'
            restart = "on-crash"
        "#;
        let config = Config::parse(source, Path::new(CONFIG_FILE)).expect("config");
        let console = &config.entries[0];
        assert_eq!(console.kind, EntryKind::Console);
        assert!(console.hidden);
        assert_eq!(console.restart, Restart::OnCrash);
    }

    #[test]
    fn the_nssm_path_is_read() {
        let source = "nssm = 'C:\\Tools\\nssm.exe'\n\n[[entries]]\nname = \"Spooler\"\n";
        let config = Config::parse(source, Path::new(CONFIG_FILE)).expect("config");
        assert_eq!(
            config.nssm.as_deref(),
            Some(Path::new(r"C:\Tools\nssm.exe"))
        );
    }

    #[test]
    fn the_start_mode_is_read_and_written_only_when_set() {
        let source = "[[entries]]\nname = \"caps-lang\"\nkind = \"service\"\nstart = \"auto\"\n\n[[entries]]\nname = \"Spooler\"\nkind = \"service\"\n";
        let config = Config::parse(source, Path::new(CONFIG_FILE)).expect("config");

        assert_eq!(config.entries[0].start, Some(StartMode::Auto));
        assert_eq!(config.entries[1].start, None);
        let written = toml::to_string_pretty(&config).expect("toml");
        assert!(written.contains("start = \"auto\""));
        assert_eq!(written.matches("start =").count(), 1);
    }

    #[test]
    fn the_old_services_table_is_rejected() {
        let source = "[[services]]\nname = \"Spooler\"\n";
        let failure = Config::parse(source, Path::new(CONFIG_FILE)).expect_err("error");
        assert!(failure.to_string().contains("services"), "{failure}");
    }

    #[test]
    fn an_unknown_key_is_a_parse_error_with_the_line() {
        let source = "[[entries]]\nname = \"Spooler\"\nprot = 631\n";
        let failure = Config::parse(source, Path::new(CONFIG_FILE)).expect_err("error");
        let message = failure.to_string();
        assert!(message.contains(CONFIG_FILE), "{message}");
        assert!(message.contains("line 3"), "{message}");
    }

    #[test]
    fn an_entry_without_a_name_is_rejected() {
        let source = "[[entries]]\nport = 631\n";
        let failure = Config::parse(source, Path::new(CONFIG_FILE)).expect_err("error");
        let message = failure.to_string();
        assert!(message.contains("entries[0] has no name"), "{message}");
    }

    #[test]
    fn a_program_without_a_command_is_rejected() {
        let source = "[[entries]]\nkind = \"program\"\nname = \"nobs\"\n";
        let failure = Config::parse(source, Path::new(CONFIG_FILE)).expect_err("error");
        let message = failure.to_string();
        assert!(message.contains("entries[0] has no command"), "{message}");
    }

    #[test]
    fn a_repeated_name_is_rejected() {
        let source = "[[entries]]\nname = \"Spooler\"\n\n[[entries]]\nname = \"Spooler\"\n";
        let failure = Config::parse(source, Path::new(CONFIG_FILE)).expect_err("error");
        let message = failure.to_string();
        assert!(message.contains("entries[1] repeats name"), "{message}");
    }

    #[test]
    fn a_default_config_has_no_entries() {
        let config = Config::default();
        assert!(config.entries.is_empty());
        assert!(config.nssm.is_none());
    }

    #[test]
    fn an_empty_file_is_an_empty_config() {
        let config = Config::parse("", Path::new(CONFIG_FILE)).expect("config");
        assert!(config.entries.is_empty());
    }

    #[test]
    fn an_upsert_replaces_the_entry_by_its_old_name() {
        let mut config = Config::default();
        let mut entry = Entry {
            name: String::from("nobs"),
            ..Entry::default()
        };
        config.upsert("", entry.clone()).expect("added");
        entry.name = String::from("obs");
        config.upsert("nobs", entry).expect("renamed");

        assert_eq!(config.entries.len(), 1);
        assert_eq!(config.entries[0].name, "obs");
    }

    #[test]
    fn an_upsert_rejects_a_name_another_entry_has() {
        let mut config = Config::default();
        for name in ["one", "two"] {
            let entry = Entry {
                name: String::from(name),
                ..Entry::default()
            };
            config.upsert("", entry).expect("added");
        }
        let entry = Entry {
            name: String::from("two"),
            ..Entry::default()
        };

        let failure = config.upsert("one", entry).expect_err("duplicate");

        assert_eq!(
            failure,
            ConfigError::Duplicate {
                name: String::from("two")
            }
        );
        assert_eq!(config.entries[0].name, "one");
    }

    #[test]
    fn an_upsert_keeps_an_entry_that_was_renamed_to_itself() {
        let mut config = Config::default();
        let entry = Entry {
            name: String::from("one"),
            ..Entry::default()
        };
        config.upsert("", entry.clone()).expect("added");

        config.upsert("one", entry).expect("updated");

        assert_eq!(config.entries.len(), 1);
    }

    #[test]
    fn a_created_config_is_read_back() {
        let path = temp_file("roundtrip");
        let config = Config::default();
        config.save(&path).expect("save");
        let read = Config::load(&path).expect("load");
        assert_eq!(read, config);
    }

    #[test]
    fn a_missing_config_is_written_and_loaded() {
        let path = temp_file("missing");
        let config = Config::load_or_create(&path);
        assert_eq!(config, Config::default());
        assert!(path.exists(), "config was not written");
    }

    #[test]
    fn a_broken_config_loads_the_defaults() {
        let path = temp_file("broken-default");
        std::fs::write(&path, "entries = [1, 2").expect("write config");
        let config = Config::load_or_create(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(config, Config::default());
    }
}
