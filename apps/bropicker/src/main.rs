use std::error::Error;
use std::path::PathBuf;

use bropicker::launcher::Launcher;
use bropicker::{BrowserEntry, Config, Theme, demo_browsers};
use brul::{Host, Value};

#[allow(dead_code)]
mod generated;

struct LauncherHost {
    browsers: Vec<BrowserEntry>,
    url: String,
}

impl Host for LauncherHost {
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
        if let Err(error) = Launcher::new().launch(browser, &self.url) {
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
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://github.com/".to_string());
    let config = Config::load_default()?;
    let browsers = if config.browsers.is_empty() {
        demo_browsers()
    } else {
        config.browsers.clone()
    };
    let selected = browsers
        .iter()
        .position(|browser| browser.is_default)
        .unwrap_or(0);

    let seeds = [
        ("url", Value::String(url.clone())),
        (
            "browsers",
            Value::Array(browsers.iter().map(entry_value).collect()),
        ),
        ("selected", Value::Number(selected as f64)),
        ("remember", Value::Bool(config.settings.remember_choice)),
        ("alwaysAsk", Value::Bool(config.settings.always_ask)),
    ];
    let theme = match config.theme {
        Theme::Light => Some("light"),
        Theme::Dark => Some("dark"),
        Theme::System => None,
    };
    let host = LauncherHost { browsers, url };

    brul::run_file_with(picker(), theme, &seeds, Some(Box::new(host)))?;

    Ok(())
}

fn picker() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("picker.brul")
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
    Value::Object(vec![
        ("icon".to_string(), Value::String(icon)),
        ("name".to_string(), Value::String(browser.name.clone())),
        ("path".to_string(), Value::String(browser.path.clone())),
        ("isDefault".to_string(), Value::Bool(browser.is_default)),
    ])
}
