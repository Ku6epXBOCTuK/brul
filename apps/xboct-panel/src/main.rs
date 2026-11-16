#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;

use brul::Value;
use brul::logging::{error, info};
use brul::{command, host};

use xboct_panel::{
    Config, ConfigError, Console, Consoles, Entry, EntryKind, Program, Restart, Service,
    ServiceError, StartMode, Started, default_path, generated, log, nssm, program, service,
};

const LOG_LINES: usize = 40;

struct AppHost {
    specs: Vec<Entry>,
    consoles: Consoles,
    nssm: Option<PathBuf>,
    path: PathBuf,
}

#[command]
fn act(host: &mut AppHost, action: String, name: String) -> Result<Value, brul::Error> {
    let Some(entry) = host.specs.iter().find(|spec| spec.name == name).cloned() else {
        let status = format!("no entry '{name}' in the config");
        return Ok(outcome(&host.specs, &mut host.consoles, status));
    };
    let status = match entry.kind {
        EntryKind::Service => match action.as_str() {
            "start" => acted(service::start(&name), &action, &name),
            "stop" => acted(service::stop(&name), &action, &name),
            "restart" => acted(service::restart(&name), &action, &name),
            other => unknown(other),
        },
        EntryKind::Program => match action.as_str() {
            "start" => acted(program::start(&entry).map(|_| ()), &action, &name),
            "stop" => acted(program::stop(&entry), &action, &name),
            "restart" => acted(program::restart(&entry).map(|_| ()), &action, &name),
            other => unknown(other),
        },
        EntryKind::Console => match action.as_str() {
            "start" => acted(host.consoles.start(&entry), &action, &name),
            "stop" => acted(host.consoles.stop(&entry), &action, &name),
            "restart" => acted(host.consoles.restart(&entry), &action, &name),
            other => unknown(other),
        },
    };
    Ok(outcome(&host.specs, &mut host.consoles, status))
}

#[command]
fn toggle_console(host: &mut AppHost, name: String) -> Result<Value, brul::Error> {
    let status = match host
        .specs
        .iter()
        .find(|spec| spec.name == name && spec.kind == EntryKind::Console)
    {
        None => format!("no console '{name}' in the config"),
        Some(entry) => match host.consoles.toggle(entry) {
            Ok(true) => format!("console shown: {name}"),
            Ok(false) => format!("console hidden: {name}"),
            Err(failure) => failure.to_string(),
        },
    };
    Ok(outcome(&host.specs, &mut host.consoles, status))
}

fn unknown(action: &str) -> String {
    format!("unknown action '{action}'")
}

#[command]
fn entry_draft(host: &mut AppHost, name: String) -> Result<Value, brul::Error> {
    let spec = host.specs.iter().find(|spec| spec.name == name);
    let ok = name.is_empty() || spec.is_some();
    let status = if ok {
        String::new()
    } else {
        format!("no entry '{name}' in the config")
    };
    Ok(Value::Object(Rc::new(vec![
        (String::from("ok"), Value::Bool(ok)),
        (String::from("status"), text(status)),
        (String::from("draft"), draft_value(spec)),
    ])))
}

#[command]
// NOTE: the command macro takes only scalars, so the whole draft cannot be one argument
#[allow(clippy::too_many_arguments)]
fn save_entry(
    host: &mut AppHost,
    original: String,
    kind: String,
    name: String,
    command: String,
    args: String,
    workdir: String,
    env: String,
    port: String,
    log: String,
    restart: String,
    start: String,
    hidden: bool,
) -> Result<Value, brul::Error> {
    let draft = Draft {
        kind: &kind,
        name: &name,
        command: &command,
        args: &args,
        workdir: &workdir,
        env: &env,
        port: &port,
        log: &log,
        restart: &restart,
        start: &start,
        hidden,
    };
    let base = host.specs.iter().find(|spec| spec.name == original);
    let status = match draft.entry(base) {
        Err(message) => message,
        Ok(entry) => match update(host, &original, entry.clone()) {
            Err(failure) => failure.to_string(),
            Ok(()) => match provision(host, &entry) {
                Ok(()) => format!("saved: {}", entry.name),
                Err(message) => message,
            },
        },
    };
    Ok(outcome(&host.specs, &mut host.consoles, status))
}

