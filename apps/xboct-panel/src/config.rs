//! Panel config: which services to show, their ports and log files.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub services: Vec<ServiceSpec>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            services: vec![
                ServiceSpec {
                    name: String::from("Spooler"),
                    ..ServiceSpec::default()
                },
                ServiceSpec {
                    name: String::from("wuauserv"),
                    ..ServiceSpec::default()
                },
            ],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ServiceSpec {
    pub name: String,
    pub port: Option<u16>,
    pub log: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read {}: {message}", .path.display())]
    Read { path: PathBuf, message: String },
    #[error("failed to parse {}: {message}", .path.display())]
    Parse { path: PathBuf, message: String },
    #[error("failed to write {}: {message}", .path.display())]
    Write { path: PathBuf, message: String },
}

impl Config {
    pub fn parse(source: &str, path: &Path) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(source).map_err(|error| ConfigError::Parse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        let unnamed = config
            .services
            .iter()
            .position(|service| service.name.trim().is_empty());
        match unnamed {
            Some(index) => Err(ConfigError::Parse {
                path: path.to_path_buf(),
                message: format!("services[{index}] has no name"),
            }),
            None => Ok(config),
        }
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

    pub fn load_or_create(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        match std::fs::read_to_string(path) {
            Ok(source) => Self::parse(&source, path),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let config = Self::default();
                config.save(path)?;
                Ok(config)
            }
            Err(error) => Err(ConfigError::Read {
                path: path.to_path_buf(),
                message: error.to_string(),
            }),
        }
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.services.iter().map(|service| service.name.as_str())
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
            [[services]]
            name = "sample-service"
            port = 8080
            log = 'C:\Logs\sample-service.log'
        "#;
        let config = Config::parse(source, Path::new(CONFIG_FILE)).expect("config");
        let service = &config.services[0];
        assert_eq!(service.name, "sample-service");
        assert_eq!(service.port, Some(8080));
        assert_eq!(
            service.log.as_deref(),
            Some(Path::new(r"C:\Logs\sample-service.log"))
        );
    }

    #[test]
    fn a_service_without_port_or_log_is_read() {
        let config = Config::parse("[[services]]\nname = \"Spooler\"\n", Path::new("p.toml"))
            .expect("config");
        let service = &config.services[0];
        assert_eq!(service.port, None);
        assert_eq!(service.log, None);
    }

    #[test]
    fn an_unknown_key_is_a_parse_error_with_the_line() {
        let source = "[[services]]\nname = \"Spooler\"\nprot = 631\n";
        let failure = Config::parse(source, Path::new(CONFIG_FILE)).expect_err("error");
        let message = failure.to_string();
        assert!(message.contains(CONFIG_FILE), "{message}");
        assert!(message.contains("line 3"), "{message}");
    }

    #[test]
    fn a_service_without_a_name_is_rejected() {
        let source = "[[services]]\nport = 631\n";
        let failure = Config::parse(source, Path::new(CONFIG_FILE)).expect_err("error");
        let message = failure.to_string();
        assert!(message.contains("services[0] has no name"), "{message}");
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
        let config = Config::load_or_create(&path).expect("config");
        assert_eq!(config, Config::default());
        assert!(path.exists(), "config was not written");
    }
}
