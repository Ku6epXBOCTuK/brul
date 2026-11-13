//! Конфиг note-obs: подключение к OBS и текст AFK.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(default)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub password: String,
    pub scene: String,
    pub source: String,
    pub afk_enabled: bool,
    pub afk_after_secs: u64,
    pub afk_text: String,
    pub presets: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: String::from("127.0.0.1"),
            port: 4455,
            password: String::new(),
            scene: String::from("Main"),
            source: String::from("note-text"),
            afk_enabled: true,
            afk_after_secs: 300,
            afk_text: String::from("Отошёл, скоро вернусь"),
            presets: vec![
                String::from("Стрим начинается"),
                String::from("Скоро вернусь"),
                String::from("Ушёл на обед"),
            ],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigError {
    Read { path: PathBuf, message: String },
    Parse { path: PathBuf, message: String },
    Write { path: PathBuf, message: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, message } => {
                write!(formatter, "failed to read {}: {message}", path.display())
            }
            Self::Parse { path, message } => {
                write!(formatter, "failed to parse {}: {message}", path.display())
            }
            Self::Write { path, message } => {
                write!(formatter, "failed to write {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn parse(source: &str, path: &Path) -> Result<Self, ConfigError> {
        toml::from_str(source).map_err(|error| ConfigError::Parse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path).map_err(|error| ConfigError::Read {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

        Self::parse(&source, path)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let path = path.as_ref();
        let source = toml::to_string_pretty(self).map_err(|error| ConfigError::Write {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|error| ConfigError::Write {
                path: parent.to_path_buf(),
                message: error.to_string(),
            })?;
        }

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

    pub fn target(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn save_afk(path: impl AsRef<Path>, enabled: bool, text: &str) -> Result<(), ConfigError> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path).map_err(|error| ConfigError::Read {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        let mut document: toml_edit::DocumentMut =
            source
                .parse()
                .map_err(|error: toml_edit::TomlError| ConfigError::Parse {
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?;
        document["afk_enabled"] = toml_edit::value(enabled);
        patch(&mut document, "afk_text", toml_edit::Value::from(text));
        std::fs::write(path, document.to_string()).map_err(|error| ConfigError::Write {
            path: path.to_path_buf(),
            message: error.to_string(),
        })
    }
}

fn patch(document: &mut toml_edit::DocumentMut, key: &str, value: toml_edit::Value) {
    match document.get_mut(key).and_then(|item| item.as_value_mut()) {
        Some(slot) => {
            let decor = slot.decor().clone();
            *slot = value;
            *slot.decor_mut() = decor;
        }
        None => document[key] = toml_edit::Item::Value(value),
    }
}

pub fn default_path() -> PathBuf {
    if let Ok(path) = std::env::var("NOBS_CONFIG")
        && !path.is_empty()
    {
        return PathBuf::from(path);
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("nobs_config.toml")))
        .unwrap_or_else(|| PathBuf::from("nobs_config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!("note-obs-{name}-{unique}.toml"))
    }

    #[test]
    fn default_config_points_at_the_local_obs() {
        let config = Config::default();

        assert_eq!(config.target(), "127.0.0.1:4455");
        assert_eq!(config.scene, "Main");
        assert!(config.afk_enabled);
    }

    #[test]
    fn missing_file_is_created_with_the_defaults() {
        let path = temp_file("created");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("temp dir");

        let config = Config::load_or_create(&path).expect("created config");
        let source = std::fs::read_to_string(&path).expect("template on disk");
        let _ = std::fs::remove_file(&path);

        assert_eq!(config, Config::default());
        assert!(source.contains("source = \"note-text\""));
    }

    #[test]
    fn a_saved_config_loads_back() {
        let path = temp_file("saved");
        let config = Config {
            port: 4460,
            password: String::from("secret"),
            afk_enabled: false,
            ..Config::default()
        };

        config.save(&path).expect("save config");
        let loaded = Config::load(&path).expect("load config");
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded, config);
    }

    #[test]
    fn a_broken_file_is_reported() {
        let path = temp_file("broken");
        std::fs::write(&path, "port = \"not a number\"").expect("write config");

        let error = Config::load(&path).expect_err("broken config");
        let _ = std::fs::remove_file(&path);

        assert!(matches!(error, ConfigError::Parse { .. }));
    }

    #[test]
    fn an_afk_change_keeps_the_comments() {
        let path = temp_file("afk");
        std::fs::write(
            &path,
            "# my obs\nafk_enabled = true\nafk_text = \"away\" # keep me\nport = 4455\n",
        )
        .expect("write config");

        Config::save_afk(&path, false, "back soon").expect("patch");
        let source = std::fs::read_to_string(&path).expect("patched file");
        let _ = std::fs::remove_file(&path);

        assert!(source.contains("# my obs"));
        assert!(source.contains("# keep me"));
        assert!(source.contains("afk_enabled = false"));
        assert!(source.contains("afk_text = \"back soon\""));
    }

    #[test]
    fn an_override_wins_over_the_executable_directory() {
        // NOTE: the variable is process-wide, so the test sets it and reads it alone.
        unsafe { std::env::set_var("NOBS_CONFIG", "picked.toml") };

        assert_eq!(default_path(), PathBuf::from("picked.toml"));
    }
}