fn provision(host: &AppHost, entry: &Entry) -> Result<(), String> {
    if entry.kind != EntryKind::Service {
        return Ok(());
    }
    let Some(nssm) = host.nssm.as_deref() else {
        return Err(String::from("nssm is not set in xboct-panel.toml"));
    };
    let exists = service::known(&entry.name).map_err(|failure| failure.to_string())?;
    nssm::apply(nssm, entry, exists).map_err(|failure| failure.to_string())
}

struct Draft<'a> {
    kind: &'a str,
    name: &'a str,
    command: &'a str,
    args: &'a str,
    workdir: &'a str,
    env: &'a str,
    port: &'a str,
    log: &'a str,
    restart: &'a str,
    start: &'a str,
    hidden: bool,
}

impl Draft<'_> {
    fn entry(&self, base: Option<&Entry>) -> Result<Entry, String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(String::from("an entry needs a name"));
        }
        let kind = match self.kind.trim() {
            "service" => EntryKind::Service,
            "program" => EntryKind::Program,
            "console" => EntryKind::Console,
            other => return Err(format!("unknown kind '{other}'")),
        };
        let mut entry = base.cloned().unwrap_or_default();
        entry.name = String::from(name);
        entry.kind = kind;
        entry.command = optional(self.command);
        entry.args = lines(self.args);
        entry.workdir = optional(self.workdir).map(PathBuf::from);
        entry.env = env_pairs(self.env)?;
        entry.port = match self.port.trim() {
            "" => None,
            value => Some(
                value
                    .parse()
                    .map_err(|_| format!("port '{value}' is not a number"))?,
            ),
        };
        entry.log = optional(self.log).map(PathBuf::from);
        entry.restart = match self.restart.trim() {
            "" | "never" => Restart::Never,
            "on-crash" => Restart::OnCrash,
            other => return Err(format!("unknown restart '{other}'")),
        };
        entry.start = match self.start.trim() {
            "" => None,
            "auto" => Some(StartMode::Auto),
            "manual" => Some(StartMode::Manual),
            other => return Err(format!("unknown start '{other}'")),
        };
        entry.hidden = self.hidden;
        Ok(entry)
    }
}

fn optional(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| String::from(text))
}

fn lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect()
}

fn env_pairs(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut pairs = BTreeMap::new();
    for line in lines(text) {
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("env line '{line}' has no '='"));
        };
        pairs.insert(String::from(key.trim()), String::from(value.trim()));
    }
    Ok(pairs)
}

fn update(host: &mut AppHost, original: &str, entry: Entry) -> Result<(), ConfigError> {
    let mut config = Config {
        nssm: host.nssm.clone(),
        entries: host.specs.clone(),
    };
    config.upsert(original, entry)?;
    config.verify()?;
    config.save(&host.path)?;
    host.specs = config.entries;
    Ok(())
}

fn draft_value(spec: Option<&Entry>) -> Value {
    let blank = Entry {
        kind: EntryKind::Program,
        hidden: false,
        ..Entry::default()
    };
    let spec = spec.unwrap_or(&blank);
    let args = spec.args.join("\n");
    let env = spec
        .env
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    let path_text = |path: Option<&PathBuf>| {
        text(path.map_or_else(String::new, |path| path.display().to_string()))
    };
    Value::Object(Rc::new(vec![
        (String::from("original"), text(spec.name.clone())),
        (String::from("kind"), text(spec.kind.to_string())),
        (String::from("name"), text(spec.name.clone())),
        (
            String::from("command"),
            text(spec.command.clone().unwrap_or_default()),
        ),
        (String::from("args"), text(args)),
        (String::from("workdir"), path_text(spec.workdir.as_ref())),
        (String::from("env"), text(env)),
        (
            String::from("port"),
            text(spec.port.map_or_else(String::new, |port| port.to_string())),
        ),
        (String::from("log"), path_text(spec.log.as_ref())),
        (String::from("restart"), text(spec.restart.to_string())),
        (
            String::from("start"),
            text(
                spec.start
                    .map_or_else(String::new, |start| start.to_string()),
            ),
        ),
        (String::from("hidden"), Value::Bool(spec.hidden)),
    ]))
}

#[command]
fn start_all_manual(host: &mut AppHost) -> Result<Value, brul::Error> {
    let status = match service::start_manual(&host.specs) {
        Ok(report) => describe(&report),
        Err(failure) => failure.to_string(),
    };
    Ok(outcome(&host.specs, &mut host.consoles, status))
}

