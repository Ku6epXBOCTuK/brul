use crate::ast::Expr;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub imports: Vec<Import>,
    pub functions: Vec<Function>,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let {
        constant: bool,
        name: String,
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
