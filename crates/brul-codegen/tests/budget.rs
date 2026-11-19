use std::path::Path;

use brul_codegen::{Command, Commands, emit};
use brul_syntax::types::Type;

const CEILING: usize = 1_200;

#[test]
fn fixed_markup_stays_within_the_output_budget() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/budget/main.brul");
    let mut commands = Commands::new();
    let command = Command {
        name: "toggleLabel".to_owned(),
        path: "crate::__brul_command_toggle_label".to_owned(),
        params: Vec::new(),
        result: Type::Void,
    };
    commands.insert(command.name.clone(), command.clone());
    commands.insert("toggle_label".to_owned(), command);
    let module = emit::module_file_with_commands(&path, &commands).expect("module");

    let lines = module.lines().count();
    assert!(
        lines <= CEILING,
        "lines: {lines}, ceiling {CEILING}: raise the ceiling only when the output doubles \
         deliberately, never to fit one more field"
    );
    assert!(
        module.contains("brul::RowMatcher::new"),
        "the fixture must emit a keyed each"
    );
    assert!(
        module.contains("brul::slot("),
        "the fixture must emit a slot view"
    );
}
