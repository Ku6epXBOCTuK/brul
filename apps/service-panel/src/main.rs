#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;
use std::rc::Rc;

use brul::Value;
use brul::logging::{error, info};
use brul::{command, host};

use service_panel::{
    Config, Service, ServiceError, ServiceSpec, Started, default_path, generated, log, service,
};

const LOG_LINES: usize = 40;

struct AppHost {
    specs: Vec<ServiceSpec>,
}

#[command]
fn act(host: &mut AppHost, action: String, name: String) -> Result<Value, brul::Error> {
    let status = match action.as_str() {
        "start" => acted(service::start(&name), &action, &name),
        "stop" => acted(service::stop(&name), &action, &name),
        "restart" => acted(service::restart(&name), &action, &name),
        other => format!("unknown action '{other}'"),
    };
    Ok(outcome(&host.specs, status))
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
    let (ok, services) = match service::query_all(&host.specs) {
        Ok(services) => (true, rows(&services)),
        Err(_) => (false, Value::Null),
    };
    Ok(Value::Object(Rc::new(vec![
        (String::from("ok"), Value::Bool(ok)),
        (String::from("services"), services),
        (String::from("log"), text(log_text(&host.specs, &name))),
    ])))
}

fn log_text(specs: &[ServiceSpec], name: &str) -> String {
    if name.is_empty() {
        return String::from("select a service to see its log");
    }
    let Some(spec) = specs.iter().find(|spec| spec.name == name) else {
        return format!("no service '{name}' in the config");
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

fn acted(result: Result<(), ServiceError>, action: &str, name: &str) -> String {
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

fn outcome(specs: &[ServiceSpec], status: String) -> Value {
    let (ok, services, status) = match service::query_all(specs) {
        Ok(services) => (true, rows(&services), status),
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

fn rows(services: &[Service]) -> Value {
    Value::Array(Rc::new(services.iter().map(row).collect()))
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
        services = config.services.len(),
        "starting panel"
    );

    let services = match service::query_all(&config.services) {
        Ok(services) => services,
        Err(failure) => {
            error!(%failure, "services are unreadable");
            eprintln!("panel: {failure}");
            return ExitCode::FAILURE;
        }
    };

    let seeds = [
        ("services", rows(&services)),
        ("status", text(String::new())),
        ("selected", text(String::new())),
        ("log", text(log_text(&config.services, ""))),
    ];
    let host = AppHost {
        specs: config.services,
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

    use super::{Service, ServiceSpec, Value, describe, log_text, row};
    use service_panel::{ServiceError, ServiceState, StartType, Started};

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

    fn spec(name: &str, log: Option<PathBuf>) -> ServiceSpec {
        ServiceSpec {
            name: String::from(name),
            port: None,
            log,
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
        assert_eq!(log_text(&[], ""), "select a service to see its log");
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
            "no service 'other-service' in the config"
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
