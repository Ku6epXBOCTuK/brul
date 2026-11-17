use std::path::Path;

use brul_codegen::emit;

const CEILING: usize = 1_200;

#[test]
fn fixed_markup_stays_within_the_output_budget() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/budget/main.brul");
    let module = emit::module_file(&path).expect("module");

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
