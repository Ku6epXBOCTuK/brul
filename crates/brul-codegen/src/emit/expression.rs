use std::collections::HashMap;
use std::fmt::Write;

use brul_script::Names;
use brul_syntax::ast::{AssignOp, BinaryOp, Expr, ExprKind, TemplatePart, UnaryOp};
use brul_syntax::error::Error;
use brul_syntax::types::Type;

use super::names;
use super::scopes::{ScopeInfo, Scopes};
use super::state::State;

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
        "|component: &mut brul::Component| -> Result<brul::Value, brul::Error> {{ {locals}let value = {}; Ok(value) }}",
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
    let rust = match ty {
        Type::Number => "f64",
        Type::String => "String",
        Type::Bool => "bool",
        _ => "brul::Value",
    };
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
    match ty {
        Type::Number => format!("brul::Value::Number({code})"),
        Type::String => format!("brul::Value::String(({code}).into())"),
        Type::Bool => format!("brul::Value::Bool({code})"),
        Type::Null | Type::Void => "brul::Value::Null".to_owned(),
        _ => code,
    }
}

pub(super) fn convert(code: &str, ty: &Type) -> String {
    match ty {
        Type::Number | Type::String | Type::Bool => format!("(&{code}).try_into()?"),
        Type::Null => format!("{{ let _ = {code}; () }}"),
        _ => code.to_owned(),
    }
}

pub(super) fn type_name(ty: &Type) -> &'static str {
    match ty {
        Type::Number => "Number",
        Type::String => "String",
        Type::Bool => "Bool",
        Type::Null => "Null",
        Type::Image => "Image",
        Type::Object => "Object",
        Type::Void => "Void",
        Type::Array(_) => "Array",
    }
}

