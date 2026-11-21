use crate::TypeName;
use crate::ast::{Expr, ExprKind, Position};
use crate::error::Error;
use crate::script::ast::{Program, Statement};
use crate::script::builtins::{self, Argument, Builtin};
use crate::types::Type;

pub fn validate(program: &Program, external: &[&str]) -> Result<(), Error> {
    validate_inner(program, external, &[])
}

pub fn validate_build(
    program: &Program,
    external: &[&str],
    commands: &[&str],
) -> Result<(), Error> {
    validate_inner(program, external, commands)
}

fn validate_inner(program: &Program, external: &[&str], commands: &[&str]) -> Result<(), Error> {
    let functions: Vec<&str> = program
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    let top_level: Vec<&str> = program
        .statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::Let { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();

    for function in &program.functions {
        let mut names = Names::new(functions.clone(), external, commands);
        names
            .variables
            .extend(function.params.iter().map(|param| param.name.as_str()));
        names.variables.extend(top_level.iter().copied());
        names.in_function = true;
        check_statements(&function.body, &mut names)?;
    }

    let mut names = Names::new(functions, external, commands);
    check_statements(&program.statements, &mut names)
}

struct Names<'a> {
    variables: Vec<&'a str>,
    functions: Vec<&'a str>,
    external: &'a [&'a str],
    commands: &'a [&'a str],
    in_function: bool,
}

impl<'a> Names<'a> {
    fn new(functions: Vec<&'a str>, external: &'a [&'a str], commands: &'a [&'a str]) -> Self {
        Self {
            variables: Vec::new(),
            functions,
            external,
            commands,
            in_function: false,
        }
    }

    fn has_variable(&self, name: &str) -> bool {
        self.variables.contains(&name) || self.external.contains(&name)
    }

    fn has_function(&self, name: &str) -> bool {
        self.functions.contains(&name)
            || self.external.contains(&name)
            || builtins::builtin(name).is_some()
            || self.commands.contains(&name)
    }
}

fn check_statements<'a>(statements: &'a [Statement], names: &mut Names<'a>) -> Result<(), Error> {
    for statement in statements {
        check_statement(statement, names)?;
    }
    Ok(())
}

fn check_statement<'a>(statement: &'a Statement, names: &mut Names<'a>) -> Result<(), Error> {
    match statement {
        Statement::Let { name, value, .. } => check_let(name, value.as_ref(), names),
        Statement::Expr(expression) => check_expression(expression, names),
        Statement::Block(statements) => check_statements(statements, names),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => check_if(condition, then_branch, else_branch.as_deref(), names),
        Statement::Return { value, position } => {
            if !names.in_function {
                return Err(Error::at_position(*position, Error::ReturnOutsideFunction));
            }
            if let Some(value) = value {
                check_expression(value, names)
            } else {
                Ok(())
            }
        }
    }
}

fn check_let<'a>(
    name: &'a str,
    value: Option<&'a Expr>,
    names: &mut Names<'a>,
) -> Result<(), Error> {
    if let Some(value) = value {
        check_expression(value, names)?;
    }
    names.variables.push(name);
    Ok(())
}

fn check_if<'a>(
    condition: &'a Expr,
    then_branch: &'a [Statement],
    else_branch: Option<&'a Statement>,
    names: &mut Names<'a>,
) -> Result<(), Error> {
    check_expression(condition, names)?;
    check_statements(then_branch, names)?;
    if let Some(else_branch) = else_branch {
        check_statement(else_branch, names)?;
    }
    Ok(())
}

fn check_expression(expression: &Expr, names: &Names<'_>) -> Result<(), Error> {
    match &expression.kind {
        ExprKind::Number(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::Null => Ok(()),
        ExprKind::Ident(name) => check_variable(name, expression.position, names),
        ExprKind::Call { target, args } => check_call(target, args, names),
        _ => {
            for child in expression.kind.children() {
                check_expression(child, names)?;
            }
            Ok(())
        }
    }
}

fn check_call(target: &Expr, args: &[Expr], names: &Names<'_>) -> Result<(), Error> {
    check_callee(target, names)?;
    if let ExprKind::Ident(name) = &target.kind
        && let Some(builtin) = builtins::builtin(name)
    {
        if args.len() != builtin.arguments.len() {
            return Err(Error::at_position(
                target.position,
                Error::custom(format!(
                    "'{name}' expects {} argument(s), found {}",
                    builtin.arguments.len(),
                    args.len()
                )),
            ));
        }
        for (argument, expected) in args.iter().zip(builtin.arguments) {
            let Argument::Value(ty) = expected else {
                continue;
            };
            if let Some(found) = literal_type(argument)
                && &found != ty
            {
                return Err(Error::at_position(
                    argument.position,
                    Error::custom(format!(
                        "'{name}' expects {}, found {}",
                        ty.type_name(),
                        found.type_name()
                    )),
                ));
            }
        }
    }
    check_arguments(target, args, names)
}

fn literal_type(expression: &Expr) -> Option<Type> {
    match &expression.kind {
        ExprKind::Number(_) => Some(Type::Number),
        ExprKind::String(_) => Some(Type::String),
        ExprKind::Bool(_) => Some(Type::Bool),
        ExprKind::Null => Some(Type::Null),
        _ => None,
    }
}

fn check_callee(target: &Expr, names: &Names<'_>) -> Result<(), Error> {
    match &target.kind {
        ExprKind::Ident(name) if !names.has_function(name) => {
            Err(undefined("function", name, target.position))
        }
        ExprKind::Ident(_) => Ok(()),
        _ => check_expression(target, names),
    }
}

fn check_arguments(target: &Expr, args: &[Expr], names: &Names<'_>) -> Result<(), Error> {
    let function_argument = match &target.kind {
        ExprKind::Ident(name) => builtins::builtin(name).and_then(Builtin::function_argument),
        _ => None,
    };
    for (index, argument) in args.iter().enumerate() {
        check_argument(function_argument == Some(index), argument, names)?;
    }
    Ok(())
}

fn check_argument(
    function_position: bool,
    argument: &Expr,
    names: &Names<'_>,
) -> Result<(), Error> {
    if function_position && let ExprKind::Ident(name) = &argument.kind {
        return check_function(name, argument.position, names);
    }
    check_expression(argument, names)
}

fn check_variable(name: &str, position: Position, names: &Names<'_>) -> Result<(), Error> {
    if names.has_variable(name) {
        Ok(())
    } else {
        Err(undefined("variable", name, position))
    }
}

fn check_function(name: &str, position: Position, names: &Names<'_>) -> Result<(), Error> {
    if names.has_function(name) {
        Ok(())
    } else {
        Err(undefined("function", name, position))
    }
}

fn undefined(what: &str, name: &str, position: Position) -> Error {
    Error::at_position(position, Error::Undefined(what.to_owned(), name.to_owned()))
}
