use std::collections::HashMap;

use brul_script::Names;
use brul_syntax::TypeName;
use brul_syntax::ast::{BinaryOp, BinaryOpKind, Expr, ExprKind, Position, UnaryOp};
use brul_syntax::error::Error;
use brul_syntax::script::builtins as syntax_builtins;
use brul_syntax::script::builtins::Builtin;
use brul_syntax::types::Type;

use super::ExprContext;
use super::names;
use super::rust_string;
use super::scopes::{ScopeInfo, Scopes};
use super::state::State;
use crate::commands::Command;

pub(super) use super::expression_operations::operation_in;

fn scope_locals(definitions: &[ScopeInfo], current: Option<usize>) -> (String, Scopes) {
    let mut scopes = Scopes::default();
    let mut locals = String::new();
    if let Some(index) = current {
        let info = &definitions[index];
        let outer = info.outer.map(|index| &definitions[index]);
        if let Some(outer) = outer {
            for field in &outer.fields {
                locals.push_str(&format!(
                    "let {} = data.{}.clone();\n",
                    field.field, field.field
                ));
            }
        }
        for field in &info.fields {
            locals.push_str(&format!(
                "let {} = data.{}.clone();\n",
                field.field, field.field
            ));
        }
        scopes.push_scope(info, outer);
        locals.insert_str(
            0,
            &format!(
                "let data = component.scope().get::<{}>().expect(\"each scope\");\n",
                info.struct_name
            ),
        );
    }
    (locals, scopes)
}

pub fn closure(
    value: &Expr,
    context: &ExprContext<'_>,
    definitions: &[ScopeInfo],
    current: Option<usize>,
) -> Result<String, Error> {
    let (locals, scopes) = scope_locals(definitions, current);
    let context = context.with_scopes(scopes);
    Ok(format!(
        "|component: &mut brul::Component| -> Result<brul::Dyn, brul::Error> {{ {locals}let value = {}; Ok(value) }}",
        operation_in(value, &context)?
    ))
}

pub fn typed_closure(
    value: &Expr,
    context: &ExprContext<'_>,
    definitions: &[ScopeInfo],
    current: Option<usize>,
    ty: &Type,
) -> Result<String, Error> {
    let (locals, scopes) = scope_locals(definitions, current);
    let context = context.with_scopes(scopes);
    let rust = ty.rust_type();
    let body = match typed(value, &context)? {
        Some((code, found)) if &found == ty => code,
        _ => format!("(&{}).try_into()?", operation_in(value, &context)?),
    };
    Ok(format!(
        "|component: &mut brul::Component| -> Result<{rust}, brul::Error> {{ {locals}Ok({body}) }}"
    ))
}

pub fn operation(
    expression: &Expr,
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
) -> Result<String, Error> {
    operation_in(expression, &ExprContext::new(names, state, props))
}

pub(super) fn wrap_typed(code: String, ty: &Type) -> String {
    ty.dyn_wrap(&code)
}

pub(super) fn convert(code: &str, ty: &Type) -> String {
    ty.convert(code)
}

fn prop_typed(
    name: &str,
    ty: &Type,
    position: brul_syntax::ast::Position,
    names: &Names<'_>,
) -> Result<Option<(String, Type)>, Error> {
    let Some(convertor) = ty.from_dyn() else {
        return Ok(None);
    };
    let id = names::variable_expr(name, position, names)?;
    Ok(Some((
        format!(
            "{{ {convertor}(brul::Dyn::from(component.read_id({id})?)).map_err(|error| error.at(brul::ast::Position {{ line: {}, column: {} }}))? }}",
            position.line, position.column
        ),
        ty.clone(),
    )))
}

