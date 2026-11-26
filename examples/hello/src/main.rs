use std::process::ExitCode;

use brul::Value;
use brul::{command, host};

mod generated;

struct HelloHost {
    greeted: usize,
}

#[command]
fn greet(host: &mut HelloHost, name: &str) -> Result<Value, brul::Error> {
    host.greeted += 1;
    let name = name.trim();
    let greeting = if name.is_empty() {
        format!("Hello! (greeting #{})", host.greeted)
    } else {
        format!("Hello, {name}! (greeting #{})", host.greeted)
    };
    Ok(Value::String(greeting.into()))
}

host! { HelloHost: greet }

fn main() -> ExitCode {
    brul::logging::init();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("hello: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let seeds = [
        ("name", Value::String("world".into())),
        ("greeting", Value::String(String::new().into())),
    ];
    let backend = match std::env::var("BRUL_EXAMPLE_BACKEND") {
        Ok(value) => Some(value.parse::<brul::Backend>()?),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => return Err(error.into()),
    };
    let mut windows = generated::windows();
    if let Some(backend) = backend {
        for window in &mut windows {
            window.config.backend = backend;
        }
    }
    brul::run_generated(
        windows,
        generated::script(),
        generated::names(),
        None,
        &seeds,
        Some(Box::new(HelloHost { greeted: 0 })),
    )
}

#[cfg(test)]
mod tests {
    use super::{HelloHost, generated};
    use brul::{Component, Host, Value};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn greet(name: &str) -> String {
        let mut host = HelloHost { greeted: 0 };
        let value = host
            .call("greet", &[Value::String(name.into())])
            .expect("command");
        match value {
            Value::String(text) => text.to_string(),
            other => panic!("expected a string, got {other:?}"),
        }
    }

    #[test]
    fn the_greeting_names_the_guest() {
        assert_eq!(greet("Ada"), "Hello, Ada! (greeting #1)");
    }

    #[test]
    fn a_blank_name_still_greets() {
        assert_eq!(greet("  "), "Hello! (greeting #1)");
    }

    #[test]
    fn every_call_counts() {
        let mut host = HelloHost { greeted: 41 };
        let value = host
            .call("greet", &[Value::String("Ada".into())])
            .expect("command");
        assert!(
            matches!(value, Value::String(text) if text.as_ref() == "Hello, Ada! (greeting #42)")
        );
    }

    fn tree() -> brul::gui::WidgetTree {
        let component: brul::ComponentHandle = Rc::new(RefCell::new(Component::new()));
        let mut state = component.borrow_mut();
        state.set_names(generated::names());
        for (name, value) in [
            ("name", Value::String("world".into())),
            ("greeting", Value::String(String::new().into())),
        ] {
            state.declare(name, value, false).expect("seed");
        }
        drop(state);

        let mut build = brul::Build::new(brul::Services::default());
        let built = generated::hello_view(Some(generated::ThemeId::Dark), &mut build, &component)
            .expect("generated view");
        brul::gui::WidgetTree::new(built.widget)
    }

    #[test]
    fn the_window_lays_out_the_kit_buttons() {
        let mut tree = tree();
        let mut engine = brul::gui::TextEngine::new();
        tree.layout(
            &mut engine,
            brul::gui::Size {
                width: 420,
                height: 240,
            },
            1.0,
        )
        .expect("layout");
        let snapshot = tree.snapshot().expect("snapshot");

        let bottom = snapshot
            .nodes
            .iter()
            .map(|node| i64::from(node.y) + i64::from(node.height))
            .max()
            .expect("nodes");
        assert!(bottom <= 240, "the content ends at {bottom}");
        assert!(
            snapshot
                .nodes
                .iter()
                .any(|node| node.kind == brul::gui::WidgetKind::View && node.height == 32),
            "the kit buttons are missing"
        );
    }
}
