use std::collections::HashMap;
use std::fmt::Write;

use brul_case::{ident, pascal, snake};
use brul_script::Names;
use brul_syntax::error::Error;
use brul_syntax::script::Program;
use brul_syntax::types::Type;

use super::expression::{convert, wrap_typed};
use super::scopes::{FunctionSig, Scopes};
use super::state::State;
use super::statement::{block_with_scopes, top_level};

pub(super) fn module(
    name: &str,
    program: Option<&Program>,
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
) -> Result<String, Error> {
    let mut out = state.definition(&format!("{name}_state_init"));
    let _ = writeln!(out, "\npub fn {name}() -> Option<brul::Script> {{");
    let Some(program) = program else {
        out.push_str("    None\n}\n");
        return Ok(out);
    };
    let functions: HashMap<String, FunctionSig> = program
        .functions
        .iter()
        .map(|function| {
            (
                function.name.clone(),
                FunctionSig {
                    rust_name: format!("{name}_fn_{}", ident(&snake(&function.name))),
                    params: function
                        .params
                        .iter()
                        .map(|param| param.ty.clone())
                        .collect(),
                    return_type: function.return_type.clone(),
                },
            )
        })
        .collect();
    let _ = writeln!(
        out,
        "    Some(brul::Script::Typed {{ init: {name}_init, call: {name}_typed_call, dispatch: {name}_dispatch }})\n}}"
    );
    let _ = writeln!(
        out,
        "#[allow(unused_variables)]\nfn {name}_init(component: &mut brul::Component) -> Result<(), brul::Error> {{"
    );
    for function in &program.functions {
        let _ = writeln!(
            out,
            "    component.register_event_call({:?}, {});",
            function.name,
            event_call(name, &function.name)
        );
    }
    top_level(&program.statements, names, state, props, &mut out)?;
    out.push_str(&state.init_call(&format!("{name}_state_init")));
    out.push_str("    Ok(())\n}\n");
    for function in &program.functions {
        let function_name = format!("{name}_fn_{}", ident(&snake(&function.name)));
        let params = function
            .params
            .iter()
            .map(|param| {
                format!(
                    "mut {}: {}",
                    ident(&snake(&param.name)),
                    Scopes::local_type(&param.ty)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let separator = if params.is_empty() { "" } else { ", " };
        let returns_unit = matches!(function.return_type, Type::Void | Type::Null);
        let return_type = if returns_unit {
            "()"
        } else {
            Scopes::local_type(&function.return_type)
        };
        let _ = writeln!(
            out,
            "#[allow(unused_mut, unused_variables)]\nfn {function_name}(component: &mut brul::Component{separator}{params}) -> Result<{return_type}, brul::Error> {{"
        );
        let mut scopes = Scopes::new()
            .with_return_type(&function.return_type)
            .with_functions(functions.clone());
        for parameter in &function.params {
            let _ = scopes.declare(&parameter.name, &parameter.ty, false)?;
        }
        block_with_scopes(&function.body, names, state, props, &mut out, &mut scopes)?;
        if returns_unit {
            out.push_str("    Ok(())\n}\n");
        } else {
            out.push_str("    Err(brul::Error::Unsupported(\"missing return\"))\n}\n");
        }
    }
    for function in &program.functions {
        let wrapper = event_call(name, &function.name);
        let function_name = format!("{name}_fn_{}", ident(&snake(&function.name)));
        let _ = writeln!(
            out,
            "#[allow(unused_variables)]\nfn {wrapper}(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {{"
        );
        let arguments = function
            .params
            .iter()
            .enumerate()
            .map(|(index, param)| {
                let argument = format!("arguments[{index}]");
                match param.ty {
                    Type::Number | Type::String | Type::Bool | Type::Null => {
                        convert(&argument, &param.ty)
                    }
                    _ => format!("({argument}).clone()"),
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let separator = if arguments.is_empty() { "" } else { ", " };
        if matches!(function.return_type, Type::Void | Type::Null) {
            let _ = writeln!(
                out,
                "    {function_name}(component{separator}{arguments})?;\n    Ok(brul::Dyn::null())\n}}"
            );
        } else {
            let _ = writeln!(
                out,
                "    let result = {function_name}(component{separator}{arguments})?;\n    Ok({})\n}}",
                wrap_typed("result".to_owned(), &function.return_type)
            );
        }
    }
    let message = format!("{}Message", pascal(name));
    let _ = writeln!(out, "\n#[allow(dead_code)]\npub enum {message} {{");
    for function in &program.functions {
        let variant = pascal(&function.name);
        if function.params.is_empty() {
            let _ = writeln!(out, "    {variant},");
        } else {
            let types = function
                .params
                .iter()
                .map(|param| Scopes::local_type(&param.ty))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(out, "    {variant}({types}),");
        }
    }
    out.push_str("}\n");
    let _ = writeln!(
        out,
        "impl From<{message}> for brul::Message {{\n    fn from(message: {message}) -> Self {{\n        brul::Message::Invoke(Box::new(message))\n    }}\n}}\n"
    );
    let _ = writeln!(
        out,
        "#[allow(unused_variables)]\nfn {name}_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {{"
    );
    let _ = writeln!(out, "    match message.downcast_ref::<{message}>() {{");
    for function in &program.functions {
        let variant = pascal(&function.name);
        let function_name = format!("{name}_fn_{}", ident(&snake(&function.name)));
        let bindings = (0..function.params.len())
            .map(|index| format!("argument{index}"))
            .collect::<Vec<_>>();
        let pattern = if bindings.is_empty() {
            variant.clone()
        } else {
            format!("{variant}({})", bindings.join(", "))
        };
        let arguments = bindings
            .iter()
            .map(|binding| format!("({binding}).clone()"))
            .collect::<Vec<_>>()
            .join(", ");
        let separator = if arguments.is_empty() { "" } else { ", " };
        if matches!(function.return_type, Type::Void | Type::Null) {
            let _ = writeln!(
                out,
                "        Some({message}::{pattern}) => {{ {function_name}(component{separator}{arguments})?; Ok(brul::Dyn::null()) }}"
            );
        } else {
            let _ = writeln!(
                out,
                "        Some({message}::{pattern}) => {{ let result = {function_name}(component{separator}{arguments})?; Ok({}) }}",
                wrap_typed("result".to_owned(), &function.return_type)
            );
        }
    }
    out.push_str("        _ => Err(brul::Error::Unsupported(\"script message\")),\n    }\n}\n");
    let _ = writeln!(
        out,
        "#[allow(unused_mut)]\nfn {name}_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {{"
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
        let arguments = function
            .params
            .iter()
            .map(|param| convert("arguments.remove(0)", &param.ty))
            .collect::<Vec<_>>()
            .join(", ");
        if matches!(function.return_type, Type::Void | Type::Null) {
            let _ = writeln!(out, "            {function_name}(component, {arguments})?;");
            out.push_str("            Ok(brul::Dyn::null())\n");
        } else {
            let _ = writeln!(
                out,
                "            let result = {function_name}(component, {arguments})?;"
            );
            let _ = writeln!(
                out,
                "            Ok({})",
                wrap_typed("result".to_owned(), &function.return_type)
            );
        }
        out.push_str("        }\n");
    }
    out.push_str("        _ => Err(brul::Error::UnknownFunction(name.to_owned())),\n    }\n}\n");
    Ok(out)
}

pub(super) fn handle(name: &str, program: Option<&Program>) -> String {
    let Some(program) = program else {
        return String::new();
    };
    let handle = pascal(name);
    let message = format!("{handle}Message");
    let mut out = format!(
        "\n#[derive(Clone)]\n#[allow(dead_code)]\npub struct {handle}(brul::Handle);\n\n#[allow(dead_code)]\nimpl {handle} {{\n"
    );
    out.push_str("    pub fn new(handle: brul::Handle) -> Self {\n        Self(handle)\n    }\n");
    for function in &program.functions {
        let method = ident(&snake(&function.name));
        let variant = pascal(&function.name);
        let arguments: Vec<String> = function
            .params
            .iter()
            .map(|param| {
                format!(
                    "{}: {}",
                    ident(&snake(&param.name)),
                    Scopes::local_type(&param.ty)
                )
            })
            .collect();
        let values = function
            .params
            .iter()
            .map(|param| ident(&snake(&param.name)))
            .collect::<Vec<_>>()
            .join(", ");
        if arguments.is_empty() {
            let _ = writeln!(
                out,
                "\n    pub fn {method}(&self) {{\n        self.0.send({message}::{variant});\n    }}"
            );
        } else {
            let arguments = arguments.join(", ");
            let _ = writeln!(
                out,
                "\n    pub fn {method}(&self, {arguments}) {{\n        self.0.send({message}::{variant}({values}));\n    }}"
            );
        }
    }
    out.push_str("}\n");
    out
}

pub(super) fn event_call(script: &str, function: &str) -> String {
    format!("{script}_event_{}", ident(&snake(function)))
}
