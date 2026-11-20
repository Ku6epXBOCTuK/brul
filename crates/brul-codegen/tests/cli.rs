use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("brul-cli-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&Path]) -> Output {
    let strings: Vec<&str> = args.iter().map(|path| path.to_str().unwrap()).collect();
    Command::new(env!("CARGO_BIN_EXE_emit"))
        .args(&strings)
        .output()
        .unwrap()
}

#[test]
fn generates_a_component_file() {
    let dir = scratch("component");
    let input = dir.join("input.brul");
    let output = dir.join("nested/expected.rs");
    fs::write(
        &input,
        "<markup>\ncomponent Badge {\n    Text { \"badge\" }\n}\n</markup>\n",
    )
    .unwrap();
    let result = run(&[&input, Path::new("--out"), &output]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let generated = fs::read_to_string(&output).unwrap();
    assert!(generated.contains("pub fn input_view("), "{generated}");
}

#[test]
fn generates_a_window_module() {
    let dir = scratch("window");
    let input = dir.join("input.brul");
    let output = dir.join("expected.rs");
    fs::write(
        &input,
        "<markup>\nwindow Demo (title: \"demo\") {\n    Text { \"hi\" }\n}\n</markup>\n",
    )
    .unwrap();
    let result = run(&[&input, Path::new("--out"), &output]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let generated = fs::read_to_string(&output).unwrap();
    assert!(generated.contains("pub fn demo_view("), "{generated}");
}

#[test]
fn regenerates_fixtures_and_drops_stale_goldens() {
    let dir = scratch("fixtures");
    let good = dir.join("good");
    let bad = dir.join("bad");
    fs::create_dir_all(&good).unwrap();
    fs::create_dir_all(&bad).unwrap();
    fs::write(
        good.join("input.brul"),
        "<markup>\ncomponent Badge {\n    Text { \"badge\" }\n}\n</markup>\n",
    )
    .unwrap();
    fs::write(bad.join("input.brul"), "<markup>\n</markup>\n").unwrap();
    fs::write(good.join("error.txt"), "stale").unwrap();
    fs::write(bad.join("expected.rs"), "stale").unwrap();

    let result = run(&[Path::new("--fixtures"), &dir]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let generated = fs::read_to_string(good.join("expected.rs")).unwrap();
    assert!(generated.contains("pub fn input_view("), "{generated}");
    assert!(!good.join("error.txt").exists());
    assert!(!bad.join("expected.rs").exists());
    let error = fs::read_to_string(bad.join("error.txt")).unwrap();
    assert!(
        error.contains("file has no component declaration"),
        "{error}"
    );
}

#[test]
fn reads_host_commands_from_the_case_directory() {
    let dir = scratch("commands");
    let input = dir.join("input.brul");
    let output = dir.join("expected.rs");
    fs::write(
        dir.join("commands.txt"),
        "capitalize|crate::__brul_command_capitalize|String|String\n",
    )
    .unwrap();
    fs::write(
        &input,
        "<script>\n    let label: String;\n    function greet(): Void {\n        label = capitalize(label);\n    }\n</script>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Text { label }\n}\n</markup>\n",
    )
    .unwrap();
    let result = run(&[&input, Path::new("--out"), &output]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let generated = fs::read_to_string(&output).unwrap();
    assert!(
        generated.contains("__brul_command_capitalize"),
        "{generated}"
    );
}

#[test]
fn reports_unknown_arguments() {
    let result = run(&[Path::new("--nope")]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("usage: emit"));
}
