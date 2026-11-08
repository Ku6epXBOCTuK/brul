use crate::ast::Expr;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub imports: Vec<Import>,
    pub statements: Vec<Statement>,
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
}
