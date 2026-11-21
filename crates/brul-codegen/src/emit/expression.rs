use std::collections::HashMap;
use std::fmt::Write;

use brul_script::Names;
use brul_syntax::TypeName;
use brul_syntax::ast::{
    AssignOp, BinaryOp, BinaryOpKind, Expr, ExprKind, Position, TemplatePart, UnaryOp,
};
use brul_syntax::error::Error;
use brul_syntax::script::builtins::{self, Argument, Builtin, BuiltinKind};
use brul_syntax::types::Type;

use super::names;
use super::scopes::{ScopeInfo, Scopes};
use super::state::State;
use crate::commands::Command;

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
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    definitions: &[ScopeInfo],
    current: Option<usize>,
) -> Result<String, Error> {
    let (locals, scopes) = scope_locals(definitions, current);
    Ok(format!(
        "|component: &mut brul::Component| -> Result<brul::Dyn, brul::Error> {{ {locals}let value = {}; Ok(value) }}",
        operation_in(value, names, state, props, &scopes)?
    ))
}

pub fn typed_closure(
    value: &Expr,
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    definitions: &[ScopeInfo],
    current: Option<usize>,
    ty: &Type,
) -> Result<String, Error> {
    let (locals, scopes) = scope_locals(definitions, current);
    let rust = ty.rust_type();
    let body = match typed(value, names, state, props, &scopes)? {
        Some((code, found)) if &found == ty => code,
        _ => format!(
            "(&{}).try_into()?",
            operation_in(value, names, state, props, &scopes)?
        ),
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
    operation_in(expression, names, state, props, &Scopes::default())
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
            "{{ let value = {convertor}(brul::Dyn::from(component.read_id({id})?)).map_err(|error| error.at(brul::ast::Position {{ line: {}, column: {} }}))?; value }}",
            position.line, position.column
        ),
        ty.clone(),
    )))
}

