use crate::ast::{Expr, ExprKind, Position, TemplatePart};
use crate::error::Error;
use crate::script::ast::{Program, Statement};

const BUILTINS: [&str; 6] = ["String", "Number", "len", "set_theme", "exit", "every"];

pub fn validate(program: &Program, external: &[&str]) -> Result<(), Error> {
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
        let mut names = Names::new(functions.clone(), external);
        names
            .variables
            .extend(function.params.iter().map(String::as_str));
        names.variables.extend(top_level.iter().copied());
        check_statements(&function.body, &mut names)?;
    }

    let mut names = Names::new(functions, external);
    check_statements(&program.statements, &mut names)
}

struct Names<'a> {
    variables: Vec<&'a str>,
    functions: Vec<&'a str>,
    external: &'a [&'a str],
}

impl<'a> Names<'a> {
    fn new(functions: Vec<&'a str>, external: &'a [&'a str]) -> Self {
        Self {
            variables: Vec::new(),
            functions,
            external,
        }
    }

    fn has_variable(&self, name: &str) -> bool {
        self.variables.contains(&name) || self.external.contains(&name)
    }

    fn has_function(&self, name: &str) -> bool {
        self.functions.contains(&name) || self.external.contains(&name) || BUILTINS.contains(&name)
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
        Statement::Let { name, value, .. } => {
            if let Some(value) = value {
                check_expression(value, names)?;
            }
            names.variables.push(name.as_str());
            Ok(())
        }
        Statement::Expr(expression) => check_expression(expression, names),
        Statement::Block(statements) => check_statements(statements, names),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            check_expression(condition, names)?;
            check_statements(then_branch, names)?;
            if let Some(else_branch) = else_branch {
                check_statement(else_branch, names)?;
            }
            Ok(())
        }
    }
}

fn check_expression(expression: &Expr, names: &Names<'_>) -> Result<(), Error> {
    match &expression.kind {
        ExprKind::Number(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::Null => Ok(()),
        ExprKind::Ident(name) => check_variable(name, expression.position, names),
        ExprKind::Field { target, .. } => check_expression(target, names),
        ExprKind::Index { target, index } => {
            check_expression(target, names)?;
            check_expression(index, names)
        }
        ExprKind::Call { target, args } => {
            match &target.kind {
                ExprKind::Ident(name) if !names.has_function(name) => {
                    return Err(undefined_function(name, target.position));
                }
                ExprKind::Ident(_) => {}
                _ => check_expression(target, names)?,
            }
            for (index, argument) in args.iter().enumerate() {
                if index == 1
                    && matches!(&target.kind, ExprKind::Ident(name) if name == "every")
                    && let ExprKind::Ident(function) = &argument.kind
                {
                    if !names.has_function(function) {
                        return Err(undefined_function(function, argument.position));
                    }
                    continue;
                }
                check_expression(argument, names)?;
            }
            Ok(())
        }
        ExprKind::Assign { target, value, .. } => {
            check_target(target, names)?;
            check_expression(value, names)
        }
        ExprKind::Unary { operand, .. } => check_expression(operand, names),
        ExprKind::Binary { left, right, .. } => {
            check_expression(left, names)?;
            check_expression(right, names)
        }
        ExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            check_expression(condition, names)?;
            check_expression(then_branch, names)?;
            check_expression(else_branch, names)
        }
        ExprKind::Array(items) => {
            for item in items {
                check_expression(item, names)?;
            }
            Ok(())
        }
        ExprKind::Object(fields) => {
            for (_, value) in fields {
                check_expression(value, names)?;
            }
            Ok(())
        }
        ExprKind::Template(parts) => {
            for part in parts {
                if let TemplatePart::Expr(expression) = part {
                    check_expression(expression, names)?;
                }
            }
            Ok(())
        }
    }
}

fn check_target(target: &Expr, names: &Names<'_>) -> Result<(), Error> {
    match &target.kind {
        ExprKind::Ident(name) => check_variable(name, target.position, names),
        ExprKind::Field { target, .. } => check_expression(target, names),
        ExprKind::Index { target, index } => {
            check_expression(target, names)?;
            check_expression(index, names)
        }
        _ => check_expression(target, names),
    }
}

fn check_variable(name: &str, position: Position, names: &Names<'_>) -> Result<(), Error> {
    if names.has_variable(name) {
        Ok(())
    } else {
        Err(Error {
            message: format!("variable '{name}' is not defined"),
            line: position.line,
            column: position.column,
        })
    }
}

fn undefined_function(name: &str, position: Position) -> Error {
    Error {
        message: format!("function '{name}' is not defined"),
        line: position.line,
        column: position.column,
    }
}
