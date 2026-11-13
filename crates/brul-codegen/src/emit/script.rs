use std::fmt::Write;

use crate::source::Script;

pub(super) fn program(name: &str, script: Option<&Script>, file: Option<&str>) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\npub fn {name}() -> Option<&'static brul::script::Program> {{"
    );
    let Some(script) = script else {
        out.push_str("    None\n}\n");
        return out;
    };
    let source = match file {
        Some(file) => format!("include_str!({file:?})"),
        None => raw(&script.text),
    };
    let _ = writeln!(out, "    const SOURCE: &str = {source};");
    out.push_str(
        "    static PROGRAM: std::sync::LazyLock<Option<brul::script::Program>> = \
         std::sync::LazyLock::new(|| {\n",
    );
    let _ = writeln!(
        out,
        "        let text = [\"\\n\".repeat({}).as_str(), SOURCE].concat();",
        script.line - 1
    );
    out.push_str(
        "        Some(brul::script::parse(&text).expect(\"the script was validated when the module was generated\"))\n    });\n    PROGRAM.as_ref()\n}\n",
    );
    out
}

fn raw(text: &str) -> String {
    let mut hashes = 1;
    while text.contains(&format!("\"{}", "#".repeat(hashes))) {
        hashes += 1;
    }
    let delimiter = "#".repeat(hashes);
    format!("r{delimiter}\"{text}\"{delimiter}")
}
