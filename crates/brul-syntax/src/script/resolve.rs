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
        Statement::Let { name, value, .. } => check_let(name, value.as_ref(), names),
        Statement::Expr(expression) => check_expression(expression, names),
        Statement::Block(statements) => check_statements(statements, names),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => check_if(condition, then_branch, else_branch.as_deref(), names),
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
        _ => check_children(expression, names),
    }
}

fn check_children(expression: &Expr, names: &Names<'_>) -> Result<(), Error> {
    for child in children(&expression.kind) {
        check_expression(child, names)?;
    }
    Ok(())
}

fn children(kind: &ExprKind) -> Vec<&Expr> {
    match kind {
        ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Bool(_)
        | ExprKind::Null
        | ExprKind::Ident(_)
        | ExprKind::Call { .. } => Vec::new(),
        ExprKind::Field { target, .. }
        | ExprKind::Unary {
            operand: target, ..
        } => vec![target],
        ExprKind::Index { target, index }
        | ExprKind::Binary {
            left: target,
            right: index,
            ..
        } => vec![target, index],
        ExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => vec![condition, then_branch, else_branch],
        ExprKind::Assign { target, value, .. } => vec![target, value],
        ExprKind::Array(items) => items.iter().collect(),
        ExprKind::Object(fields) => fields.iter().map(|(_, value)| value).collect(),
        ExprKind::Template(parts) => parts.iter().filter_map(template_expression).collect(),
    }
}

fn template_expression(part: &TemplatePart) -> Option<&Expr> {
    match part {
        TemplatePart::Expr(expression) => Some(expression),
        TemplatePart::Text(_) => None,
    }
}

fn check_call(target: &Expr, args: &[Expr], names: &Names<'_>) -> Result<(), Error> {
    check_callee(target, names)?;
    check_arguments(target, args, names)
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
    let every = is_every(target);
    for (index, argument) in args.iter().enumerate() {
        check_argument(every && index == 1, argument, names)?;
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

fn is_every(target: &Expr) -> bool {
    matches!(&target.kind, ExprKind::Ident(name) if name == "every")
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
    Error {
        message: format!("{what} '{name}' is not defined"),
        line: position.line,
        column: position.column,
    }
}
