//! Конфиг bropicker: браузеры, правила доменов, запомненный выбор.
//!
//! ```
//! use bropicker::{Config, ResolveSource};
//!
//! let config = Config::parse(
//!     r#"
//! theme = "dark"
//!
//! [[browsers]]
//! name = "Firefox"
//! path = "firefox"
//! default = true
//!
//! [[browsers]]
//! name = "Chrome"
//! path = "chrome"
//!
//! [rules]
//! "example.com" = "Chrome"
//!
//! [settings]
//! always_ask = false
//! "#,
//! )
//! .expect("config");
//!
//! let resolved = config.resolve("https://example.com/page").expect("browser");
//! assert_eq!(resolved.browser.name, "Chrome");
//! assert_eq!(resolved.source, ResolveSource::Rule);
//! assert!(resolved.auto_launch);
//! ```
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct BrowserEntry {
    pub name: String,
    pub path: String,
    #[serde(default, alias = "args")]
    pub flags: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub emoji: String,
    #[serde(default, rename = "default")]
    pub is_default: bool,
}

pub fn demo_browsers() -> Vec<BrowserEntry> {
    [
        ("Firefox", "firefox", true),
        ("Chrome", "chrome", false),
        ("Brave", "brave", false),
        ("Edge", "msedge", false),
    ]
    .into_iter()
    .map(|(name, path, is_default)| BrowserEntry {
        name: name.to_string(),
        path: path.to_string(),
        flags: String::new(),
        icon: String::new(),
        emoji: String::new(),
        is_default,
    })
    .collect()
}

const KNOWN_ICONS: &[(&str, &str)] = &[
    ("firefox", "assets/logos/firefox_48x48.png"),
    ("chrome", "assets/logos/chrome_48x48.png"),
    ("opera", "assets/logos/opera_48x48.png"),
    ("brave", "assets/logos/brave_48x48.png"),
    ("яндекс", "assets/logos/yandex_48x48.png"),
    ("yandex", "assets/logos/yandex_48x48.png"),
];

pub fn is_known_browser(name: &str) -> bool {
    KNOWN_ICONS
        .iter()
        .any(|(keyword, _)| name.to_lowercase().contains(keyword))
}

pub fn icon_for(name: &str) -> &'static str {
    let name = name.to_lowercase();
    KNOWN_ICONS
        .iter()
        .find(|(keyword, _)| name.contains(keyword))
        .map_or("assets/icons/globe.svg", |(_, icon)| icon)
}

pub fn detect_browsers() -> Vec<BrowserEntry> {
    from_registry(registered_browsers(), |path| path.exists())
}

fn from_registry(
    entries: Vec<(String, String)>,
    exists: impl Fn(&Path) -> bool,
) -> Vec<BrowserEntry> {
    let mut browsers: Vec<BrowserEntry> = Vec::new();
    for (name, command) in entries {
        let Some(path) = command_path(&command) else {
            continue;
        };
        if !exists(Path::new(path)) {
            continue;
        }
        if browsers
            .iter()
            .any(|known| known.path.eq_ignore_ascii_case(path))
        {
            continue;
        }
        browsers.push(BrowserEntry {
            name,
            path: path.to_string(),
            flags: String::new(),
            icon: String::new(),
            emoji: String::new(),
            is_default: false,
        });
    }
    browsers
}

#[cfg(windows)]
fn registered_browsers() -> Vec<(String, String)> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    let mut browsers = Vec::new();
    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let Ok(clients) = RegKey::predef(hive).open_subkey(r"SOFTWARE\Clients\StartMenuInternet")
        else {
            continue;
        };
        for client in clients.enum_keys().flatten() {
            let Ok(key) = clients.open_subkey(&client) else {
                continue;
            };
            let name: String = key.get_value("").unwrap_or(client);
            let command: String = key
                .open_subkey(r"shell\open\command")
                .and_then(|command| command.get_value(""))
                .unwrap_or_default();
            browsers.push((name, command));
        }
    }
    browsers
}

