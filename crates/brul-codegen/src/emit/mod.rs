use std::fmt::Write;

use brul_gui::TextStyle;
use brul_syntax::ast::ExprKind;
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, BlockItem, Element};
use brul_syntax::style::Stylesheet;

use crate::error_at;
use crate::styles;

mod expression;
mod style;

pub use expression::expression;
use style::{layout, style_set, text_style};

pub fn module(source: &str) -> Result<String, Error> {
    let parsed = crate::parse_source(source)?;
    let root = crate::root(&parsed.window)?;
    let mut out = String::new();
    out.push_str("pub fn view(theme: Option<&str>) -> Result<brul_gui::Widget, String> {\n");
    out.push_str("    match theme {\n");
    branch(&mut out, "None", root, &parsed.stylesheet, None)?;
    for theme in &parsed.stylesheet.themes {
        let pattern = format!("Some({:?})", theme.name);
        branch(
            &mut out,
            &pattern,
            root,
            &parsed.stylesheet,
            Some(&theme.name),
        )?;
    }
    out.push_str("        Some(theme) => Err(format!(\"unknown theme '{theme}'\")),\n");
    out.push_str("    }\n}\n");
    Ok(out)
}

fn branch(
    out: &mut String,
    pattern: &str,
    root: &Element,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
) -> Result<(), Error> {
    let _ = writeln!(out, "        {pattern} => Ok(");
    widget(out, root, stylesheet, theme, 3)?;
    out.push_str(",\n        ),\n");
    Ok(())
}

fn widget(
    out: &mut String,
    element: &Element,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
    depth: usize,
) -> Result<(), Error> {
    let classes: Vec<&str> = element
        .args
        .iter()
        .filter_map(|arg| match arg {
            Arg::Class(class) => Some(class.as_str()),
            Arg::Named { .. } | Arg::Handler { .. } => None,
        })
        .collect();
    let resolved = styles::resolve(stylesheet, &classes, theme)
        .map_err(|message| error_at(element.position, message))?;
    let pad = "    ".repeat(depth);
    let _ = write!(out, "{pad}");
    match element.name.as_str() {
        "View" | "Row" | "Column" => {
            let call = if element.name == "Row" {
                "row"
            } else {
                "column"
            };
            if element.block.items.is_empty() {
                let _ = write!(out, "brul_gui::Widget::{call}(vec![])");
            } else {
                let _ = writeln!(out, "brul_gui::Widget::{call}(vec![");
                for item in &element.block.items {
                    let BlockItem::Element(child) = item else {
                        return Err(error_at(
                            element.position,
                            "only elements are generated yet",
                        ));
                    };
                    widget(out, child, stylesheet, theme, depth + 1)?;
                    out.push_str(",\n");
                }
                let _ = write!(out, "{pad}])");
            }
        }
        "Text" => text(out, element, resolved.text)?,
        name => {
            return Err(error_at(
                element.position,
                format!("widget '{name}' is not generated yet"),
            ));
        }
    }
    if let Some(layout) = layout(resolved.layout) {
        let _ = write!(out, "\n{pad}.with_layout({layout})");
    }
    if let Some(styles) = style_set(resolved.styles) {
        let _ = write!(out, "\n{pad}.with_styles({styles})");
    }
    Ok(())
}

fn text(out: &mut String, element: &Element, style: TextStyle) -> Result<(), Error> {
    match element.block.items.as_slice() {
        [BlockItem::Expr(expression)] => {
            let ExprKind::String(content) = &expression.kind else {
                return Err(error_at(
                    expression.position,
                    "generated Text supports only string literals yet",
                ));
            };
            let _ = match text_style(style) {
                Some(style) => {
                    write!(out, "brul_gui::Widget::styled_text({content:?}, {style})")
                }
                None => write!(out, "brul_gui::Widget::text({content:?})"),
            };
            Ok(())
        }
        _ => Err(error_at(
            element.position,
            "Text requires a single content expression",
        )),
    }
}
