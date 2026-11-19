use std::sync::Arc;

use crate::ast::Expr;
use crate::types::Type;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub functions: Vec<Arc<Function>>,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let {
        constant: bool,
        name: String,
        ty: Type,
        value: Option<Expr>,
    },
    Expr(Expr),
    Block(Vec<Statement>),
    If {
        condition: Expr,
        then_branch: Vec<Statement>,
        else_branch: Option<Box<Statement>>,
    },
}
