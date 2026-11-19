use std::fmt;

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

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
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

    pub fn children(&self) -> impl Iterator<Item = &Expr> + '_ {
        let fixed: [Option<&Expr>; 3] = match self {
            Self::Field { target, .. }
            | Self::Unary {
                operand: target, ..
            } => [Some(target), None, None],
            Self::Index { target, index }
            | Self::Binary {
                left: target,
                right: index,
                ..
            } => [Some(target), Some(index), None],
            Self::Ternary {
                condition,
                then_branch,
                else_branch,
            } => [Some(condition), Some(then_branch), Some(else_branch)],
            Self::Assign { target, value, .. } => [Some(target), Some(value), None],
            Self::Call { target, .. } => [Some(target), None, None],
            _ => [None, None, None],
        };
        let variable: Box<dyn Iterator<Item = &Expr> + '_> = match self {
            Self::Call { args, .. } | Self::Array(args) => Box::new(args.iter()),
            Self::Object(fields) => Box::new(fields.iter().map(|(_, value)| value)),
            Self::Template(parts) => Box::new(parts.iter().filter_map(|part| match part {
                TemplatePart::Expr(expr) => Some(expr.as_ref()),
                TemplatePart::Text(_) => None,
            })),
            _ => Box::new(std::iter::empty()),
        };
        fixed.into_iter().flatten().chain(variable)
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

    #[test]
    fn expr_kind_children_returns_every_child() {
        let ident = |name: &str| Expr::new(ExprKind::Ident(name.to_owned()), POSITION);
        fn child_names(kind: &ExprKind) -> Vec<&str> {
            kind.children()
                .map(|child| match &child.kind {
                    ExprKind::Ident(name) => name.as_str(),
                    _ => panic!("expected identifier"),
                })
                .collect()
        }

        assert!(child_names(&ExprKind::Number(1.0)).is_empty());
        assert!(child_names(&ExprKind::Ident("x".to_owned())).is_empty());
        assert_eq!(
            child_names(&ExprKind::Field {
                target: Box::new(ident("a")),
                name: "a".to_owned(),
            }),
            ["a"]
        );
        assert_eq!(
            child_names(&ExprKind::Index {
                target: Box::new(ident("a")),
                index: Box::new(ident("b")),
            }),
            ["a", "b"]
        );
        assert_eq!(
            child_names(&ExprKind::Call {
                target: Box::new(ident("a")),
                args: vec![ident("b"), ident("c")],
            }),
            ["a", "b", "c"]
        );
        assert_eq!(
            child_names(&ExprKind::Unary {
                op: UnaryOp::Not,
                operand: Box::new(ident("a")),
            }),
            ["a"]
        );
        assert_eq!(
            child_names(&ExprKind::Binary {
                op: BinaryOp::Add,
                left: Box::new(ident("a")),
                right: Box::new(ident("b")),
            }),
            ["a", "b"]
        );
        assert_eq!(
            child_names(&ExprKind::Ternary {
                condition: Box::new(ident("a")),
                then_branch: Box::new(ident("b")),
                else_branch: Box::new(ident("c")),
            }),
            ["a", "b", "c"]
        );
        assert_eq!(
            child_names(&ExprKind::Assign {
                target: Box::new(ident("a")),
                op: AssignOp::Set,
                value: Box::new(ident("b")),
            }),
            ["a", "b"]
        );
        assert_eq!(
            child_names(&ExprKind::Array(vec![ident("a"), ident("b")])),
            ["a", "b"]
        );
        assert_eq!(
            child_names(&ExprKind::Object(vec![
                ("a".to_owned(), ident("a")),
                ("b".to_owned(), ident("b")),
            ])),
            ["a", "b"]
        );
        assert_eq!(
            child_names(&ExprKind::Template(vec![
                TemplatePart::Text("x".to_owned()),
                TemplatePart::Expr(Box::new(ident("a"))),
            ])),
            ["a"]
        );
    }
}
