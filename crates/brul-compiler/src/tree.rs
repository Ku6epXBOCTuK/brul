use brul_gui::{Direction, Widget};
use brul_syntax::ast::{ExprKind, Position};
use brul_syntax::error::Error;
use brul_syntax::markup::{BlockItem, Element};

pub fn build_tree(element: &Element) -> Result<Widget, Error> {
    if !element.args.is_empty() {
        return Err(error_at(
            element.position,
            "element arguments are not supported yet",
        ));
    }

    match element.name.as_str() {
        "View" => container(Direction::Column, element),
        "Row" => container(Direction::Row, element),
        "Column" => container(Direction::Column, element),
        "Text" => text(element),
        "Image" => Err(error_at(
            element.position,
            "widget 'Image' is not supported yet",
        )),
        name => Err(error_at(
            element.position,
            format!("unknown widget '{name}'"),
        )),
    }
}

fn container(direction: Direction, element: &Element) -> Result<Widget, Error> {
    let mut children = Vec::with_capacity(element.block.items.len());
    for item in &element.block.items {
        match item {
            BlockItem::Element(child) => children.push(build_tree(child)?),
            BlockItem::If(_) => {
                return Err(error_at(element.position, "'if' is not supported yet"));
            }
            BlockItem::Each(_) => {
                return Err(error_at(element.position, "'each' is not supported yet"));
            }
            BlockItem::Expr(expression) => {
                return Err(error_at(
                    expression.position,
                    "unexpected expression in a container",
                ));
            }
        }
    }
    Ok(Widget::view(direction, children))
}

fn text(element: &Element) -> Result<Widget, Error> {
    match element.block.items.as_slice() {
        [BlockItem::Expr(expression)] => match &expression.kind {
            ExprKind::String(content) => Ok(Widget::text(content.clone())),
            _ => Err(error_at(
                expression.position,
                "Text content must be a string literal",
            )),
        },
        _ => Err(error_at(
            element.position,
            "Text requires a single content expression",
        )),
    }
}

fn error_at(position: Position, message: impl Into<String>) -> Error {
    Error {
        message: message.into(),
        line: position.line,
        column: position.column,
    }
}
