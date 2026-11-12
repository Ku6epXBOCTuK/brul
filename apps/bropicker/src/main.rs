#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::cell::RefCell;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use bropicker::launcher::{Launcher, Runner};
use bropicker::{
    BrowserEntry, Config, Theme, config::icon_for, config::is_known_browser, default_path,
    demo_browsers, detect_browsers, domain_of, generated,
};
use brul::logging::{error, info};
use brul::{Host, Value};

struct Session {
    url: String,
    config: Rc<RefCell<Config>>,
    theme: Option<&'static str>,
}

impl Session {
    fn seeds(&self) -> Vec<(&'static str, Value)> {
        let config = self.config.borrow();
        let selected = config
            .browsers
            .iter()
            .position(|browser| browser.is_default)
            .unwrap_or(0);
        vec![
            ("url", Value::String(self.url.clone().into())),
            ("domain", Value::String(domain_of(&self.url).into())),
            ("browsers", browsers_value(&config)),
            ("remembered", remembered_value(&config)),
            ("selected", Value::Number(selected as f64)),
            ("remember", Value::Bool(config.settings.remember_choice)),
            ("alwaysAsk", Value::Bool(config.settings.always_ask)),
        ]
    }

    fn run<R: Runner + 'static>(self, launcher: Launcher<R>) -> Result<(), Box<dyn Error>> {
        let seeds = self.seeds();
        let host = LauncherHost {
            config: Rc::clone(&self.config),
            url: self.url,
            launcher,
            path: Some(default_path()),
        };
        brul::run_generated(
            generated::windows(),
            generated::script(),
            generated::names(),
            self.theme,
            &seeds,
            Some(Box::new(host)),
        )
    }
}

fn session(url: Option<String>, mut config: Config) -> Session {
    if config.browsers.is_empty() {
        config.browsers = demo_browsers();
    }
    let theme = match config.theme {
        Theme::Light => Some("light"),
        Theme::Dark => Some("dark"),
        Theme::System => None,
    };
    Session {
        url: url.unwrap_or_else(|| "https://github.com/".to_string()),
        config: Rc::new(RefCell::new(config)),
        theme,
    }
}

struct LauncherHost<R> {
    config: Rc<RefCell<Config>>,
    url: String,
    launcher: Launcher<R>,
    path: Option<PathBuf>,
}

impl<R: Runner> LauncherHost<R> {
    fn save(&self) {
        let Some(path) = &self.path else {
            return;
        };
        if let Err(error) = self.config.borrow().save(path) {
            error!(%error, "config save failed");
        }
    }

    fn launch(&mut self, arguments: &[Value]) -> Result<Value, brul::Error> {
        let index = match arguments {
            [Value::Number(index)] if index.fract() == 0.0 && *index >= 0.0 => *index as usize,
            [value] => {
                return Err(brul::Error::Argument {
                    name: "launchBrowser",
                    expected: "a browser index",
                    found: value.type_name(),
                });
            }
            _ => {
                return Err(brul::Error::Arity {
                    name: "launchBrowser".to_owned(),
                    expected: "one",
                });
            }
        };
        let browser = self.config.borrow().browsers.get(index).cloned();
        let Some(browser) = browser else {
            error!(index, "browser is not configured");
            return Ok(Value::Bool(false));
        };
        info!(browser = %browser.name, domain = %domain_of(&self.url), "launching");
        if let Err(error) = self.launcher.launch(&browser, &self.url) {
            error!(browser = %browser.name, %error, "browser launch failed");
            return Ok(Value::Bool(false));
        }
        Ok(Value::Bool(true))
    }

    fn remove_browser(&mut self, arguments: &[Value]) -> Result<Value, brul::Error> {
        let name = text(arguments, "removeBrowser")?;
        info!(browser = %name, "removing browser");
        self.config
            .borrow_mut()
            .browsers
            .retain(|browser| browser.name != name);
        self.save();
        Ok(browsers_value(&self.config.borrow()))
    }

    fn remove_remembered(&mut self, arguments: &[Value]) -> Result<Value, brul::Error> {
        let domain = text(arguments, "removeRemembered")?;
        info!(%domain, "removing a remembered domain");
        self.config.borrow_mut().remembered.remove(domain);
        self.save();
        Ok(remembered_value(&self.config.borrow()))
    }

    fn detect(&mut self, arguments: &[Value]) -> Result<Value, brul::Error> {
        if !arguments.is_empty() {
            return Err(brul::Error::Arity {
                name: "detectBrowsers".to_owned(),
                expected: "no",
            });
        }
        let detected = detect_browsers();
        info!(count = detected.len(), "detected browsers");
        self.config.borrow_mut().add_detected(detected);
        self.save();
        Ok(browsers_value(&self.config.borrow()))
    }
}

