use std::cell::RefCell;
use std::rc::Rc;

use bropicker::{
    BrowserEntry, CommandSpec, LaunchError, Launcher, Runner, command_for, split_flags,
};

fn browser(path: &str, flags: &str) -> BrowserEntry {
    BrowserEntry {
        name: String::from("Browser"),
        path: String::from(path),
        flags: String::from(flags),
        icon: String::new(),
        emoji: String::new(),
        is_default: false,
    }
}

struct Recorder {
    commands: Rc<RefCell<Vec<CommandSpec>>>,
    result: Result<(), LaunchError>,
}

impl Recorder {
    fn ok() -> Self {
        Self {
            commands: Rc::new(RefCell::new(Vec::new())),
            result: Ok(()),
        }
    }

    fn failing() -> Self {
        Self {
            commands: Rc::new(RefCell::new(Vec::new())),
            result: Err(LaunchError::Spawn(String::from("boom"))),
        }
    }
}

impl Runner for Recorder {
    fn run(&self, command: &CommandSpec) -> Result<(), LaunchError> {
        self.commands.borrow_mut().push(command.clone());
        self.result.clone()
    }
}

#[test]
fn split_flags_handles_quotes_and_spaces() {
    assert_eq!(
        split_flags(r#"--profile-directory="Profile 1" -P "My Work""#),
        vec![
            String::from("--profile-directory=Profile 1"),
            String::from("-P"),
            String::from("My Work"),
        ]
    );
}

#[test]
fn split_flags_supports_single_quotes() {
    assert_eq!(
        split_flags("--name 'Work profile'"),
        vec![String::from("--name"), String::from("Work profile")]
    );
}

#[test]
fn split_flags_ignores_extra_spaces() {
    assert_eq!(
        split_flags("  --one   --two  "),
        vec![String::from("--one"), String::from("--two")]
    );
    assert!(split_flags("").is_empty());
}

#[test]
fn command_puts_flags_before_url() {
    let browser = browser(
        "C:/Browser/browser.exe",
        r#"--profile-directory="Work" --new-window"#,
    );

    let command = command_for(&browser, "https://example.com");

    assert_eq!(command.program, "C:/Browser/browser.exe");
    assert_eq!(
        command.args,
        vec![
            String::from("--profile-directory=Work"),
            String::from("--new-window"),
            String::from("https://example.com"),
        ]
    );
}

#[test]
fn command_without_flags_contains_only_url() {
    let browser = browser("browser", "");

    let command = command_for(&browser, "https://example.com");

    assert_eq!(command.args, vec![String::from("https://example.com")]);
}

#[test]
fn program_path_with_spaces_is_one_value() {
    let browser = browser(r"C:\Program Files\Browser\browser.exe", "");

    let command = command_for(&browser, "https://example.com");

    assert_eq!(command.program, r"C:\Program Files\Browser\browser.exe");
}

#[test]
fn launcher_passes_command_to_runner() {
    let recorder = Recorder::ok();
    let commands = Rc::clone(&recorder.commands);
    let launcher = Launcher::with_runner(recorder);
    let browser = browser("browser", "-private");

    launcher
        .launch(&browser, "https://example.com")
        .expect("launch");

    let commands = commands.borrow();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].program, "browser");
    assert_eq!(
        commands[0].args,
        vec![
            String::from("-private"),
            String::from("https://example.com")
        ]
    );
}

#[test]
fn launcher_propagates_runner_error() {
    let launcher = Launcher::with_runner(Recorder::failing());
    let browser = browser("browser", "");

    let error = launcher
        .launch(&browser, "https://example.com")
        .expect_err("error");

    assert_eq!(error, LaunchError::Spawn(String::from("boom")));
}

#[test]
fn launcher_rejects_empty_path_and_url() {
    let launcher = Launcher::with_runner(Recorder::ok());

    assert_eq!(
        launcher.launch(&browser("", ""), "https://example.com"),
        Err(LaunchError::EmptyPath)
    );
    assert_eq!(
        launcher.launch(&browser("browser", ""), "   "),
        Err(LaunchError::EmptyUrl)
    );
}

#[test]
fn launch_errors_render_distinct_messages() {
    let errors = [
        LaunchError::EmptyPath,
        LaunchError::EmptyUrl,
        LaunchError::Spawn(String::from("boom")),
    ];

    let mut messages: Vec<String> = errors.iter().map(|error| error.to_string()).collect();

    assert!(messages.iter().all(|message| !message.is_empty()));
    messages.sort();
    messages.dedup();
    assert_eq!(messages.len(), errors.len());
}
