use brul_syntax::script::{Program, Statement};

use crate::component::Component;
use crate::error::Error;
use crate::eval::eval;
use crate::value::Value;

pub fn execute_program(component: &mut Component, program: &Program) -> Result<(), Error> {
    for function in &program.functions {
        component.define_function(function.clone())?;
    }
    execute(component, &program.statements)
}

pub fn execute(component: &mut Component, statements: &[Statement]) -> Result<(), Error> {
    for statement in statements {
        execute_statement(component, statement)?;
    }
    Ok(())
}

pub fn execute_statement(component: &mut Component, statement: &Statement) -> Result<(), Error> {
    match statement {
        Statement::Let {
            constant,
            name,
            value,
        } => {
            let value = match value {
                Some(value) => eval(component, value)?,
                None => Value::Null,
            };
            component.declare(name, value, *constant)
        }
        Statement::Expr(expression) => {
            eval(component, expression)?;
            Ok(())
        }
        Statement::Block(statements) => execute(component, statements),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition = match eval(component, condition)? {
                Value::Bool(condition) => condition,
                other => {
                    return Err(Error::Expected {
                        operation: "if",
                        expected: "Bool",
                        found: other.type_name(),
                    }
                    .at(condition.position));
                }
            };
            if condition {
                execute(component, then_branch)
            } else if let Some(else_branch) = else_branch {
                execute_statement(component, else_branch)
            } else {
                Ok(())
            }
        }
    }
}
