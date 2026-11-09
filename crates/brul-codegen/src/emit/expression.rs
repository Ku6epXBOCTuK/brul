use brul_syntax::ast::{Expr, ExprKind, TemplatePart};

pub fn expression(expression: &Expr) -> String {
    let position = format!(
        "brul::ast::Position {{ line: {}, column: {} }}",
        expression.position.line, expression.position.column
    );
    format!(
        "brul::ast::Expr::new({}, {position})",
        expression_kind(&expression.kind)
    )
}

fn expression_kind(kind: &ExprKind) -> String {
    match kind {
        ExprKind::Number(value) => format!("brul::ast::ExprKind::Number({value:?})"),
        ExprKind::String(text) => format!("brul::ast::ExprKind::String({text:?}.to_owned())"),
        ExprKind::Bool(value) => format!("brul::ast::ExprKind::Bool({value})"),
        ExprKind::Null => "brul::ast::ExprKind::Null".to_owned(),
        ExprKind::Ident(name) => format!("brul::ast::ExprKind::Ident({name:?}.to_owned())"),
        ExprKind::Field { target, name } => format!(
            "brul::ast::ExprKind::Field {{ target: Box::new({}), name: {name:?}.to_owned() }}",
            expression(target)
        ),
        ExprKind::Index { target, index } => format!(
            "brul::ast::ExprKind::Index {{ target: Box::new({}), index: Box::new({}) }}",
            expression(target),
            expression(index)
        ),
        ExprKind::Call { target, args } => format!(
            "brul::ast::ExprKind::Call {{ target: Box::new({}), args: vec![{}] }}",
            expression(target),
            expressions(args)
        ),
        ExprKind::Unary { op, operand } => format!(
            "brul::ast::ExprKind::Unary {{ op: brul::ast::UnaryOp::{op:?}, operand: Box::new({}) }}",
            expression(operand)
        ),
        ExprKind::Binary { op, left, right } => format!(
            "brul::ast::ExprKind::Binary {{ op: brul::ast::BinaryOp::{op:?}, left: Box::new({}), right: Box::new({}) }}",
            expression(left),
            expression(right)
        ),
        ExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "brul::ast::ExprKind::Ternary {{ condition: Box::new({}), then_branch: Box::new({}), else_branch: Box::new({}) }}",
            expression(condition),
            expression(then_branch),
            expression(else_branch)
        ),
        ExprKind::Assign { target, op, value } => format!(
            "brul::ast::ExprKind::Assign {{ target: Box::new({}), op: brul::ast::AssignOp::{op:?}, value: Box::new({}) }}",
            expression(target),
            expression(value)
        ),
        ExprKind::Array(items) => {
            format!("brul::ast::ExprKind::Array(vec![{}])", expressions(items))
        }
        ExprKind::Object(fields) => format!(
            "brul::ast::ExprKind::Object(vec![{}])",
            fields
                .iter()
                .map(|(name, value)| format!("({name:?}.to_owned(), {})", expression(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ExprKind::Template(parts) => format!(
            "brul::ast::ExprKind::Template(vec![{}])",
            parts
                .iter()
                .map(|part| match part {
                    TemplatePart::Text(text) => {
                        format!("brul::ast::TemplatePart::Text({text:?}.to_owned())")
                    }
                    TemplatePart::Expr(expression) => format!(
                        "brul::ast::TemplatePart::Expr(Box::new({}))",
                        self::expression(expression)
                    ),
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn expressions(expressions: &[Expr]) -> String {
    expressions
        .iter()
        .map(expression)
        .collect::<Vec<_>>()
        .join(", ")
}
