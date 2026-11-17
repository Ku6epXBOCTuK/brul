#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    NotEq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Set,
    Add,
    Sub,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    Text(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub position: Position,
}

impl Expr {
    pub fn new(kind: ExprKind, position: Position) -> Self {
        Self { kind, position }
    }
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
    Ident(String),
    Field {
        target: Box<Expr>,
        name: String,
    },
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    Call {
        target: Box<Expr>,
        args: Vec<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Ternary {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Assign {
        target: Box<Expr>,
        op: AssignOp,
        value: Box<Expr>,
    },
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>),
    Template(Vec<TemplatePart>),
}

impl ExprKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ident(_) => "identifier",
            Self::Field { .. } => "field access",
            Self::Index { .. } => "index access",
            Self::Call { .. } => "call",
            Self::Assign { .. } => "assignment",
            Self::Array(_) => "array literal",
            Self::Object(_) => "object literal",
            Self::Template(_) => "template string",
            Self::Number(_)
            | Self::String(_)
            | Self::Bool(_)
            | Self::Null
            | Self::Unary { .. }
            | Self::Binary { .. }
            | Self::Ternary { .. } => "expression",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POSITION: Position = Position { line: 1, column: 1 };

    fn ident() -> Expr {
        Expr::new(ExprKind::Ident("x".to_owned()), POSITION)
    }

    #[test]
    fn expr_kind_names_every_variant() {
        let target = || Box::new(ident());
        let named = [
            (ExprKind::Ident("x".to_owned()), "identifier"),
            (
                ExprKind::Field {
                    target: target(),
                    name: "x".to_owned(),
                },
                "field access",
            ),
            (
                ExprKind::Index {
                    target: target(),
                    index: target(),
                },
                "index access",
            ),
            (
                ExprKind::Call {
                    target: target(),
                    args: Vec::new(),
                },
                "call",
            ),
            (
                ExprKind::Assign {
                    target: target(),
                    op: AssignOp::Set,
                    value: target(),
                },
                "assignment",
            ),
            (ExprKind::Array(Vec::new()), "array literal"),
            (ExprKind::Object(Vec::new()), "object literal"),
            (ExprKind::Template(Vec::new()), "template string"),
            (ExprKind::Number(1.0), "expression"),
            (ExprKind::String(String::new()), "expression"),
            (ExprKind::Bool(true), "expression"),
            (ExprKind::Null, "expression"),
            (
                ExprKind::Unary {
                    op: UnaryOp::Not,
                    operand: target(),
                },
                "expression",
            ),
            (
                ExprKind::Binary {
                    op: BinaryOp::Add,
                    left: target(),
                    right: target(),
                },
                "expression",
            ),
            (
                ExprKind::Ternary {
                    condition: target(),
                    then_branch: target(),
                    else_branch: target(),
                },
                "expression",
            ),
        ];
        for (kind, name) in named {
            assert_eq!(kind.name(), name);
        }
    }
}
