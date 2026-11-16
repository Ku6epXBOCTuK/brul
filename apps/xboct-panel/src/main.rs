#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;
use std::rc::Rc;

use brul::Value;
use brul::logging::{error, info};
use brul::{command, host};

use xboct_panel::{
    Config, Entry, EntryKind, Program, Service, Started, default_path, generated, log, program,
    service,
};

const LOG_LINES: usize = 40;

struct AppHost {
    specs: Vec<Entry>,
}

#[command]
fn act(host: &mut AppHost, action: String, name: String) -> Result<Value, brul::Error> {
    let status = match host.specs.iter().find(|spec| spec.name == name) {
        None => format!("no entry '{name}' in the config"),
        Some(entry) => match entry.kind {
            EntryKind::Service => match action.as_str() {
                "start" => acted(service::start(&name), &action, &name),
                "stop" => acted(service::stop(&name), &action, &name),
                "restart" => acted(service::restart(&name), &action, &name),
                other => unknown(other),
            },
            EntryKind::Program => match action.as_str() {
                "start" => acted(program::start(entry).map(|_| ()), &action, &name),
                "stop" => acted(program::stop(entry), &action, &name),
                "restart" => acted(program::restart(entry).map(|_| ()), &action, &name),
                other => unknown(other),
            },
            EntryKind::Console => format!("console '{name}' is not supported yet"),
        },
    };
    Ok(outcome(&host.specs, status))
}

fn unknown(action: &str) -> String {
    format!("unknown action '{action}'")
}

#[command]
fn start_all_manual(host: &mut AppHost) -> Result<Value, brul::Error> {
    let status = match service::start_manual(&host.specs) {
        Ok(report) => describe(&report),
        Err(failure) => failure.to_string(),
    };
    Ok(outcome(&host.specs, status))
}

#[command]
fn refresh(host: &mut AppHost, name: String) -> Result<Value, brul::Error> {
    let (ok, services) = match collect(&host.specs) {
        Ok(rows) => (true, Value::Array(Rc::new(rows))),
        Err(_) => (false, Value::Null),
    };
    Ok(Value::Object(Rc::new(vec![
        (String::from("ok"), Value::Bool(ok)),
        (String::from("services"), services),
        (String::from("log"), text(log_text(&host.specs, &name))),
    ])))
}

fn log_text(specs: &[Entry], name: &str) -> String {
    if name.is_empty() {
        return String::from("select an entry to see its log");
    }
    let Some(spec) = specs.iter().find(|spec| spec.name == name) else {
        return format!("no entry '{name}' in the config");
    };
    let Some(path) = spec.log.as_deref() else {
        return format!("no log configured for '{name}'");
    };
    match log::tail(path, LOG_LINES) {
        Ok(text) if text.trim().is_empty() => format!("{} is empty", path.display()),
        Ok(text) => text,
        Err(failure) => failure.to_string(),
    }
}

fn acted<P: std::fmt::Display>(result: Result<(), P>, action: &str, name: &str) -> String {
    result.map_or_else(
        |failure| failure.to_string(),
        |()| format!("{action}: {name}"),
    )
}

fn describe(report: &Started) -> String {
    let mut parts: Vec<String> = report
        .failed
        .iter()
        .map(|(name, failure)| format!("{name}: {failure}"))
        .collect();
    if !report.started.is_empty() {
        parts.push(format!("started: {}", report.started.join(", ")));
    }
    if parts.is_empty() {
        return String::from("no manual services to start");
    }
    parts.join("; ")
}

fn outcome(specs: &[Entry], status: String) -> Value {
    let (ok, services, status) = match collect(specs) {
        Ok(rows) => (true, Value::Array(Rc::new(rows)), status),
        Err(failure) => (
            false,
            Value::Null,
            format!("{status}; refresh failed: {failure}"),
        ),
    };
    Value::Object(Rc::new(vec![
        (String::from("ok"), Value::Bool(ok)),
        (String::from("status"), text(status)),
        (String::from("services"), services),
    ]))
}

fn collect(specs: &[Entry]) -> Result<Vec<Value>, String> {
    let services = service::query_all(specs).map_err(|failure| failure.to_string())?;
    let programs = program::query_all(specs);
    Ok(specs
        .iter()
        .filter_map(|spec| match spec.kind {
            EntryKind::Service => services
                .iter()
                .find(|service| service.name == spec.name)
                .map(row),
            EntryKind::Program => programs
                .iter()
                .find(|program| program.name == spec.name)
                .map(program_row),
            EntryKind::Console => None,
        })
        .collect())
}

fn program_row(program: &Program) -> Value {
    Value::Object(Rc::new(vec![
        (String::from("name"), text(program.name.clone())),
        (String::from("display"), text(program.display.clone())),
        (String::from("state"), text(program.state.to_string())),
        (String::from("start"), text(String::from("program"))),
        (
            String::from("port"),
            text(
                program
                    .port
                    .map_or_else(String::new, |port| port.to_string()),
            ),
        ),
    ]))
}

fn row(service: &Service) -> Value {
    Value::Object(Rc::new(vec![
        (String::from("name"), text(service.name.clone())),
        (String::from("display"), text(service.display.clone())),
        (String::from("state"), text(service.state.to_string())),
        (String::from("start"), text(service.start_type.to_string())),
        (
            String::from("port"),
            text(
                service
                    .port
                    .map_or_else(String::new, |port| port.to_string()),
            ),
        ),
    ]))
}

fn text(value: String) -> Value {
    Value::String(value.into())
}