pub(super) fn typed(
    expression: &Expr,
    context: &ExprContext<'_>,
) -> Result<Option<(String, Type)>, Error> {
    Ok(match &expression.kind {
        ExprKind::Number(value) => Some((format!("{value:?}_f64"), Type::Number)),
        ExprKind::String(text) => Some((format!("{}.to_owned()", rust_string(text)), Type::String)),
        ExprKind::Bool(value) => Some((value.to_string(), Type::Bool)),
        ExprKind::Null => Some(("()".to_owned(), Type::Null)),
        ExprKind::Ident(name) => {
            if let Some(value) = context.scopes.read_typed(name) {
                Some(value)
            } else if let Some(value) = context.state.read_typed(name) {
                Some(value)
            } else if let Some(ty) = context.props.get(name) {
                prop_typed(name, ty, expression.position, context.names)?
            } else {
                None
            }
        }
        ExprKind::Unary { op, operand } => {
            let Some((code, ty)) = typed(operand, context)? else {
                return Ok(None);
            };
            match (op, &ty) {
                (UnaryOp::Neg, Type::Number) => Some((format!("-({code})"), ty)),
                (UnaryOp::Not, Type::Bool) => Some((format!("!({code})"), ty)),
                _ => {
                    return Err(crate::error_at(
                        expression.position,
                        format!("operator {op:?} is not defined for {}", ty.type_name()),
                    ));
                }
            }
        }
        ExprKind::Binary { op, left, right } => {
            let Some((lhs, left_ty)) = typed(left, context)? else {
                return Ok(None);
            };
            let Some((rhs, right_ty)) = typed(right, context)? else {
                return Ok(None);
            };
            if left_ty != right_ty {
                return Err(crate::error_at(
                    expression.position,
                    format!(
                        "operator {op:?} cannot compare {} and {}",
                        left_ty.type_name(),
                        right_ty.type_name()
                    ),
                ));
            }
            let operator = match op {
                BinaryOp::Add if left_ty == Type::Number => "+",
                BinaryOp::Sub if left_ty == Type::Number => "-",
                BinaryOp::Mul if left_ty == Type::Number => "*",
                BinaryOp::Div if left_ty == Type::Number => "/",
                BinaryOp::Rem if left_ty == Type::Number => "%",
                BinaryOp::Eq => "==",
                BinaryOp::NotEq => "!=",
                BinaryOp::Lt if matches!(left_ty, Type::Number | Type::String) => "<",
                BinaryOp::Gt if matches!(left_ty, Type::Number | Type::String) => ">",
                BinaryOp::Le if matches!(left_ty, Type::Number | Type::String) => "<=",
                BinaryOp::Ge if matches!(left_ty, Type::Number | Type::String) => ">=",
                BinaryOp::And if left_ty == Type::Bool => "&&",
                BinaryOp::Or if left_ty == Type::Bool => "||",
                _ => {
                    return Err(crate::error_at(
                        expression.position,
                        format!("operator {op:?} is not defined for {}", left_ty.type_name()),
                    ));
                }
            };
            let string_comparison =
                left_ty == Type::String && matches!(op.kind(), BinaryOpKind::Comparison);
            let ty = match op.kind() {
                BinaryOpKind::Comparison | BinaryOpKind::Logical => Type::Bool,
                BinaryOpKind::Arithmetic => left_ty,
            };
            let code = if string_comparison {
                format!("({lhs}).as_str() {operator} ({rhs}).as_str()")
            } else {
                format!("({lhs}) {operator} ({rhs})")
            };
            Some((code, ty))
        }
        ExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            let Some((condition, condition_ty)) = typed(condition, context)? else {
                return Ok(None);
            };
            if condition_ty != Type::Bool {
                return Err(crate::error_at(
                    expression.position,
                    format!(
                        "?: condition must be Bool, found {}",
                        condition_ty.type_name()
                    ),
                ));
            }
            let Some((then_code, then_ty)) = typed(then_branch, context)? else {
                return Ok(None);
            };
            let Some((else_code, else_ty)) = typed(else_branch, context)? else {
                return Ok(None);
            };
            if then_ty != else_ty {
                return Err(crate::error_at(
                    expression.position,
                    format!(
                        "?: branches have different types: {} and {}",
                        then_ty.type_name(),
                        else_ty.type_name()
                    ),
                ));
            }
            Some((
                format!("if {condition} {{ {then_code} }} else {{ {else_code} }}"),
                then_ty,
            ))
        }
        ExprKind::Call { target, args } => {
            let ExprKind::Ident(name) = &target.kind else {
                return Ok(None);
            };
            if let Some(builtin) = syntax_builtins::builtin(name) {
                return builtin_call(builtin, args, expression.position, context);
            }
            if let Some(command) = context.state.command(name) {
                return host_call(command, args, expression.position, context);
            }
            let Some(signature) = context.scopes.function(name) else {
                return Ok(None);
            };
            if args.len() != signature.params.len() {
                return Err(crate::error_at(
                    expression.position,
                    format!(
                        "function '{name}' expects {} arguments, found {}",
                        signature.params.len(),
                        args.len()
                    ),
                ));
            }
            let mut arguments = Vec::new();
            for (argument, expected) in args.iter().zip(&signature.params) {
                let code = if let Some((code, found)) = typed(argument, context)? {
                    if &found != expected {
                        return Err(crate::arg_mismatch(argument.position, expected, &found));
                    }
                    code
                } else {
                    convert(&operation_in(argument, context)?, expected)
                };
                arguments.push(code);
            }
            Some((
                format!(
                    "{}(component, {})?",
                    signature.rust_name,
                    arguments.join(", ")
                ),
                signature.return_type.clone(),
            ))
        }
        _ => None,
    })
}