fn prop_typed(
    name: &str,
    ty: &Type,
    position: brul_syntax::ast::Position,
    names: &Names<'_>,
) -> Result<Option<(String, Type)>, Error> {
    let pattern = match ty {
        Type::Number => "brul::Value::Number(value) => value",
        Type::String => "brul::Value::String(value) => value.to_string()",
        Type::Bool => "brul::Value::Bool(value) => value",
        Type::Null => "brul::Value::Null => ()",
        _ => return Ok(None),
    };
    let id = names::variable_expr(name, position, names)?;
    Ok(Some((
        format!(
            "match component.read_id({id})? {{ {pattern}, other => return Err(brul::Error::Expected {{ operation: \"read\", expected: {:?}, found: other.type_name() }}.at(brul::ast::Position {{ line: {}, column: {} }})) }}",
            type_name(ty),
            position.line,
            position.column
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
                        format!("operator {op:?} is not defined for {ty}"),
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
                    format!("operator {op:?} cannot compare {left_ty} and {right_ty}"),
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
                        format!("operator {op:?} is not defined for {left_ty}"),
                    ));
                }
            };
            let ty = if matches!(
                op,
                BinaryOp::Eq
                    | BinaryOp::NotEq
                    | BinaryOp::Lt
                    | BinaryOp::Gt
                    | BinaryOp::Le
                    | BinaryOp::Ge
                    | BinaryOp::And
                    | BinaryOp::Or
            ) {
                Type::Bool
            } else {
                left_ty
            };
            Some((format!("({lhs}) {operator} ({rhs})"), ty))
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
                    format!("?: condition must be Bool, found {condition_ty}"),
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
                    format!("?: branches have different types: {then_ty} and {else_ty}"),
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
                            return Err(crate::error_at(
                                argument.position,
                                format!(
                                    "argument expects {}, found {}",
                                    type_name(expected),
                                    type_name(&found)
                                ),
                            ));
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
        ExprKind::Number(value) => format!("brul::Value::Number({value:?})"),
        ExprKind::String(text) => format!("brul::Value::String({text:?}.into())"),
        ExprKind::Bool(value) => format!("brul::Value::Bool({value})"),
        ExprKind::Null => "brul::Value::Null".to_owned(),
        ExprKind::Ident(name) => {
            if let Some(read) = scopes.read(name) {
                read
            } else if let Some(read) = state.read(name) {
                read
            } else {
                format!(
                    "component.read_id({})?",
                    names::variable_expr(name, expression.position, names)?
                )
            }
        }
        ExprKind::Field { target, name } => format!(
            "brul::eval::read_field(&({}), {name:?})?",
            operation_in(target, names, state, props, scopes)?
        ),
        ExprKind::Index { target, index } => format!(
            "brul::eval::read_index(&({}), &({}))?",
            operation_in(target, names, state, props, scopes)?,
            operation_in(index, names, state, props, scopes)?
        ),
        ExprKind::Array(items) => format!(
            "brul::Value::Array(vec![{}].into())",
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
            format!("brul::Value::Object(vec![{fields}].into())")
        }
        ExprKind::Template(parts) => {
            let parts = parts
                .iter()
                .map(|part| match part {
                    TemplatePart::Text(text) => Ok(format!("brul::Value::String({text:?}.into())")),
                    TemplatePart::Expr(expression) => {
                        operation_in(expression, names, state, props, scopes)
                    }
                })
                .collect::<Result<Vec<_>, Error>>()?
                .join(", ");
            format!("brul::Value::String(brul::eval::template(vec![{parts}])?)")
        }
        ExprKind::Call { target, args } => call(target, args, names, state, props, scopes)?,
        ExprKind::Unary { op, operand } => format!(
            "brul::eval::apply_unary(brul::ast::UnaryOp::{op:?}, {})?",
            operation_in(operand, names, state, props, scopes)?
        ),
        ExprKind::Binary { op, left, right } => format!(
            "brul::eval::apply_binary(brul::ast::BinaryOp::{op:?}, {}, {})?",
            operation_in(left, names, state, props, scopes)?,
            operation_in(right, names, state, props, scopes)?
        ),
        ExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "match {} {{ brul::Value::Bool(true) => {}, brul::Value::Bool(false) => {}, other => return Err(brul::Error::Expected {{ operation: \"?:\", expected: \"Bool\", found: other.type_name() }}), }}",
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
    if name == "every" {
        return every(arguments, names, state, props, scopes);
    }
    Ok(format!(
        "{{ let arguments = vec![{}]; brul::eval::call(component, {name:?}, arguments)? }}",
        operations(arguments, names, state, props, scopes)?
    ))
}

fn every(
    arguments: &[Expr],
    names: &Names<'_>,
    state: &State,
    props: &HashMap<String, Type>,
    scopes: &Scopes,
) -> Result<String, Error> {
    let [interval, function] = arguments else {
        return Ok(
            "{ return Err(brul::Error::Arity { name: \"every\".to_owned(), expected: \"two\" }) }"
                .to_owned(),
        );
    };
    let ExprKind::Ident(function) = &function.kind else {
        return Ok(format!(
            "{{ return Err(brul::Error::Argument {{ name: \"every\", expected: \"function name\", found: {:?} }}) }}",
            function.kind.name()
        ));
    };
    Ok(format!(
        "match {} {{ brul::Value::Number(interval) => {{ component.every(interval, {function:?})?; brul::Value::Null }}, other => return Err(brul::Error::Argument {{ name: \"every\", expected: \"Number\", found: other.type_name() }}), }}",
        operation_in(interval, names, state, props, scopes)?
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
            "brul::eval::apply_binary(brul::ast::BinaryOp::Add, {}, {})?",
            operation_in(target, names, state, props, scopes)?,
            operation_in(value, names, state, props, scopes)?
        ),
        AssignOp::Sub => format!(
            "brul::eval::apply_binary(brul::ast::BinaryOp::Sub, {}, {})?",
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
                "component.assign_id({}, {value}.clone())?;",
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
            let _ = writeln!(
                out,
                "brul::eval::assign_field(&mut {container}, {name:?}, {value}.clone())?;"
            );
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
            let _ = writeln!(
                out,
                "brul::eval::assign_index(&mut {container}, &{position}, {value}.clone())?;"
            );
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