impl<R: Runner> Host for LauncherHost<R> {
    fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, brul::Error> {
        match name {
            "launchBrowser" => self.launch(arguments),
            "removeBrowser" => self.remove_browser(arguments),
            "removeRemembered" => self.remove_remembered(arguments),
            "detectBrowsers" => self.detect(arguments),
            _ => Err(brul::Error::UnknownFunction(name.to_owned())),
        }
    }
}

fn text<'a>(arguments: &'a [Value], name: &'static str) -> Result<&'a str, brul::Error> {
    match arguments {
        [Value::String(value)] => Ok(value),
        [value] => Err(brul::Error::Argument {
            name,
            expected: "String",
            found: value.type_name(),
        }),
        _ => Err(brul::Error::Arity {
            name: name.to_owned(),
            expected: "one",
        }),
    }
}

fn browsers_value(config: &Config) -> Value {
    Value::Array(
        config
            .browsers
            .iter()
            .map(entry_value)
            .collect::<Vec<_>>()
            .into(),
    )
}

fn remembered_value(config: &Config) -> Value {
    let mut entries: Vec<(&String, &String)> = config.remembered.iter().collect();
    entries.sort_by(|left, right| left.0.cmp(right.0));
    Value::Array(
        entries
            .into_iter()
            .map(|(domain, browser)| {
                Value::Object(
                    vec![
                        ("domain".to_string(), Value::String(domain.as_str().into())),
                        (
                            "browser".to_string(),
                            Value::String(browser.as_str().into()),
                        ),
                    ]
                    .into(),
                )
            })
            .collect::<Vec<_>>()
            .into(),
    )
}

fn main() {
    brul::logging::init();
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        error!(panic = %info, "panic");
        default(info);
    }));
    if let Err(error) = run() {
        error!(%error, "bropicker failed");
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let config = Config::load_default()?;
    session(std::env::args().nth(1), config).run(Launcher::new())
}

fn entry_value(browser: &BrowserEntry) -> Value {
    let icon = if browser.icon.is_empty() {
        asset_path(icon_for(&browser.name))
    } else {
        browser.icon.clone()
    };
    let masked =
        browser.icon.is_empty() && browser.emoji.is_empty() && !is_known_browser(&browser.name);
    Value::Object(
        vec![
            ("icon".to_string(), Value::Image(icon.into())),
            (
                "emoji".to_string(),
                Value::String(browser.emoji.clone().into()),
            ),
            ("masked".to_string(), Value::Bool(masked)),
            (
                "name".to_string(),
                Value::String(browser.name.clone().into()),
            ),
            (
                "path".to_string(),
                Value::String(browser.path.clone().into()),
            ),
            ("isDefault".to_string(), Value::Bool(browser.is_default)),
        ]
        .into(),
    )
}

fn asset_path(relative: &str) -> String {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    resolve_asset(Path::new(relative), &exe_dir, manifest_dir())
        .display()
        .to_string()
}

fn resolve_asset(relative: &Path, exe_dir: &Path, source_dir: Option<&Path>) -> PathBuf {
    let installed = exe_dir.join(relative);
    if installed.is_file() {
        return installed;
    }
    if let Some(source_dir) = source_dir {
        let source = source_dir.join(relative);
        if source.is_file() {
            return source;
        }
    }
    installed
}

#[cfg(debug_assertions)]
fn manifest_dir() -> Option<&'static Path> {
    Some(Path::new(env!("CARGO_MANIFEST_DIR")))
}

