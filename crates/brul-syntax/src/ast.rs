#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
    Ident(String),
    Field { target: Box<Expr>, name: String },
    Index { target: Box<Expr>, index: Box<Expr> },
    Call { target: Box<Expr>, args: Vec<Expr> },
}
