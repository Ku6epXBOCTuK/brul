use std::cell::Cell;
use std::rc::Rc;

use brul::{Error, Host, Ui, Value};
use brul_compiler::load_with;
use brul_gui::window::WindowCommand;
use brul_gui::{Size, TextEngine, WidgetEvent, WidgetId, WidgetKind};

const APP: &str = r#"
<script>
let hits = 0;
function hit() {
    hits = hits + 1;
    bump();
}
</script>

<markup>
window Demo (title: "Demo") {
    Column {
        Text (on click: hit) { greeting }
    }
}
</markup>
"#;

struct Counter {
    hits: Rc<Cell<u32>>,
}

impl Host for Counter {
    fn call(&mut self, name: &str, _arguments: &[Value]) -> Result<Value, Error> {
        if name != "bump" {
            return Err(Error::UnknownFunction(name.to_owned()));
        }
        self.hits.set(self.hits.get() + 1);
        Ok(Value::Null)
    }
}

fn text_ids(ui: &Ui) -> Vec<WidgetId> {
    ui.tree
        .borrow_mut()
        .layout(
            &mut TextEngine::new(),
            Size {
                width: 300,
                height: 100,
            },
            1.0,
        )
        .expect("layout");
    let snapshot = ui.tree.borrow_mut().snapshot().expect("snapshot").clone();
    snapshot
        .nodes
        .iter()
        .filter(|node| node.kind == WidgetKind::Text)
        .map(|node| node.id)
        .collect()
}

#[test]
fn seeds_are_visible_to_scripts_and_markup() {
    let loaded = load_with(
        APP,
        None,
        &[("greeting", Value::String("seeded".to_owned()))],
    )
    .expect("app loads");

    assert_eq!(
        loaded.component.borrow().read("greeting"),
        Ok(Value::String("seeded".to_owned()))
    );
}

#[test]
fn missing_seed_fails_the_build() {
    let error = match load_with(APP, None, &[]) {
        Ok(_) => panic!("seed is required"),
        Err(error) => error,
    };

    assert!(
        error.message.contains("greeting"),
        "unexpected error: {}",
        error.message
    );
}

#[test]
fn app_host_receives_calls_from_the_script() {
    let hits = Rc::new(Cell::new(0));
    let host = Counter {
        hits: Rc::clone(&hits),
    };
    let loaded = load_with(
        APP,
        None,
        &[("greeting", Value::String("seeded".to_owned()))],
    )
    .expect("app loads");
    let mut ui = Ui::with_host(loaded, Some(Box::new(host)));
    let id = text_ids(&ui)[0];

    ui.dispatch(&[WidgetEvent::Click(id)]);

    assert_eq!(hits.get(), 1);
    assert_eq!(ui.component.borrow().read("hits"), Ok(Value::Number(1.0)));
}

#[test]
fn window_commands_still_reach_the_window_with_an_app_host() {
    const QUIT: &str = r#"
<script>
function quit() {
    exit();
}
</script>

<markup>
window Demo (title: "Demo") {
    Text (on click: quit) { "quit" }
}
</markup>
"#;
    let host = Counter {
        hits: Rc::new(Cell::new(0)),
    };
    let loaded = load_with(QUIT, None, &[]).expect("app loads");
    let mut ui = Ui::with_host(loaded, Some(Box::new(host)));
    let id = text_ids(&ui)[0];

    let commands = ui.dispatch(&[WidgetEvent::Click(id)]);

    assert_eq!(commands, vec![WindowCommand::Exit]);
}
