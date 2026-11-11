use std::error::Error;

use bropicker::launcher::{Launcher, Runner};
use bropicker::{BrowserEntry, Config, Settings, Theme, demo_browsers, generated};
use brul::{Host, Value};

struct Session {
    url: String,
    browsers: Vec<BrowserEntry>,
    settings: Settings,
    theme: Option<&'static str>,
}

impl Session {
    fn seeds(&self) -> Vec<(&'static str, Value)> {
        let selected = self
            .browsers
            .iter()
            .position(|browser| browser.is_default)
            .unwrap_or(0);
        vec![
            ("url", Value::String(self.url.clone().into())),
            (
                "browsers",
                Value::Array(
                    self.browsers
                        .iter()
                        .map(entry_value)
                        .collect::<Vec<_>>()
                        .into(),
                ),
            ),
            ("selected", Value::Number(selected as f64)),
            ("remember", Value::Bool(self.settings.remember_choice)),
            ("alwaysAsk", Value::Bool(self.settings.always_ask)),
        ]
    }

    fn run<R: Runner + 'static>(self, launcher: Launcher<R>) -> Result<(), Box<dyn Error>> {
        let seeds = self.seeds();
        let host = LauncherHost {
            browsers: self.browsers,
            url: self.url,
            launcher,
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

fn session(url: Option<String>, config: Config) -> Session {
    let browsers = if config.browsers.is_empty() {
        demo_browsers()
    } else {
        config.browsers.clone()
    };
    Session {
        url: url.unwrap_or_else(|| "https://github.com/".to_string()),
        browsers,
        settings: config.settings,
        theme: match config.theme {
            Theme::Light => Some("light"),
            Theme::Dark => Some("dark"),
            Theme::System => None,
        },
    }
}

struct LauncherHost<R> {
    browsers: Vec<BrowserEntry>,
    url: String,
    launcher: Launcher<R>,
}

impl<R: Runner> Host for LauncherHost<R> {
    fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, brul::Error> {
        if name != "launchBrowser" {
            return Err(brul::Error::UnknownFunction(name.to_owned()));
        }
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
                    name: name.to_owned(),
                    expected: "one",
                });
            }
        };
        let Some(browser) = self.browsers.get(index) else {
            eprintln!("launch: browser {index} is not configured");
            return Ok(Value::Null);
        };
        if let Err(error) = self.launcher.launch(browser, &self.url) {
            eprintln!("launch: {error}");
        }
        Ok(Value::Null)
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("bropicker: {error}");
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let config = Config::load_default()?;
    session(std::env::args().nth(1), config).run(Launcher::new())
}

fn entry_value(browser: &BrowserEntry) -> Value {
    let icon = if browser.emoji.is_empty() {
        browser
            .name
            .chars()
            .next()
            .map(|character| character.to_uppercase().to_string())
            .unwrap_or_default()
    } else {
        browser.emoji.clone()
    };
    Value::Object(
        vec![
            ("icon".to_string(), Value::String(icon.into())),
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
        let host = LauncherHost {
            browsers: vec![
                browser("Firefox", "firefox", true),
                browser("Chrome", "chrome", false),
            ],
            url: "https://example.com/".to_string(),
            launcher: Launcher::with_runner(runner),
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
            host.call("closeWindow", &[]),
            Err(brul::Error::UnknownFunction(name)) if name == "closeWindow"
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
            Ok(Value::Null)
        ));
        assert!(commands.borrow().is_empty());
    }

    #[test]
    fn the_browser_at_the_index_is_launched_with_the_url() {
        let (mut host, commands) = host(Ok(()));
        assert!(matches!(
            call(&mut host, &[Value::Number(1.0)]),
            Ok(Value::Null)
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
            Ok(Value::Null)
        ));
        assert_eq!(commands.borrow().len(), 1);
    }

    #[test]
    fn session_falls_back_to_the_demo_browsers() {
        let config = Config::default();
        let session = session(None, config);
        assert_eq!(session.browsers.len(), demo_browsers().len());
        assert_eq!(session.url, "https://github.com/");
    }

    #[test]
    fn session_keeps_the_configured_browsers_and_url() {
        let config = Config {
            browsers: vec![browser("Firefox", "firefox", true)],
            ..Config::default()
        };
        let session = session(Some("https://example.com/".to_string()), config);
        assert_eq!(session.browsers.len(), 1);
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
            settings: Settings {
                remember_choice: false,
                always_ask: false,
            },
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
        assert!(matches!(get("remember"), Value::Bool(false)));
        assert!(matches!(get("alwaysAsk"), Value::Bool(false)));
        assert!(matches!(get("browsers"), Value::Array(browsers) if browsers.len() == 2));
    }

    fn icon_of(value: &Value) -> String {
        let Value::Object(fields) = value else {
            panic!("an entry is an object");
        };
        let icon = fields.iter().find(|(key, _)| key == "icon");
        match icon {
            Some((_, Value::String(icon))) => icon.to_string(),
            _ => panic!("an entry carries an icon"),
        }
    }

    #[test]
    fn entry_value_uses_the_emoji_and_falls_back_to_the_initial() {
        let mut marked = browser("firefox", "firefox", true);
        marked.emoji = "\u{1f525}".to_string();
        let mut plain = browser("chrome", "chrome", false);
        plain.emoji = String::new();
        let nameless = browser("", "chrome", false);

        assert_eq!(icon_of(&entry_value(&marked)), "\u{1f525}");
        assert_eq!(icon_of(&entry_value(&plain)), "C");
        assert_eq!(icon_of(&entry_value(&nameless)), "");
    }
}