#[cfg(not(debug_assertions))]
fn manifest_dir() -> Option<&'static Path> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use bropicker::{CommandSpec, LaunchError};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Recording {
        commands: Rc<RefCell<Vec<CommandSpec>>>,
        outcome: Result<(), LaunchError>,
    }

    impl Runner for Recording {
        fn run(&self, command: &CommandSpec) -> Result<(), LaunchError> {
            self.commands.borrow_mut().push(command.clone());
            self.outcome.clone()
        }
    }

    fn browser(name: &str, path: &str, is_default: bool) -> BrowserEntry {
        BrowserEntry {
            name: name.to_string(),
            path: path.to_string(),
            flags: String::new(),
            icon: String::new(),
            emoji: String::new(),
            is_default,
        }
    }

    fn host(
        outcome: Result<(), LaunchError>,
    ) -> (LauncherHost<Recording>, Rc<RefCell<Vec<CommandSpec>>>) {
        let commands = Rc::new(RefCell::new(Vec::new()));
        let runner = Recording {
            commands: Rc::clone(&commands),
            outcome,
        };
        let config = Config {
            browsers: vec![
                browser("Firefox", "firefox", true),
                browser("Chrome", "chrome", false),
            ],
            ..Config::default()
        };
        let host = LauncherHost {
            config: Rc::new(RefCell::new(config)),
            url: "https://example.com/".to_string(),
            launcher: Launcher::with_runner(runner),
            path: None,
        };
        (host, commands)
    }

    fn call(host: &mut LauncherHost<Recording>, arguments: &[Value]) -> Result<Value, brul::Error> {
        host.call("launchBrowser", arguments)
    }

    #[test]
    fn an_unknown_command_is_rejected() {
        let (mut host, _) = host(Ok(()));
        assert!(matches!(
            host.call("theme", &[]),
            Err(brul::Error::UnknownFunction(name)) if name == "theme"
        ));
    }

    #[test]
    fn removing_a_browser_updates_the_config_and_the_list() {
        let (mut host, _) = host(Ok(()));

        let value = host
            .call("removeBrowser", &[Value::String("Chrome".into())])
            .expect("removed");
        let Value::Array(items) = value else {
            panic!("the list is an array");
        };
        assert_eq!(items.len(), 1);
        assert_eq!(host.config.borrow().browsers.len(), 1);
    }

    #[test]
    fn removing_an_unknown_browser_changes_nothing() {
        let (mut host, _) = host(Ok(()));

        let value = host
            .call("removeBrowser", &[Value::String("Netscape".into())])
            .expect("removed");
        assert!(matches!(value, Value::Array(items) if items.len() == 2));
        assert_eq!(host.config.borrow().browsers.len(), 2);
    }

    #[test]
    fn a_remembered_domain_can_be_removed() {
        let (mut host, _) = host(Ok(()));
        host.config
            .borrow_mut()
            .remembered
            .insert("example.com".to_string(), "Chrome".to_string());

        let value = host
            .call("removeRemembered", &[Value::String("example.com".into())])
            .expect("removed");

        assert!(matches!(value, Value::Array(items) if items.is_empty()));
        assert!(host.config.borrow().remembered.is_empty());
    }

    #[test]
    fn detection_takes_no_arguments() {
        let (mut host, _) = host(Ok(()));
        assert!(matches!(
            host.call("detectBrowsers", &[Value::Number(1.0)]),
            Err(brul::Error::Arity { expected: "no", .. })
        ));
    }

    #[test]
    fn the_command_takes_exactly_one_argument() {
        let (mut host, _) = host(Ok(()));
        assert!(matches!(
            call(&mut host, &[]),
            Err(brul::Error::Arity {
                expected: "one",
                ..
            })
        ));
        assert!(matches!(
            call(&mut host, &[Value::Number(0.0), Value::Number(1.0)]),
            Err(brul::Error::Arity {
                expected: "one",
                ..
            })
        ));
    }

    #[test]
    fn an_argument_that_is_not_a_browser_index_is_rejected() {
        let (mut host, _) = host(Ok(()));
        for argument in [
            Value::String("zero".into()),
            Value::Number(1.5),
            Value::Number(-1.0),
            Value::Number(f64::NAN),
        ] {
            assert!(
                matches!(
                    call(&mut host, std::slice::from_ref(&argument)),
                    Err(brul::Error::Argument { found, .. }) if found == argument.type_name()
                ),
                "{argument:?} was accepted"
            );
        }
    }

    #[test]
    fn a_missing_browser_launches_nothing() {
        let (mut host, commands) = host(Ok(()));
        assert!(matches!(
            call(&mut host, &[Value::Number(9.0)]),
            Ok(Value::Bool(false))
        ));
        assert!(commands.borrow().is_empty());
    }

    #[test]
    fn the_browser_at_the_index_is_launched_with_the_url() {
        let (mut host, commands) = host(Ok(()));
        assert!(matches!(
            call(&mut host, &[Value::Number(1.0)]),
            Ok(Value::Bool(true))
        ));
        let commands = commands.borrow();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].program, "chrome");
        assert_eq!(commands[0].args, ["https://example.com/".to_string()]);
    }

    #[test]
    fn a_failed_launch_does_not_escape() {
        let (mut host, commands) = host(Err(LaunchError::EmptyUrl));
        assert!(matches!(
            call(&mut host, &[Value::Number(0.0)]),
            Ok(Value::Bool(false))
        ));
        assert_eq!(commands.borrow().len(), 1);
    }

    #[test]
    fn session_falls_back_to_the_demo_browsers() {
        let config = Config::default();
        let session = session(None, config);
        assert_eq!(
            session.config.borrow().browsers.len(),
            demo_browsers().len()
        );
        assert_eq!(session.url, "https://github.com/");
    }

    #[test]
    fn session_keeps_the_configured_browsers_and_url() {
        let config = Config {
            browsers: vec![browser("Firefox", "firefox", true)],
            ..Config::default()
        };
        let session = session(Some("https://example.com/".to_string()), config);
        assert_eq!(session.config.borrow().browsers.len(), 1);
        assert_eq!(session.url, "https://example.com/");
    }

    #[test]
    fn session_maps_the_theme() {
        let themes = [
            (Theme::Light, Some("light")),
            (Theme::Dark, Some("dark")),
            (Theme::System, None),
        ];
        for (theme, expected) in themes {
            let config = Config {
                theme,
                ..Config::default()
            };
            assert_eq!(session(None, config).theme, expected);
        }
    }

    #[test]
    fn the_selected_seed_points_at_the_default_browser() {
        let config = Config {
            browsers: vec![
                browser("Firefox", "firefox", false),
                browser("Chrome", "chrome", true),
            ],
            settings: bropicker::Settings {
                remember_choice: false,
                always_ask: false,
            },
            remembered: std::collections::HashMap::from([(
                "example.com".to_string(),
                "Chrome".to_string(),
            )]),
            ..Config::default()
        };
        let seeds = session(None, config).seeds();
        let get = |name: &str| {
            seeds
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.clone())
                .expect("seed")
        };
        assert!(matches!(get("selected"), Value::Number(selected) if selected == 1.0));
        assert!(matches!(get("domain"), Value::String(domain) if domain.as_ref() == "github.com"));
        assert!(matches!(get("remember"), Value::Bool(false)));
        assert!(matches!(get("alwaysAsk"), Value::Bool(false)));
        assert!(matches!(get("browsers"), Value::Array(browsers) if browsers.len() == 2));
        let remembered = get("remembered");
        assert!(
            matches!(remembered, Value::Array(items) if items.iter().any(|item| matches!(item, Value::Object(fields) if fields.iter().any(|(key, value)| key == "domain" && matches!(value, Value::String(domain) if &**domain == "example.com")))))
        );
    }

    fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
        let Value::Object(fields) = value else {
            panic!("an entry is an object");
        };
        fields
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
            .expect("field")
    }

    fn icon_of(value: &Value) -> String {
        match field(value, "icon") {
            Value::Image(path) => path.to_string(),
            other => panic!("an entry carries an image, found {}", other.type_name()),
        }
    }

    #[test]
    fn assets_are_resolved_next_to_the_executable_with_a_source_fallback() {
        let root = std::env::temp_dir().join(format!("bropicker-assets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let exe = root.join("installed");
        let source = root.join("source");
        let relative = Path::new("assets/logos/firefox_48x48.png");
        let installed = exe.join(relative);
        let fallback = source.join(relative);
        std::fs::create_dir_all(installed.parent().expect("parent")).expect("installed dir");
        std::fs::create_dir_all(fallback.parent().expect("parent")).expect("source dir");
        std::fs::write(&installed, b"installed").expect("installed asset");
        std::fs::write(&fallback, b"source").expect("source asset");

        assert_eq!(resolve_asset(relative, &exe, Some(&source)), installed);

        std::fs::remove_file(&installed).expect("installed asset");
        assert_eq!(resolve_asset(relative, &exe, Some(&source)), fallback);

        std::fs::remove_file(&fallback).expect("source asset");
        assert_eq!(resolve_asset(relative, &exe, Some(&source)), installed);
        std::fs::remove_dir_all(root).expect("scratch directory");
    }

    #[test]
    fn entry_value_maps_known_browsers_to_their_logos() {
        let firefox = browser("Firefox", "firefox", true);
        let netscape = browser("Netscape", "netscape", false);

        let firefox = entry_value(&firefox);
        assert_eq!(
            icon_of(&firefox),
            asset_path("assets/logos/firefox_48x48.png")
        );
        assert_eq!(field(&firefox, "masked"), &Value::Bool(false));

        let netscape = entry_value(&netscape);
        assert_eq!(icon_of(&netscape), asset_path("assets/icons/globe.svg"));
        assert_eq!(field(&netscape, "masked"), &Value::Bool(true));
    }

    #[test]
    fn entry_value_keeps_a_custom_icon_and_an_emoji() {
        let mut custom = browser("Custom", "custom", false);
        custom.icon = "C:/icons/custom.png".to_string();
        let mut marked = browser("Netscape", "netscape", false);
        marked.emoji = "\u{1f525}".to_string();

        let custom = entry_value(&custom);
        assert_eq!(icon_of(&custom), "C:/icons/custom.png");
        assert_eq!(field(&custom, "masked"), &Value::Bool(false));

        let marked = entry_value(&marked);
        assert_eq!(field(&marked, "emoji"), &Value::String("\u{1f525}".into()));
        assert_eq!(field(&marked, "masked"), &Value::Bool(false));
    }
}