#[command]
fn refresh(host: &mut AppHost, name: String) -> Result<Value, brul::Error> {
    let (ok, services) = match collect(&host.specs, &mut host.consoles) {
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

fn outcome(specs: &[Entry], consoles: &mut Consoles, status: String) -> Value {
    let (ok, services, status) = match collect(specs, consoles) {
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

fn collect(specs: &[Entry], consoles: &mut Consoles) -> Result<Vec<Value>, ServiceError> {
    let services = service::query_all(specs)?;
    let programs = program::query_all(specs);
    let console_rows = consoles.query(specs);
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
            EntryKind::Console => console_rows
                .iter()
                .find(|console| console.name == spec.name)
                .map(console_row),
        })
        .collect())
}

fn console_row(console: &Console) -> Value {
    Value::Object(Rc::new(vec![
        (String::from("name"), text(console.name.clone())),
        (String::from("display"), text(console.display.clone())),
        (String::from("state"), text(console.state.to_string())),
        (String::from("start"), text(String::from("console"))),
        (
            String::from("port"),
            text(
                console
                    .port
                    .map_or_else(String::new, |port| port.to_string()),
            ),
        ),
    ]))
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

host! { AppHost: act, start_all_manual, refresh, toggle_console, entry_draft, save_entry }

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

    let mut consoles = Consoles::new();
    let services = match collect(&config.entries, &mut consoles) {
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
        ("draft", draft_value(None)),
        ("dialogOpen", Value::Bool(false)),
    ];
    let host = AppHost {
        specs: config.entries,
        consoles,
        nssm: config.nssm,
        path,
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

    use super::{
        Console, Draft, Entry, Program, Restart, Service, StartMode, Value, console_row, describe,
        draft_value, log_text, program_row, row,
    };
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

    #[test]
    fn a_console_row_says_it_is_a_console() {
        let console = Console {
            name: String::from("deskflow-server"),
            display: String::from("deskflow-server"),
            state: ProgramState::Stopped,
            port: None,
            log: None,
            pid: None,
        };

        let row = console_row(&console);

        assert_eq!(field(&row, "name").as_text(), Some("deskflow-server"));
        assert_eq!(field(&row, "state").as_text(), Some("stopped"));
        assert_eq!(field(&row, "start").as_text(), Some("console"));
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
    fn a_draft_carries_the_entry_fields_and_keeps_the_display_name() {
        let base = Entry {
            name: String::from("obs"),
            display: Some(String::from("OBS Studio")),
            command: Some(String::from("C:/obs/obs64.exe")),
            ..Entry::default()
        };
        let draft = Draft {
            kind: "program",
            name: "obs",
            command: "C:/obs/obs64.exe",
            args: "--startrecording",
            workdir: "C:/obs",
            env: "OBS_PORT=4455\nOBS_DEBUG=1",
            port: "",
            log: "",
            restart: "on-crash",
            start: "auto",
            hidden: true,
        };

        let entry = draft.entry(Some(&base)).expect("entry");

        assert_eq!(entry.display.as_deref(), Some("OBS Studio"));
        assert_eq!(entry.args, ["--startrecording"]);
        assert_eq!(entry.env.get("OBS_DEBUG").map(String::as_str), Some("1"));
        assert_eq!(entry.restart, Restart::OnCrash);
        assert_eq!(entry.start, Some(StartMode::Auto));
        assert!(entry.hidden);
    }

    #[test]
    fn a_new_draft_is_a_program_with_empty_fields() {
        let draft = draft_value(None);

        assert_eq!(field(&draft, "kind").as_text(), Some("program"));
        assert_eq!(field(&draft, "name").as_text(), Some(""));
        assert_eq!(field(&draft, "restart").as_text(), Some("never"));
    }

    #[test]
    fn an_env_line_without_a_separator_is_rejected() {
        let draft = Draft {
            kind: "console",
            name: "shell",
            command: "cmd.exe",
            args: "",
            workdir: "",
            env: "BROKEN",
            port: "",
            log: "",
            restart: "never",
            start: "",
            hidden: false,
        };

        let failure = draft.entry(None).expect_err("broken env");

        assert!(failure.contains("BROKEN"), "{failure}");
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
