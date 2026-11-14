use brul_syntax::ast::BinaryOp;

use crate::error::Error;
use crate::value::Value;

pub(super) fn mismatch(op: BinaryOp, left: &Value, right: &Value) -> Error {
    Error::Mismatch {
        operation: op_name(op),
        left: left.type_name(),
        right: right.type_name(),
    }
}

pub(super) fn expected(operation: &'static str, expected: &'static str, found: &Value) -> Error {
    Error::Expected {
        operation,
        expected,
        found: found.type_name(),
    }
}

pub(super) fn argument_error(name: &'static str, expected: &'static str, found: &Value) -> Error {
    Error::Argument {
        name,
        expected,
        found: found.type_name(),
    }
}

fn op_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::Eq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Gt => ">",
        BinaryOp::Le => "<=",
        BinaryOp::Ge => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_name_spells_every_operator() {
        let named = [
            (BinaryOp::Add, "+"),
            (BinaryOp::Sub, "-"),
            (BinaryOp::Mul, "*"),
            (BinaryOp::Div, "/"),
            (BinaryOp::Rem, "%"),
            (BinaryOp::Eq, "=="),
            (BinaryOp::NotEq, "!="),
            (BinaryOp::Lt, "<"),
            (BinaryOp::Gt, ">"),
            (BinaryOp::Le, "<="),
            (BinaryOp::Ge, ">="),
            (BinaryOp::And, "&&"),
            (BinaryOp::Or, "||"),
        ];
        for (op, name) in named {
            assert_eq!(op_name(op), name);
        }
    }
}