#[cfg(not(windows))]
fn registered_browsers() -> Vec<(String, String)> {
    Vec::new()
}

fn command_path(command: &str) -> Option<&str> {
    let command = command.trim();
    let path = match command.strip_prefix('"') {
        Some(quoted) => quoted.split_once('"')?.0,
        None => command.split_whitespace().next()?,
    }
    .trim();
    (!path.is_empty()).then_some(path)
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub remember_choice: bool,
    pub always_ask: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            remember_choice: true,
            always_ask: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
#[serde(default)]
pub struct Config {
    pub theme: Theme,
    pub browsers: Vec<BrowserEntry>,
    pub rules: HashMap<String, String>,
    pub remembered: HashMap<String, String>,
    pub settings: Settings,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigError {
    Read {
        path: PathBuf,
        message: String,
    },
    Parse {
        path: Option<PathBuf>,
        message: String,
    },
    Invalid(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, message } => {
                write!(
                    formatter,
                    "failed to read config {}: {message}",
                    path.display()
                )
            }
            Self::Parse { path, message } => match path {
                Some(path) => write!(
                    formatter,
                    "failed to parse config {}: {message}",
                    path.display()
                ),
                None => write!(formatter, "failed to parse config: {message}"),
            },
            Self::Invalid(message) => write!(formatter, "invalid config: {message}"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolveSource {
    Remembered,
    Rule,
    Default,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolution<'a> {
    pub domain: String,
    pub browser: &'a BrowserEntry,
    pub source: ResolveSource,
    pub auto_launch: bool,
}

impl Config {
    pub fn parse(source: &str) -> Result<Self, ConfigError> {
        Self::parse_at(source, None)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path).map_err(|error| ConfigError::Read {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

        Self::parse_at(&source, Some(path))
    }

    pub fn load_default() -> Result<Self, ConfigError> {
        Self::load_or_default(&default_path())
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let path = path.as_ref();
        let source = toml::to_string_pretty(self)
            .map_err(|error| ConfigError::Invalid(error.to_string()))?;
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|error| ConfigError::Read {
                path: parent.to_path_buf(),
                message: error.to_string(),
            })?;
        }

        std::fs::write(path, source).map_err(|error| ConfigError::Read {
            path: path.to_path_buf(),
            message: error.to_string(),
        })
    }

    pub fn save_default(&self) -> Result<(), ConfigError> {
        self.save(default_path())
    }

    fn load_or_default(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(source) => Self::parse_at(&source, Some(path)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(ConfigError::Read {
                path: path.to_path_buf(),
                message: error.to_string(),
            }),
        }
    }

    pub fn browser(&self, name: &str) -> Option<&BrowserEntry> {
        self.browsers.iter().find(|browser| browser.name == name)
    }

    pub fn default_browser(&self) -> Option<&BrowserEntry> {
        self.browsers.iter().find(|browser| browser.is_default)
    }

    pub fn add_detected(&mut self, detected: Vec<BrowserEntry>) {
        for entry in detected {
            if self
                .browsers
                .iter()
                .any(|browser| browser.path.eq_ignore_ascii_case(&entry.path))
            {
                continue;
            }
            self.browsers.push(entry);
        }
    }

    pub fn resolve<'a>(&'a self, url: &str) -> Option<Resolution<'a>> {
        let domain = domain_of(url);
        if domain.is_empty() {
            return None;
        }

        let (browser, source) = if let Some(browser) = self.lookup(&self.remembered, &domain) {
            (browser, ResolveSource::Remembered)
        } else if let Some(browser) = self.lookup(&self.rules, &domain) {
            (browser, ResolveSource::Rule)
        } else {
            let browser = self.default_browser()?;
            (browser, ResolveSource::Default)
        };

        let auto_launch = matches!(source, ResolveSource::Remembered | ResolveSource::Rule)
            && !self.settings.always_ask;

        Some(Resolution {
            domain,
            browser,
            source,
            auto_launch,
        })
    }

    fn lookup<'a>(
        &'a self,
        map: &'a HashMap<String, String>,
        domain: &str,
    ) -> Option<&'a BrowserEntry> {
        map.iter()
            .find(|(key, _)| domain_of(key) == domain)
            .and_then(|(_, name)| self.browser(name))
    }

    fn parse_at(source: &str, path: Option<&Path>) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(source).map_err(|error| ConfigError::Parse {
            path: path.map(Path::to_path_buf),
            message: error.to_string(),
        })?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        let mut names = HashSet::new();
        let mut defaults = 0;

        for browser in &self.browsers {
            if browser.name.trim().is_empty() {
                return Err(ConfigError::Invalid("browser name is empty".to_string()));
            }
            if browser.path.trim().is_empty() {
                return Err(ConfigError::Invalid(format!(
                    "browser '{}' has an empty path",
                    browser.name
                )));
            }
            if !names.insert(browser.name.as_str()) {
                return Err(ConfigError::Invalid(format!(
                    "browser '{}' is listed twice",
                    browser.name
                )));
            }
            if browser.is_default {
                defaults += 1;
            }
        }

        if defaults > 1 {
            return Err(ConfigError::Invalid(
                "default = true is allowed on one browser only".to_string(),
            ));
        }

        Ok(())
    }
}

