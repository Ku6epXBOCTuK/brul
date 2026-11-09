use brul_gui::{Direction, Widget};
use brul_syntax::ast::{ExprKind, Position};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, BlockItem, Element};
use brul_syntax::style::Stylesheet;

use crate::styles::{self, Resolved};

pub fn build_tree(
    element: &Element,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
) -> Result<Widget, Error> {
    let resolved = resolve(element, stylesheet, theme)?;

    match element.name.as_str() {
        "View" => container(Direction::Column, element, stylesheet, theme, resolved),
        "Row" => container(Direction::Row, element, stylesheet, theme, resolved),
        "Column" => container(Direction::Column, element, stylesheet, theme, resolved),
        "Text" => text(element, resolved),
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

fn resolve(
    element: &Element,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
) -> Result<Resolved, Error> {
    let mut classes = Vec::new();
    for arg in &element.args {
        match arg {
            Arg::Class(class) => classes.push(class.as_str()),
            _ => {
                return Err(error_at(
                    element.position,
                    "element arguments are not supported yet",
                ));
            }
        }
    }
    styles::resolve(stylesheet, &classes, theme)
        .map_err(|message| error_at(element.position, message))
}

fn container(
    direction: Direction,
    element: &Element,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
    resolved: Resolved,
) -> Result<Widget, Error> {
    let mut children = Vec::with_capacity(element.block.items.len());
    for item in &element.block.items {
        match item {
            BlockItem::Element(child) => children.push(build_tree(child, stylesheet, theme)?),
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
    Ok(Widget::view(direction, children)
        .with_layout(resolved.layout)
        .with_styles(resolved.styles))
}

fn text(element: &Element, resolved: Resolved) -> Result<Widget, Error> {
    match element.block.items.as_slice() {
        [BlockItem::Expr(expression)] => match &expression.kind {
            ExprKind::String(content) => Ok(Widget::styled_text(content.clone(), resolved.text)
                .with_layout(resolved.layout)
                .with_styles(resolved.styles)),
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
