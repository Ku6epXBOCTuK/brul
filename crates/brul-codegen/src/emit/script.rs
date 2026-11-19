use std::fmt::Write;

use brul_script::Names;
use brul_syntax::error::Error;
use brul_syntax::script::Program;

use super::statement::block;

const RESERVED: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in",
    "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "static", "struct", "super", "trait", "true", "try", "type", "typeof", "unsafe",
    "unsized", "use", "virtual", "where", "while", "yield",
];

pub(super) fn module(
    name: &str,
    program: Option<&Program>,
    names: &Names<'_>,
) -> Result<String, Error> {
    let mut out = String::new();
    let _ = writeln!(out, "\npub fn {name}() -> Option<brul::Script<'static>> {{");
    let Some(program) = program else {
        out.push_str("    None\n}\n");
        return Ok(out);
    };
    let _ = writeln!(
        out,
        "    Some(brul::Script::Generated {{ init: {name}_init, dispatch: {name}_dispatch }})\n}}"
    );
    let _ = writeln!(
        out,
        "#[allow(unused_variables)]\nfn {name}_init(component: &mut brul::Component) -> Result<(), brul::Error> {{"
    );
    block(&program.statements, names, &mut out)?;
    out.push_str("    Ok(())\n}\n");
    for function in &program.functions {
        let function_name = format!("{name}_fn_{}", ident(&snake(&function.name)));
        let _ = writeln!(
            out,
            "#[allow(unused_mut, unused_variables)]\nfn {function_name}(component: &mut brul::Component, mut arguments: Vec<brul::Value>) -> Result<brul::Value, brul::Error> {{"
        );
        for parameter in &function.params {
            let _ = writeln!(
                out,
                "    component.declare({parameter:?}, arguments.remove(0), false)?;"
            );
        }
        block(&function.body, names, &mut out)?;
        out.push_str("    Ok(brul::Value::Null)\n}\n");
    }
    let _ = writeln!(
        out,
        "fn {name}_dispatch(component: &mut brul::Component, name: &str, arguments: Vec<brul::Value>) -> Result<brul::Value, brul::Error> {{"
    );
    out.push_str("    match name {\n");
    for function in &program.functions {
        let function_name = format!("{name}_fn_{}", ident(&snake(&function.name)));
        let expected = function.params.len();
        let wrong = if expected == 0 {
            "!arguments.is_empty()".to_owned()
        } else {
            format!("arguments.len() != {expected}")
        };
        let _ = writeln!(out, "        {:?} => {{", function.name);
        let _ = writeln!(
            out,
            "            if {wrong} {{ return Err(brul::Error::FunctionArity {{ name: name.to_owned(), expected: {expected}, found: arguments.len() }}); }}"
        );
        let _ = writeln!(out, "            component.push_frame();");
        let _ = writeln!(
            out,
            "            let result = {function_name}(component, arguments);"
        );
        out.push_str("            component.pop_frame();\n            result\n        }\n");
    }
    out.push_str("        _ => Err(brul::Error::UnknownFunction(name.to_owned())),\n    }\n}\n");
    Ok(out)
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