pub fn default_path() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok();
    path_from(std::env::var("BP_CONFIG").ok().as_deref(), home.as_deref())
}

fn path_from(override_path: Option<&str>, home: Option<&str>) -> PathBuf {
    if let Some(path) = override_path {
        return PathBuf::from(path);
    }
    if let Some(home) = home {
        return PathBuf::from(home).join(".config").join("bropicker.toml");
    }
    PathBuf::from("config.toml")
}

pub fn domain_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let host = if let Some(stripped) = host.strip_prefix('[') {
        stripped.split_once(']').map_or(stripped, |(host, _)| host)
    } else {
        host.split(':').next().unwrap_or(host)
    };
    let host = host.trim_end_matches('.').to_lowercase();

    host.strip_prefix("www.").unwrap_or(&host).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!("bropicker-{name}-{unique}.toml"))
    }

    #[test]
    fn default_path_prefers_the_override() {
        assert_eq!(
            path_from(Some("picked.toml"), Some("roaming")),
            PathBuf::from("picked.toml")
        );
    }

    #[test]
    fn default_path_uses_the_home_directory_without_an_override() {
        assert_eq!(
            path_from(None, Some("profile")),
            PathBuf::from("profile")
                .join(".config")
                .join("bropicker.toml")
        );
    }

    #[test]
    fn default_path_falls_back_to_the_current_directory() {
        assert_eq!(path_from(None, None), PathBuf::from("config.toml"));
    }

    #[test]
    fn missing_file_loads_the_default_config() {
        let config = Config::load_or_default(&temp_file("missing")).expect("default config");

        assert_eq!(config, Config::default());
    }

    #[test]
    fn existing_file_is_parsed_on_load() {
        let path = temp_file("present");
        std::fs::write(&path, "theme = \"light\"\n").expect("write config");
        let config = Config::load_or_default(&path).expect("parsed config");
        let _ = std::fs::remove_file(&path);

        assert_eq!(config.theme, Theme::Light);
    }

    #[test]
    fn a_saved_config_loads_back() {
        let path = temp_file("saved");
        let config = Config {
            theme: Theme::Dark,
            browsers: vec![BrowserEntry {
                name: "Firefox".to_string(),
                path: "firefox".to_string(),
                flags: String::new(),
                icon: String::new(),
                emoji: String::new(),
                is_default: true,
            }],
            remembered: HashMap::from([("example.com".to_string(), "Firefox".to_string())]),
            ..Config::default()
        };

        config.save(&path).expect("save config");
        let loaded = Config::load(&path).expect("load config");
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded, config);
    }

    #[test]
    fn saving_creates_the_config_directory() {
        let root = temp_file("nested");
        let path = root.join(".config").join("bropicker.toml");

        Config::default().save(&path).expect("save config");

        assert!(path.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_quoted_command_yields_the_executable_path() {
        assert_eq!(
            command_path(r#""C:\Program Files\Mozilla Firefox\firefox.exe" -osint -url "%1""#),
            Some(r"C:\Program Files\Mozilla Firefox\firefox.exe")
        );
    }

    #[test]
    fn an_unquoted_command_yields_the_executable_path() {
        assert_eq!(
            command_path(r"C:\tools\app.exe --other"),
            Some(r"C:\tools\app.exe")
        );
    }

    #[test]
    fn a_command_without_an_executable_is_ignored() {
        assert_eq!(command_path(""), None);
        assert_eq!(command_path("   "), None);
        assert_eq!(command_path(r#"" %1"#), None);
    }

    #[test]
    fn a_registered_browser_keeps_its_name_and_needs_its_file() {
        let entries = vec![
            (
                "Browser".to_string(),
                r#""C:\browsers\browser.exe" "%1""#.to_string(),
            ),
            ("Gone".to_string(), r"C:\missing\gone.exe".to_string()),
            ("Nameless".to_string(), String::new()),
        ];

        let found = from_registry(entries, |path| {
            path == Path::new(r"C:\browsers\browser.exe")
        });

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Browser");
        assert_eq!(found[0].path, r"C:\browsers\browser.exe");
    }

    #[test]
    fn a_browser_registered_by_both_hives_is_listed_once() {
        let entries = vec![
            ("Firefox".to_string(), r"C:\Firefox\firefox.exe".to_string()),
            ("Firefox".to_string(), r"C:\FIREFOX\FIREFOX.EXE".to_string()),
        ];

        let found = from_registry(entries, |_| true);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, r"C:\Firefox\firefox.exe");
    }

    #[test]
    fn detected_browsers_are_added_once_by_path() {
        let mut config = Config {
            browsers: vec![BrowserEntry {
                name: "Firefox".to_string(),
                path: "C:/Firefox/firefox.exe".to_string(),
                flags: String::new(),
                icon: String::new(),
                emoji: String::new(),
                is_default: false,
            }],
            ..Config::default()
        };
        let detected = ["C:/firefox/FIREFOX.EXE", "C:/Chrome/chrome.exe"]
            .into_iter()
            .map(|path| BrowserEntry {
                name: "Detected".to_string(),
                path: path.to_string(),
                flags: String::new(),
                icon: String::new(),
                emoji: String::new(),
                is_default: false,
            })
            .collect();

        config.add_detected(detected);

        assert_eq!(config.browsers.len(), 2);
        assert_eq!(config.browsers[1].path, "C:/Chrome/chrome.exe");
    }

    #[test]
    fn known_browsers_map_to_their_logos() {
        assert_eq!(
            icon_for("Mozilla Firefox"),
            "assets/logos/firefox_48x48.png"
        );
        assert_eq!(icon_for("Google Chrome"), "assets/logos/chrome_48x48.png");
        assert_eq!(icon_for("Яндекс Браузер"), "assets/logos/yandex_48x48.png");
        assert_eq!(icon_for("Yandex"), "assets/logos/yandex_48x48.png");
        assert_eq!(icon_for("Netscape"), "assets/icons/globe.svg");
        assert!(is_known_browser("Brave"));
        assert!(!is_known_browser("Netscape"));
    }

    #[test]
    fn unreadable_path_is_reported_as_a_read_error() {
        assert!(matches!(
            Config::load_or_default(Path::new("config\0.toml")),
            Err(ConfigError::Read { .. })
        ));
    }
}
