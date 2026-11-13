#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(clippy::non_ascii_literal)]

use std::error::Error;
use std::process::ExitCode;
use std::rc::Rc;

use brul::Value;
use brul::logging::{error, info};
use brul::{command, host};

use note_obs::worker::Worker;
use note_obs::{Config, default_path, generated};

const USAGE: &str = "\
nobs: текст в источник OBS

Использование: nobs [--help]

  --help, -h   показать эту справку
";

struct AppHost {
    worker: Worker,
}

#[command]
fn submit(host: &mut AppHost, text: String) -> Result<Value, brul::Error> {
    let status = if text.trim().is_empty() {
        String::from("нечего показывать")
    } else {
        sent(host.worker.show(text), "отправляю в OBS")
    };
    Ok(Value::String(status.into()))
}

#[command]
fn clear(host: &mut AppHost) -> Result<Value, brul::Error> {
    Ok(Value::String(sent(host.worker.clear(), "очищаю").into()))
}

fn sent(result: Result<(), String>, done: &str) -> String {
    result.map_or_else(|message| message, |()| String::from(done))
}

fn presets(config: &Config) -> Value {
    let items = config
        .presets
        .iter()
        .map(|preset| Value::String(preset.as_str().into()))
        .collect();
    Value::Array(Rc::new(items))
}

host! { AppHost: submit, clear }

fn main() -> ExitCode {
    brul::logging::init();
    std::panic::set_hook(Box::new(|info| error!(panic = %info, "panic")));
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error!(%error, "nobs failed");
            eprintln!("nobs: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        println!("{USAGE}");
        return Ok(());
    }
    if let Some(argument) = arguments.first() {
        return Err(format!("unknown option '{argument}', try --help").into());
    }

    let path = default_path();
    let config = Config::load_or_create(&path)?;
    info!(path = %path.display(), target = %config.target(), "starting nobs");

    let hint = format!(
        "OBS {} · сцена {} · источник {} · Ctrl+Enter — показать",
        config.target(),
        config.scene,
        config.source
    );
    let (worker, job) = note_obs::worker::worker(config.clone());
    let host = AppHost { worker };
    let seeds = [
        ("draft", Value::String(String::new().into())),
        ("status", Value::String(String::from("idle").into())),
        ("hint", Value::String(hint.into())),
        ("presets", presets(&config)),
    ];

    brul::run_generated_with(
        generated::windows(),
        generated::script(),
        generated::names(),
        None,
        &seeds,
        Some(Box::new(host)),
        move |handle| job.start(handle),
    )
}

#[cfg(test)]
mod tests {
    use super::{Config, Value, presets};

    #[test]
    fn presets_become_a_script_array() {
        let config = Config {
            presets: vec![String::from("one"), String::from("two")],
            ..Config::default()
        };

        let Value::Array(items) = presets(&config) else {
            panic!("presets are not an array");
        };
        assert_eq!(items.len(), 2);
    }
}