pub(super) fn typed(
    expression: &Expr,
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<Option<(String, Type)>, Error> {
    Ok(match &expression.kind {
        ExprKind::Number(value) => Some((format!("{value:?}_f64"), Type::Number)),
        ExprKind::String(text) => Some((format!("{text:?}.to_owned()"), Type::String)),
        ExprKind::Bool(value) => Some((value.to_string(), Type::Bool)),
        ExprKind::Null => Some(("()".to_owned(), Type::Null)),
        ExprKind::Ident(name) => {
            if let Some(value) = scopes.read_typed(name) {
                Some(value)
            } else if let Some(value) = state.read_typed(name) {
                Some(value)
            } else if let Some(ty) = props.get(name) {
                prop_typed(name, ty, expression.position, names)?
            } else {
                None
            }
        }
        ExprKind::Unary { op, operand } => {
            let Some((code, ty)) = typed(operand, names, state, props, scopes)? else {
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
            let Some((lhs, left_ty)) = typed(left, names, state, props, scopes)? else {
                return Ok(None);
            };
            let Some((rhs, right_ty)) = typed(right, names, state, props, scopes)? else {
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
            let Some((condition, condition_ty)) = typed(condition, names, state, props, scopes)?
            else {
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
            let Some((then_code, then_ty)) = typed(then_branch, names, state, props, scopes)?
            else {
                return Ok(None);
            };
            let Some((else_code, else_ty)) = typed(else_branch, names, state, props, scopes)?
            else {
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
            if let Some(builtin) = builtins::builtin(name) {
                return builtin_call(
                    builtin,
                    args,
                    expression.position,
                    names,
                    state,
                    props,
                    scopes,
                );
            }
            if let Some(command) = state.command(name) {
                return host_call(
                    command,
                    args,
                    expression.position,
                    names,
                    state,
                    props,
                    scopes,
                );
            }
            let Some(signature) = scopes.function(name) else {
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
                let code =
                    if let Some((code, found)) = typed(argument, names, state, props, scopes)? {
                        if &found != expected {
                            return Err(crate::arg_mismatch(argument.position, expected, &found));
                        }
                        code
                    } else {
                        convert(
                            &operation_in(argument, names, state, props, scopes)?,
                            expected,
                        )
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
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
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
    match builtin.kind {
        BuiltinKind::Host { marker } => {
            let params = builtin
                .arguments
                .iter()
                .map(|argument| match argument {
                    Argument::Value(ty) => ty.clone(),
                    Argument::Any | Argument::Function => Type::Object,
                })
                .collect();
            let command = Command {
                name: builtin.name.to_owned(),
                path: format!("brul::builtins::{marker}"),
                params,
                result: builtin.result.clone(),
            };
            host_call(&command, arguments, position, names, state, props, scopes)
        }
        BuiltinKind::StringFn => Ok(Some((
            format!(
                "brul::builtins::string({})?",
                value_argument(&arguments[0], names, state, props, scopes)?
            ),
            Type::String,
        ))),
        BuiltinKind::NumberFn => Ok(Some((
            format!(
                "brul::builtins::number({})?",
                value_argument(&arguments[0], names, state, props, scopes)?
            ),
            Type::Number,
        ))),
        BuiltinKind::Len => Ok(Some((
            format!(
                "brul::builtins::len({})?",
                value_argument(&arguments[0], names, state, props, scopes)?
            ),
            Type::Number,
        ))),
        BuiltinKind::Every => timer_typed(
            "every",
            "component.every",
            arguments,
            names,
            state,
            props,
            scopes,
        ),
        BuiltinKind::After => timer_typed(
            "after",
            "component.after",
            arguments,
            names,
            state,
            props,
            scopes,
        ),
        BuiltinKind::Cancel => cancel_typed(arguments, names, state, props, scopes),
    }
}

fn value_argument(
    expression: &Expr,
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<String, Error> {
    match typed(expression, names, state, props, scopes)? {
        Some((code, ty)) => Ok(wrap_typed(code, &ty)),
        None => operation_in(expression, names, state, props, scopes),
    }
}

fn timer_typed(
    name: &'static str,
    method: &'static str,
    arguments: &[Expr],
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<Option<(String, Type)>, Error> {
    let interval = &arguments[0];
    let function = &arguments[1];
    let interval_code = match typed(interval, names, state, props, scopes)? {
        Some((code, Type::Number)) => code,
        Some((_, ty)) => {
            return Err(crate::arg_mismatch(interval.position, &Type::Number, &ty));
        }
        None => format!(
            "(&{}).try_into()?",
            operation_in(interval, names, state, props, scopes)?
        ),
    };
    let ExprKind::Ident(callee) = &function.kind else {
        return Err(crate::error_at(
            function.position,
            format!("{name} expects a function name"),
        ));
    };
    let arity = if let Some(command) = state.command(callee) {
        command.params.len()
    } else if let Some(signature) = scopes.function(callee) {
        signature.params.len()
    } else {
        return Err(crate::error_at(
            function.position,
            format!("function '{callee}' is not defined"),
        ));
    };
    if arity != 0 {
        return Err(crate::error_at(
            function.position,
            format!("{name} function '{callee}' takes no arguments"),
        ));
    }
    Ok(Some((
        format!(
            "{{ let __brul_interval = {interval_code}; {method}(__brul_interval, {callee:?})? }}"
        ),
        Type::Timer,
    )))
}

fn cancel_typed(
    arguments: &[Expr],
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<Option<(String, Type)>, Error> {
    let timer = &arguments[0];
    let code = match typed(timer, names, state, props, scopes)? {
        Some((code, Type::Timer)) => code,
        Some((_, ty)) => return Err(crate::arg_mismatch(timer.position, &Type::Timer, &ty)),
        None => format!(
            "(&{}).try_into()?",
            operation_in(timer, names, state, props, scopes)?
        ),
    };
    Ok(Some((
        format!("{{ let __brul_timer = {code}; component.cancel(__brul_timer)?; }}"),
        Type::Void,
    )))
}

fn host_call(
    command: &Command,
    arguments: &[Expr],
    position: Position,
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
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
        let code = if let Some((code, found)) = typed(argument, names, state, props, scopes)? {
            if &found != expected {
                return Err(crate::arg_mismatch(argument.position, expected, &found));
            }
            code
        } else {
            convert(
                &operation_in(argument, names, state, props, scopes)?,
                expected,
            )
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
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<String, Error> {
    if let Some((code, ty)) = typed(expression, names, state, props, scopes)?
        && matches!(ty, Type::Null | Type::Void)
    {
        return Ok(code);
    }
    operation_in(expression, names, state, props, scopes)
}

pub(super) fn operation_in(
    expression: &Expr,
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<String, Error> {
    if let Some((code, ty)) = typed(expression, names, state, props, scopes)? {
        return Ok(wrap_typed(code, &ty));
    }
    Ok(match &expression.kind {
        ExprKind::Number(value) => format!("brul::Dyn::num({value:?})"),
        ExprKind::String(text) => format!("brul::Dyn::text({text:?})"),
        ExprKind::Bool(value) => format!("brul::Dyn::flag({value})"),
        ExprKind::Null => "brul::Dyn::null()".to_owned(),
        ExprKind::Ident(name) => {
            if let Some(read) = scopes.read(name) {
                read
            } else if let Some(read) = state.read(name) {
                read
            } else {
                format!(
                    "brul::Dyn::from(component.read_id({})?)",
                    names::variable_expr(name, expression.position, names)?
                )
            }
        }
        ExprKind::Field { target, name } => format!(
            "({}).field({name:?})?",
            operation_in(target, names, state, props, scopes)?
        ),
        ExprKind::Index { target, index } => format!(
            "({}).index(&({}))?",
            operation_in(target, names, state, props, scopes)?,
            operation_in(index, names, state, props, scopes)?
        ),
        ExprKind::Array(items) => format!(
            "brul::Dyn::array(vec![{}])",
            operations(items, names, state, props, scopes)?
        ),
        ExprKind::Object(fields) => {
            let fields = fields
                .iter()
                .map(|(name, value)| {
                    Ok(format!(
                        "({name:?}.to_owned(), {})",
                        operation_in(value, names, state, props, scopes)?
                    ))
                })
                .collect::<Result<Vec<_>, Error>>()?
                .join(", ");
            format!("brul::Dyn::object(vec![{fields}])")
        }
        ExprKind::Template(parts) => {
            let parts = parts
                .iter()
                .map(|part| match part {
                    TemplatePart::Text(text) => Ok(format!("brul::Dyn::text({text:?})")),
                    TemplatePart::Expr(expression) => {
                        operation_in(expression, names, state, props, scopes)
                    }
                })
                .collect::<Result<Vec<_>, Error>>()?
                .join(", ");
            format!("brul::Dyn::template(vec![{parts}])?")
        }
        ExprKind::Call { target, args } => call(target, args, names, state, props, scopes)?,
        ExprKind::Unary { op, operand } => format!(
            "brul::Dyn::unary(brul::ast::UnaryOp::{op:?}, &({}))?",
            operation_in(operand, names, state, props, scopes)?
        ),
        ExprKind::Binary { op, left, right } => format!(
            "brul::Dyn::binary(brul::ast::BinaryOp::{op:?}, &({}), &({}))?",
            operation_in(left, names, state, props, scopes)?,
            operation_in(right, names, state, props, scopes)?
        ),
        ExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "if ({}).boolean()? {{ {} }} else {{ {} }}",
            operation_in(condition, names, state, props, scopes)?,
            operation_in(then_branch, names, state, props, scopes)?,
            operation_in(else_branch, names, state, props, scopes)?
        ),
        ExprKind::Assign { target, op, value } => {
            assignment(target, *op, value, names, state, props, scopes)?
        }
    })
}

fn operations(
    expressions: &[Expr],
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<String, Error> {
    Ok(expressions
        .iter()
        .map(|expression| operation_in(expression, names, state, props, scopes))
        .collect::<Result<Vec<_>, Error>>()?
        .join(", "))
}

fn call(
    target: &Expr,
    arguments: &[Expr],
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<String, Error> {
    let ExprKind::Ident(name) = &target.kind else {
        return Ok("{ return Err(brul::Error::Unsupported(\"method call\")) }".to_owned());
    };
    Ok(format!(
        "{{ let arguments = vec![{}]; brul::Dyn::call(component, {name:?}, arguments)? }}",
        operations(arguments, names, state, props, scopes)?
    ))
}

fn assignment(
    target: &Expr,
    op: AssignOp,
    value: &Expr,
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<String, Error> {
    let value = match op {
        AssignOp::Set => operation_in(value, names, state, props, scopes)?,
        AssignOp::Add => format!(
            "brul::Dyn::binary(brul::ast::BinaryOp::Add, &({}), &({}))?",
            operation_in(target, names, state, props, scopes)?,
            operation_in(value, names, state, props, scopes)?
        ),
        AssignOp::Sub => format!(
            "brul::Dyn::binary(brul::ast::BinaryOp::Sub, &({}), &({}))?",
            operation_in(target, names, state, props, scopes)?,
            operation_in(value, names, state, props, scopes)?
        ),
    };
    let mut out = format!("{{ let value = {value};\n");
    assign_to(&mut out, target, "value", 0, names, state, props, scopes)?;
    out.push_str("\nvalue }");
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn assign_to(
    out: &mut String,
    target: &Expr,
    value: &str,
    depth: usize,
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<(), Error> {
    match &target.kind {
        ExprKind::Ident(name) => {
            if let Some(write) = scopes.write(name, value)? {
                let _ = writeln!(out, "    {write}");
                return Ok(());
            }
            if let Some(write) = state.write(name, value) {
                let _ = writeln!(out, "    {write}");
                return Ok(());
            }
            let _ = writeln!(
                out,
                "component.assign_id({}, ({value}.clone()).into())?;",
                names::variable_expr(name, target.position, names)?
            );
            Ok(())
        }
        ExprKind::Field {
            target: field,
            name,
        } => {
            let container = format!("container{depth}");
            let _ = writeln!(
                out,
                "let mut {container} = {};",
                operation_in(field, names, state, props, scopes)?
            );
            let _ = writeln!(out, "{container}.assign_field({name:?}, &{value})?;");
            assign_to(
                out,
                field,
                &container,
                depth + 1,
                names,
                state,
                props,
                scopes,
            )
        }
        ExprKind::Index {
            target: container_expression,
            index,
        } => {
            let position = format!("index{depth}");
            let container = format!("container{depth}");
            let _ = writeln!(
                out,
                "let {position} = {};",
                operation_in(index, names, state, props, scopes)?
            );
            let _ = writeln!(
                out,
                "let mut {container} = {};",
                operation_in(container_expression, names, state, props, scopes)?
            );
            let _ = writeln!(out, "{container}.assign_index(&{position}, &{value})?;");
            assign_to(
                out,
                container_expression,
                &container,
                depth + 1,
                names,
                state,
                props,
                scopes,
            )
        }
        other => {
            let _ = write!(
                out,
                "return Err(brul::Error::Unsupported({:?}));",
                other.name()
            );
            Ok(())
        }
    }
}
