#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;
use std::rc::Rc;

use brul::Value;
use brul::logging::{error, info};
use brul::{command, host};

use service_panel::{
    Config, Service, ServiceError, ServiceSpec, Started, default_path, generated, service,
};

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

host! { AppHost: act, start_all_manual }

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
    use super::{Service, Value, describe, row};
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
}
