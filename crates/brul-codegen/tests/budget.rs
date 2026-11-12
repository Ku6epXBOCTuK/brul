use std::path::Path;

use brul_codegen::emit;

const LINES: usize = 379;
const FUNCTIONS: usize = 18;
const BYTES: usize = 17_494;

#[test]
fn fixed_markup_stays_within_the_output_budget() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/budget/main.brul");
    let module = emit::module_file(&path).expect("module");

    check("lines", module.lines().count(), LINES);
    check(
        "functions",
        module
            .lines()
            .filter(|line| {
                let line = line.trim_start();
                line.starts_with("fn ") || line.starts_with("pub fn ")
            })
            .count(),
        FUNCTIONS,
    );
    check("bytes", module.len(), BYTES);
}

fn check(metric: &str, actual: usize, budget: usize) {
    assert!(
        actual <= budget,
        "{metric}: {actual}, budget {budget}, {} over budget. \
         Lower the budget when the output shrinks; raise it only for a deliberate growth.",
        actual.saturating_sub(budget)
    );
}
