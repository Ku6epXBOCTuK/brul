use std::fmt::Write;

use brul_syntax::script::Program;

use crate::source::Script;

const RESERVED: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in",
    "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "static", "struct", "super", "trait", "true", "try", "type", "typeof", "unsafe",
    "unsized", "use", "virtual", "where", "while", "yield",
];

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

pub(super) fn handle(program: Option<&Program>) -> String {
    let Some(program) = program else {
        return String::new();
    };
    let mut out = String::from(
        "\n#[derive(Clone)]\n#[allow(dead_code)]\npub struct Script(brul::Handle);\n\n#[allow(dead_code)]\nimpl Script {\n",
    );
    out.push_str("    pub fn new(handle: brul::Handle) -> Self {\n        Self(handle)\n    }\n");
    for function in &program.functions {
        let method = ident(&snake(&function.name));
        let name = &function.name;
        let arguments: Vec<String> = function
            .params
            .iter()
            .map(|param| format!("{}: impl Into<brul::Value>", ident(&snake(param))))
            .collect();
        if arguments.is_empty() {
            let _ = writeln!(
                out,
                "\n    pub fn {method}(&self) {{\n        self.0.call({name:?});\n    }}"
            );
        } else {
            let values = function
                .params
                .iter()
                .map(|param| format!("{}.into()", ident(&snake(param))))
                .collect::<Vec<_>>()
                .join(", ");
            let arguments = arguments.join(", ");
            let _ = writeln!(
                out,
                "\n    pub fn {method}(&self, {arguments}) {{\n        self.0.call_with({name:?}, ::std::vec![{values}]);\n    }}"
            );
        }
    }
    out.push_str("}\n");
    out
}

fn snake(name: &str) -> String {
    let mut output = String::with_capacity(name.len());
    for (index, character) in name.chars().enumerate() {
        if character.is_uppercase() {
            if index > 0 && !output.ends_with('_') {
                output.push('_');
            }
            output.extend(character.to_lowercase());
        } else {
            output.push(character);
        }
    }
    output
}

fn ident(name: &str) -> String {
    if RESERVED.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_owned()
    }
}

fn raw(text: &str) -> String {
    let mut hashes = 1;
    while text.contains(&format!("\"{}", "#".repeat(hashes))) {
        hashes += 1;
    }
    let delimiter = "#".repeat(hashes);
    format!("r{delimiter}\"{text}\"{delimiter}")
}