host! { AppHost: act, start_all_manual, refresh }

fn main() -> ExitCode {
    brul::logging::init_with(&[env!("CARGO_PKG_NAME")]);
    std::panic::set_hook(Box::new(|info| error!(panic = %info, "panic")));

    info!(version = env!("CARGO_PKG_VERSION"), "panel starting");

    let path = default_path();
    let config = match Config::load_or_create(&path) {
        Ok(config) => config,
        Err(failure) => {
            error!(%failure, "config is unusable");
            eprintln!("panel: {failure}");
            return ExitCode::FAILURE;
        }
    };
    info!(
        path = %path.display(),
        entries = config.entries.len(),
        "starting panel"
    );

    let services = match collect(&config.entries) {
        Ok(rows) => Value::Array(Rc::new(rows)),
        Err(failure) => {
            error!(%failure, "entries are unreadable");
            eprintln!("panel: {failure}");
            return ExitCode::FAILURE;
        }
    };

    let seeds = [
        ("services", services),
        ("status", text(String::new())),
        ("selected", text(String::new())),
        ("log", text(log_text(&config.entries, ""))),
    ];
    let host = AppHost {
        specs: config.entries,
    };

    match brul::run_generated_with(
        generated::windows(),
        generated::script(),
        generated::names(),
        None,
        &seeds,
        Some(Box::new(host)),
        |_| {},
    ) {
        Ok(()) => {
            info!("panel closed");
            ExitCode::SUCCESS
        }
        Err(failure) => {
            error!(%failure, "panel failed");
            eprintln!("panel: {failure}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{Entry, Program, Service, Value, describe, log_text, program_row, row};
    use xboct_panel::{ProgramState, ServiceError, ServiceState, StartType, Started};

    fn sample() -> Service {
        Service {
            name: String::from("sample-service"),
            display: String::from("Sample Service"),
            state: ServiceState::Running,
            start_type: StartType::Automatic,
            port: Some(8080),
            log: None,
        }
    }

    fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
        let Value::Object(fields) = value else {
            panic!("expected an object, got {value:?}");
        };
        fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("no field '{name}'"))
    }

    #[test]
    fn a_row_carries_the_service_fields() {
        let row = row(&sample());

        assert_eq!(field(&row, "name").as_text(), Some("sample-service"));
        assert_eq!(field(&row, "display").as_text(), Some("Sample Service"));
        assert_eq!(field(&row, "state").as_text(), Some("running"));
        assert_eq!(field(&row, "start").as_text(), Some("automatic"));
        assert_eq!(field(&row, "port").as_text(), Some("8080"));
    }

    #[test]
    fn a_service_without_a_port_has_an_empty_port() {
        let service = Service {
            port: None,
            ..sample()
        };

        assert_eq!(field(&row(&service), "port").as_text(), Some(""));
    }

    #[test]
    fn a_start_report_names_what_started_and_what_failed() {
        let report = Started {
            started: vec![String::from("one")],
            failed: vec![(
                String::from("two"),
                ServiceError::AccessDenied {
                    name: String::from("two"),
                },
            )],
        };

        let text = describe(&report);

        assert!(text.contains("started: one"), "{text}");
        assert!(
            text.contains("two: access to service two is denied"),
            "{text}"
        );
    }

    #[test]
    fn a_start_report_without_work_says_so() {
        assert_eq!(describe(&Started::default()), "no manual services to start");
    }

    #[test]
    fn a_program_row_says_it_is_a_program() {
        let program = Program {
            name: String::from("caps-lang"),
            display: String::from("caps-lang"),
            state: ProgramState::Running,
            port: None,
            log: None,
            pid: Some(7),
        };

        let row = program_row(&program);

        assert_eq!(field(&row, "name").as_text(), Some("caps-lang"));
        assert_eq!(field(&row, "state").as_text(), Some("running"));
        assert_eq!(field(&row, "start").as_text(), Some("program"));
        assert_eq!(field(&row, "port").as_text(), Some(""));
    }

    fn spec(name: &str, log: Option<PathBuf>) -> Entry {
        Entry {
            name: String::from(name),
            log,
            ..Entry::default()
        }
    }

    fn written_log(name: &str, text: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time after the epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("panel-window-{name}-{stamp}.log"));
        fs::write(&path, text).expect("log file");
        path
    }

    #[test]
    fn a_log_without_a_selection_asks_for_one() {
        assert_eq!(log_text(&[], ""), "select an entry to see its log");
    }

    #[test]
    fn a_selection_without_a_log_names_the_service() {
        assert_eq!(
            log_text(&[spec("sample-service", None)], "sample-service"),
            "no log configured for 'sample-service'"
        );
    }

    #[test]
    fn a_selection_unknown_to_the_config_is_reported() {
        assert_eq!(
            log_text(&[spec("sample-service", None)], "other-service"),
            "no entry 'other-service' in the config"
        );
    }

    #[test]
    fn a_logged_selection_returns_the_tail_of_its_file() {
        let path = written_log("tail", "one\ntwo\nthree\n");
        let text = log_text(
            &[spec("sample-service", Some(path.clone()))],
            "sample-service",
        );
        assert!(text.ends_with("two\nthree"), "{text}");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn an_empty_log_names_the_path() {
        let path = written_log("empty", "");
        let text = log_text(
            &[spec("sample-service", Some(path.clone()))],
            "sample-service",
        );
        assert!(text.contains("panel-window-empty"), "{text}");
        assert!(text.ends_with("is empty"), "{text}");
        let _ = fs::remove_file(&path);
    }
}
