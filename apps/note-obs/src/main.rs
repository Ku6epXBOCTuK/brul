#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(clippy::non_ascii_literal)]

use std::cell::RefCell;
use std::error::Error;
use std::process::ExitCode;
use std::rc::Rc;

use brul::Value;
use brul::logging::{error, info};
use brul::{command, host};

use note_obs::{Config, default_path, generated};

const USAGE: &str = "\
nobs: текст в источник OBS

Использование: nobs [--help]

  --help, -h   показать эту справку
";

struct State {
    status: String,
}

struct AppHost {
    state: Rc<RefCell<State>>,
}

#[command]
fn submit(host: &mut AppHost, text: String) -> Result<Value, brul::Error> {
    let status = if text.trim().is_empty() {
        "нечего показывать"
    } else {
        "OBS не подключён"
    };
    host.state.borrow_mut().status = status.to_owned();
    Ok(Value::String(status.into()))
}

#[command]
fn clear(host: &mut AppHost) -> Result<Value, brul::Error> {
    host.state.borrow_mut().status = String::from("idle");
    Ok(Value::String(String::from("idle").into()))
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
    let state = Rc::new(RefCell::new(State {
        status: String::from("idle"),
    }));
    let host = AppHost {
        state: Rc::clone(&state),
    };
    let seeds = [
        ("draft", Value::String(String::new().into())),
        ("status", Value::String(String::from("idle").into())),
        ("hint", Value::String(hint.into())),
    ];

    brul::run_generated(
        generated::windows(),
        generated::script(),
        generated::names(),
        None,
        &seeds,
        Some(Box::new(host)),
    )
}
