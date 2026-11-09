use brul_compiler::load;
use brul_gui::{Color, Size, TextEngine, WidgetTree};
use brul_script::Value;
use brul_syntax::error::Error;

fn snapshot(
    source: &str,
    theme: Option<&str>,
) -> (brul_gui::window::WindowConfig, brul_gui::WidgetSnapshot) {
    let loaded = load(source, theme).expect("app loads");
    let mut tree = WidgetTree::new(loaded.tree);
    let mut engine = TextEngine::new();
    tree.layout(
        &mut engine,
        Size {
            width: 300,
            height: 300,
        },
        1.0,
    )
    .expect("layout");
    (loaded.config, tree.snapshot().expect("snapshot").clone())
}

fn failure(source: &str) -> Error {
    match load(source, None) {
        Ok(_) => panic!("expected an error"),
        Err(error) => error,
    }
}

const PICKER: &str = r#"
<script>
let count = 0;
function launch(i) {
    count = i;
}
</script>

<markup>
window Picker (
    title: "bropicker", width: 488, height: 800,
    frameless: true, transparent: true
) {
    Column (root) {
        Text { "Hello" }
    }
}
</markup>

<style>
.root { width: 488px; height: 800px; background: #334455; }
</style>
"#;

#[test]
fn full_file_loads_config_tree_and_program() {
    let (config, snapshot) = snapshot(PICKER, None);

    assert_eq!(config.title, "bropicker");
    assert_eq!((config.width, config.height), (488, 800));

    let root = snapshot.nodes.first().expect("root node");
    assert_eq!(root.width, 488);
    assert_eq!(root.height, 800);
    assert_eq!(root.style.background, Color::rgb(0x33, 0x44, 0x55));
}

#[test]
fn script_block_is_parsed_into_a_program() {
    let loaded = load(PICKER, None).expect("app loads");
    let program = loaded.program.expect("script block");

    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "launch");
}

#[test]
fn script_runs_once_at_load() {
    let loaded = load(PICKER, None).expect("app loads");

    assert_eq!(loaded.component.read("count"), Ok(Value::Number(0.0)));
}

#[test]
fn script_execution_error_keeps_its_position() {
    let error = failure(
        r#"
<script>
let value = 1 / 0;
</script>

<markup>
window Picker (title: "x") { Column { } }
</markup>
"#,
    );

    assert_eq!(error.message, "division by zero");
    assert_eq!((error.line, error.column), (3, 13));
}

#[test]
fn use_block_is_parsed_before_markup() {
    let loaded = load(
        "<use>\nimport BrowserItem from \"./browser_item\";\n</use>\n\n<markup>\nwindow Picker (title: \"x\") { Column { } }\n</markup>",
        None,
    )
    .expect("app loads");

    assert_eq!(loaded.uses.len(), 1);
    assert_eq!(loaded.uses[0].name, "BrowserItem");
    assert_eq!(loaded.uses[0].path, "./browser_item");
}

#[test]
fn missing_arguments_keep_window_defaults() {
    let (config, _) = snapshot(
        r#"
        <markup>
        window Picker (title: "x") { Column { } }
        </markup>
        "#,
        None,
    );

    assert_eq!(config.title, "x");
    assert_eq!((config.width, config.height), (960, 640));
}

#[test]
fn unknown_window_argument_points_at_its_value() {
    let error =
        failure("<markup>\nwindow Picker (title: \"x\", scale: 2) { Column { } }\n</markup>");

    assert_eq!(error.message, "unknown window argument 'scale'");
    assert_eq!((error.line, error.column), (2, 35));
}

#[test]
fn width_must_be_a_positive_integer() {
    let error = failure("<markup>\nwindow Picker (width: 0) { Column { } }\n</markup>");

    assert_eq!(
        error.message,
        "window argument 'width' must be a positive integer"
    );
    assert_eq!(error.line, 2);
}

#[test]
fn false_window_flags_are_rejected() {
    let error = failure("<markup>\nwindow Picker (frameless: false) { Column { } }\n</markup>");

    assert_eq!(error.message, "window is always frameless");
    assert_eq!(error.line, 2);
}

#[test]
fn script_errors_use_file_coordinates() {
    let error = failure(
        "<script>\nlet count = ;\n</script>\n\n<markup>\nwindow Picker (title: \"x\") { Column { } }\n</markup>",
    );

    assert_eq!(error.message, "expected expression");
    assert_eq!((error.line, error.column), (2, 13));
}

#[test]
fn style_errors_use_file_coordinates() {
    let error = failure(
        "<markup>\nwindow Picker (title: \"x\") { Column (root) { } }\n</markup>\n\n<style>\n.root { nope: 1px; }\n</style>",
    );

    assert_eq!(error.line, 6);
}

#[test]
fn window_without_a_root_element_is_rejected() {
    let error = failure("<markup>\nwindow Picker (title: \"x\") { }\n</markup>");

    assert_eq!(error.message, "window has no root element");
    assert_eq!(error.line, 2);
}