fn builtin_call(
    builtin: &Builtin,
    arguments: &[Expr],
    position: Position,
    context: &ExprContext<'_>,
) -> Result<Option<(String, Type)>, Error> {
    if arguments.len() != builtin.arguments.len() {
        return Err(crate::error_at(
            position,
            format!(
                "builtin '{}' expects {} arguments, found {}",
                builtin.name,
                builtin.arguments.len(),
                arguments.len()
            ),
        ));
    }
    super::builtins::emit_builtin(builtin, arguments, position, context)
}

pub(super) fn host_call(
    command: &Command,
    arguments: &[Expr],
    position: Position,
    context: &ExprContext<'_>,
) -> Result<Option<(String, Type)>, Error> {
    if arguments.len() != command.params.len() {
        return Err(crate::error_at(
            position,
            format!(
                "command '{}' expects {} arguments, found {}",
                command.name,
                command.params.len(),
                arguments.len()
            ),
        ));
    }
    let mut values = Vec::new();
    for (argument, expected) in arguments.iter().zip(&command.params) {
        let code = if let Some((code, found)) = typed(argument, context)? {
            if &found != expected {
                return Err(crate::arg_mismatch(argument.position, expected, &found));
            }
            code
        } else {
            convert(&operation_in(argument, context)?, expected)
        };
        values.push(code);
    }
    let bindings: String = values
        .iter()
        .enumerate()
        .map(|(index, value)| format!("let __brul_arg{index} = {value}; "))
        .collect();
    let variables: Vec<String> = (0..values.len())
        .map(|index| format!("__brul_arg{index}"))
        .collect();
    let tuple = if variables.is_empty() {
        "()".to_owned()
    } else {
        format!("({},)", variables.join(", "))
    };
    let (open, close) = if bindings.is_empty() {
        ("", "")
    } else {
        ("{ ", " }")
    };
    let code = format!(
        "{open}{bindings}<{} as brul::HostCommand>::invoke(component, {tuple})?{close}",
        command.path
    );
    Ok(Some((code, command.result.clone())))
}

pub(super) fn operation_statement(
    expression: &Expr,
    context: &ExprContext<'_>,
) -> Result<String, Error> {
    if let Some((code, ty)) = typed(expression, context)?
        && matches!(ty, Type::Null | Type::Void)
    {
        return Ok(code);
    }
    operation_in(expression, context)
}
