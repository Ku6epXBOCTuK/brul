use std::collections::HashMap;

use brul_script::Names;
use brul_syntax::ast::{Expr, ExprKind, Position};
use brul_syntax::error::Error;
use brul_syntax::script::{Argument, Builtin};
use brul_syntax::types::Type;

use super::expression::{convert, host_call, operation_in, typed, wrap_typed};
use super::scopes::Scopes;
use super::state::State;
use crate::commands::Command;

struct Context<'a> {
    position: Position,
    names: &'a Names<'a>,
    state: &'a State,
    props: &'a HashMap<String, Type>,
    scopes: &'a Scopes,
}

fn value_argument(expression: &Expr, context: &Context<'_>) -> Result<String, Error> {
    match typed(
        expression,
        context.names,
        context.state,
        context.props,
        context.scopes,
    )? {
        Some((code, ty)) => Ok(wrap_typed(code, &ty)),
        None => operation_in(
            expression,
            context.names,
            context.state,
            context.props,
            context.scopes,
        ),
    }
}

fn call_value_function(
    path: &'static str,
    result: Type,
    arguments: &[Expr],
    context: &Context<'_>,
) -> Result<Option<(String, Type)>, Error> {
    let code = value_argument(&arguments[0], context)?;
    Ok(Some((format!("{path}({code})?"), result)))
}

fn call_host(
    builtin: &Builtin,
    arguments: &[Expr],
    context: &Context<'_>,
) -> Result<Option<(String, Type)>, Error> {
    let params = builtin
        .arguments
        .iter()
        .map(|argument| match argument {
            Argument::Value(ty) => ty.clone(),
            Argument::Any | Argument::Function => Type::Object,
        })
        .collect();
    let marker = match builtin.emit {
        brul_syntax::script::BuiltinEmit::Host { marker } => marker,
        _ => unreachable!("only host builtins call call_host"),
    };
    let command = Command {
        name: builtin.name.to_owned(),
        path: format!("brul::builtins::{marker}"),
        params,
        result: builtin.result.clone(),
    };
    host_call(
        &command,
        arguments,
        context.position,
        context.names,
        context.state,
        context.props,
        context.scopes,
    )
}

fn call_method(
    method: &'static str,
    function_argument: Option<usize>,
    result: Type,
    builtin: &Builtin,
    arguments: &[Expr],
    context: &Context<'_>,
) -> Result<Option<(String, Type)>, Error> {
    let mut bindings = Vec::new();
    let mut values = Vec::new();
    for (index, argument) in arguments.iter().enumerate() {
        if function_argument == Some(index) {
            let ExprKind::Ident(callee) = &argument.kind else {
                return Err(crate::error_at(
                    argument.position,
                    format!("{method} expects a function name"),
                ));
            };
            let arity = if let Some(command) = context.state.command(callee) {
                command.params.len()
            } else if let Some(signature) = context.scopes.function(callee) {
                signature.params.len()
            } else {
                return Err(crate::error_at(
                    argument.position,
                    format!("function '{callee}' is not defined"),
                ));
            };
            if arity != 0 {
                return Err(crate::error_at(
                    argument.position,
                    format!("{method} function '{callee}' takes no arguments"),
                ));
            }
            values.push(format!("{callee:?}"));
            continue;
        }
        let expected = match &builtin.arguments[index] {
            Argument::Value(ty) => ty.clone(),
            Argument::Any | Argument::Function => Type::Object,
        };
        let code = match typed(
            argument,
            context.names,
            context.state,
            context.props,
            context.scopes,
        )? {
            Some((code, found)) if found == expected => code,
            Some((_, found)) => {
                return Err(crate::arg_mismatch(argument.position, &expected, &found));
            }
            None => convert(
                &operation_in(
                    argument,
                    context.names,
                    context.state,
                    context.props,
                    context.scopes,
                )?,
                &expected,
            ),
        };
        let variable = match expected {
            Type::Number => "__brul_interval".to_owned(),
            Type::Timer => "__brul_timer".to_owned(),
            _ => format!("__brul_arg{index}"),
        };
        bindings.push(format!("let {variable} = {code};"));
        values.push(variable);
    }
    let semicolon = if result == Type::Void { ";" } else { "" };
    let code = format!(
        "{{ {} {method}({})?{semicolon} }}",
        bindings.join(" "),
        values.join(", ")
    );
    Ok(Some((code, result)))
}

include!(concat!(env!("OUT_DIR"), "/lang_builtin_dispatch.rs"));
