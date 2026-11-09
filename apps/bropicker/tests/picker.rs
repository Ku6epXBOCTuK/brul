use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use brul::{Error, Host, Ui, Value};
use brul_compiler::load_file_with;
use brul_gui::window::WindowCommand;
use brul_gui::{Size, TextEngine, WidgetEvent};

struct LaunchRecorder {
    calls: Rc<RefCell<Vec<f64>>>,
}

impl Host for LaunchRecorder {
    fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, Error> {
        if name != "launchBrowser" {
            return Err(Error::UnknownFunction(name.to_owned()));
        }
        let [Value::Number(index)] = arguments else {
            return Err(Error::Argument {
                name: "launchBrowser",
                expected: "a browser index",
                found: arguments.first().map_or("nothing", Value::type_name),
            });
        };
        self.calls.borrow_mut().push(*index);
        Ok(Value::Null)
    }
}

fn picker() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("picker.brul")
}

fn seeds(selected: f64) -> Vec<(&'static str, Value)> {
    let browser = |icon: &str, name: &str, path: &str, is_default: bool| {
        Value::Object(vec![
            ("icon".to_string(), Value::String(icon.to_string())),
            ("name".to_string(), Value::String(name.to_string())),
            ("path".to_string(), Value::String(path.to_string())),
            ("isDefault".to_string(), Value::Bool(is_default)),
        ])
    };
    vec![
        ("url", Value::String("https://example.com/".to_string())),
        (
            "browsers",
            Value::Array(vec![
                browser("F", "Firefox", "firefox", true),
                browser("C", "Chrome", "chrome", false),
                browser("B", "Brave", "brave", false),
            ]),
        ),
        ("selected", Value::Number(selected)),
        ("remember", Value::Bool(true)),
        ("alwaysAsk", Value::Bool(false)),
    ]
}

#[test]
fn picker_builds_from_seeded_config() {
    let loaded = load_file_with(picker(), None, &seeds(1.0)).expect("picker loads");

    assert_eq!(loaded.config.title, "bropicker");
    assert_eq!((loaded.config.width, loaded.config.height), (488, 800));
    assert_eq!(
        loaded.component.borrow().read("selectedIndex"),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn launch_selects_the_row_and_calls_the_host() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let loaded = load_file_with(picker(), None, &seeds(0.0)).expect("picker loads");
    let ui = Ui::with_host(
        loaded,
        Some(Box::new(LaunchRecorder {
            calls: Rc::clone(&calls),
        })),
    );

    ui.component
        .borrow_mut()
        .call_function("launch", vec![Value::Number(2.0)])
        .expect("launch runs");

    assert_eq!(*calls.borrow(), vec![2.0]);
    assert_eq!(
        ui.component.borrow().read("selectedIndex"),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn checkbox_handlers_flip_the_seeded_values() {
    let loaded = load_file_with(picker(), None, &seeds(0.0)).expect("picker loads");
    let ui = Ui::new(loaded);

    ui.component
        .borrow_mut()
        .call_function("toggleRemember", Vec::new())
        .expect("remember toggles");
    ui.component
        .borrow_mut()
        .call_function("toggleAlwaysAsk", Vec::new())
        .expect("always ask toggles");

    assert_eq!(
        ui.component.borrow().read("rememberChoice"),
        Ok(Value::Bool(false))
    );
    assert_eq!(
        ui.component.borrow().read("alwaysAskChoice"),
        Ok(Value::Bool(true))
    );
}

#[test]
fn clicking_a_row_selects_it_and_launches() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let loaded = load_file_with(picker(), None, &seeds(0.0)).expect("picker loads");
    let mut ui = Ui::with_host(
        loaded,
        Some(Box::new(LaunchRecorder {
            calls: Rc::clone(&calls),
        })),
    );
    let mut engine = TextEngine::new();
    ui.tree
        .borrow_mut()
        .layout(
            &mut engine,
            Size {
                width: 488,
                height: 800,
            },
            1.0,
        )
        .expect("layout");
    let snapshot = ui.tree.borrow_mut().snapshot().expect("snapshot").clone();
    let mut rows: Vec<_> = snapshot
        .nodes
        .iter()
        .filter(|node| node.style.radius == 14)
        .collect();
    rows.sort_by_key(|node| node.y);
    let row = rows.get(1).expect("two browser rows");
    let x = row.x + row.width as i32 / 2;
    let y = row.y + row.height as i32 / 2;
    let events = {
        let mut tree = ui.tree.borrow_mut();
        tree.pointer_down(x, y).expect("press");
        tree.pointer_up(x, y).expect("release")
    };

    ui.dispatch(&events);

    assert_eq!(*calls.borrow(), vec![1.0]);
    assert_eq!(
        ui.component.borrow().read("selectedIndex"),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn escape_closes_the_picker() {
    let loaded = load_file_with(picker(), None, &seeds(0.0)).expect("picker loads");
    let mut ui = Ui::new(loaded);

    let commands = ui.dispatch(&[WidgetEvent::Key("Escape".to_string())]);

    assert_eq!(commands, vec![WindowCommand::Exit]);
}
