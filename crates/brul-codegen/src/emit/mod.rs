use std::fmt::Write;

use brul_gui::TextStyle;
use brul_syntax::ast::{Expr, ExprKind};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Block, BlockItem, Element};
use brul_syntax::script::Program;
use brul_syntax::style::Stylesheet;

use crate::error_at;
use crate::styles;

mod events;
mod expression;
mod script;
mod structure;
mod style;

pub use expression::expression;
use style::{layout, style_set, text_style};

pub(super) struct Emit<'a> {
    stylesheet: &'a Stylesheet,
    program: Option<&'a Program>,
    functions: Vec<String>,
    next_structure: usize,
}

pub fn module(source: &str) -> Result<String, Error> {
    let parsed = crate::parse_source(source)?;
    let (config, window_handlers) = crate::arguments(&parsed.window)?;
    let window_handlers = events::window(&window_handlers, parsed.program.as_ref())?;
    let root = crate::root(&parsed.window)?;
    let mut emit = Emit {
        stylesheet: &parsed.stylesheet,
        program: parsed.program.as_ref(),
        functions: Vec::new(),
        next_structure: 0,
    };
    let mut out = String::new();
    out.push_str("#[allow(unused_mut, unused_variables, clippy::let_and_return)]\n");
    out.push_str("pub fn view(theme: Option<&str>, component: &brul::ComponentHandle) -> Result<brul::Built, String> {\n");
    out.push_str("    let mut bindings = Vec::new();\n");
    out.push_str("    let mut handlers = Vec::new();\n");
    out.push_str("    let mut structures = Vec::new();\n");
    out.push_str("    let mut props = Vec::new();\n");
    out.push_str("    let widget = match theme {\n");
    branch(&mut out, "None", root, &mut emit, None)?;
    for theme in &parsed.stylesheet.themes {
        let pattern = format!("Some({:?})", theme.name);
        branch(&mut out, &pattern, root, &mut emit, Some(&theme.name))?;
    }
    out.push_str("        Some(theme) => return Err(format!(\"unknown theme '{theme}'\")),\n");
    out.push_str("    };\n");
    for (event, handler) in &window_handlers {
        let _ = writeln!(
            out,
            "    handlers.push(brul::Handler::new(component, None, brul::HandlerEvent::{event:?}, {handler:?}));"
        );
    }
    out.push_str("    Ok(brul::Built {\n");
    out.push_str("        widget,\n");
    out.push_str("        bindings,\n");
    out.push_str("        handlers,\n");
    out.push_str("        structures,\n");
    out.push_str("        props,\n");
    out.push_str("    })\n}\n");
    for function in &emit.functions {
        out.push('\n');
        out.push_str(function);
    }
    out.push('\n');
    window_config(&mut out, &config);
    out.push_str(&script::program(parsed.program.as_ref()));
    Ok(out)
}

fn window_config(out: &mut String, config: &brul_gui::window::WindowConfig) {
    let _ = write!(
        out,
        "pub fn window() -> brul::WindowConfig {{
    brul::WindowConfig {{
        title: {:?}.to_owned(),
        width: {},
        height: {},
        ..brul::WindowConfig::default()
    }}
}}
",
        config.title, config.width, config.height
    );
}

fn branch(
    out: &mut String,
    pattern: &str,
    root: &Element,
    emit: &mut Emit<'_>,
    theme: Option<&str>,
) -> Result<(), Error> {
    let mut counter = 0;
    let _ = writeln!(out, "        {pattern} => {{");
    widget(out, root, emit, theme, 3, &mut counter)?;
    out.push_str("\n        }\n");
    Ok(())
}

fn widget(
    out: &mut String,
    element: &Element,
    emit: &mut Emit<'_>,
    theme: Option<&str>,
    depth: usize,
    counter: &mut usize,
) -> Result<(), Error> {
    let classes: Vec<&str> = element
        .args
        .iter()
        .filter_map(|arg| match arg {
            Arg::Class(class) => Some(class.as_str()),
            Arg::Named { .. } | Arg::Handler { .. } => None,
        })
        .collect();
    let resolved = styles::resolve(emit.stylesheet, &classes, theme)
        .map_err(|message| error_at(element.position, message))?;
    let events = events::widget(element, emit.program)?;
    let name = format!("widget{}", *counter);
    *counter += 1;
    let pad = "    ".repeat(depth);
    let inner = "    ".repeat(depth + 1);
    let _ = writeln!(out, "{pad}{{");
    let _ = write!(out, "{inner}let {name} = ");
    let mut text = None;
    let direction = if element.name == "Row" {
        "Row"
    } else {
        "Column"
    };
    let spacing = resolved.layout.spacing;
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
                items(
                    out,
                    &element.block,
                    emit,
                    theme,
                    direction,
                    spacing,
                    depth + 2,
                    counter,
                )?;
                let _ = write!(out, "{inner}])");
            }
        }
        "Text" => text = text_widget(out, element, resolved.text)?,
        name => {
            return Err(error_at(
                element.position,
                format!("widget '{name}' is not generated yet"),
            ));
        }
    }
    if let Some(layout) = layout(resolved.layout) {
        let _ = write!(out, "\n{inner}.with_layout({layout})");
    }
    if let Some(styles) = style_set(resolved.styles) {
        let _ = write!(out, "\n{inner}.with_styles({styles})");
    }
    if events.drag {
        let _ = write!(out, "\n{inner}.draggable(true)");
    }
    out.push_str(";\n");
    if let Some(text_expression) = text {
        let _ = writeln!(
            out,
            "{inner}bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::Text({})));",
            expression(&text_expression)
        );
    }
    parameters(out, element, &name, &inner)?;
    for (event, handler) in &events.handlers {
        let _ = writeln!(
            out,
            "{inner}handlers.push(brul::Handler::new(component, Some({name}.id()), brul::HandlerEvent::{event:?}, {handler:?}));"
        );
    }
    let _ = writeln!(out, "{inner}{name}");
    let _ = write!(out, "{pad}}}");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn items(
    out: &mut String,
    block: &Block,
    emit: &mut Emit<'_>,
    theme: Option<&str>,
    direction: &str,
    spacing: u32,
    depth: usize,
    counter: &mut usize,
) -> Result<(), Error> {
    let theme_expression = match theme {
        Some(name) => format!("Some({name:?})"),
        None => "None".to_owned(),
    };
    for item in &block.items {
        match item {
            BlockItem::Element(child) => {
                widget(out, child, emit, theme, depth, counter)?;
                out.push_str(",\n");
            }
            BlockItem::If(structure) => {
                structure::structure_if(
                    out,
                    structure,
                    emit,
                    direction,
                    spacing,
                    &theme_expression,
                    depth,
                    counter,
                )?;
                out.push_str(",\n");
            }
            BlockItem::Each(structure) => {
                return Err(error_at(
                    structure.collection.position,
                    "each is not generated yet",
                ));
            }
            BlockItem::Expr(expression) => {
                return Err(error_at(
                    expression.position,
                    "unexpected expression in a container",
                ));
            }
        }
    }
    Ok(())
}

fn text_widget(
    out: &mut String,
    element: &Element,
    style: TextStyle,
) -> Result<Option<Expr>, Error> {
    match element.block.items.as_slice() {
        [BlockItem::Expr(expression)] => {
            let ExprKind::String(content) = &expression.kind else {
                let _ = match text_style(style) {
                    Some(style) => write!(out, "brul_gui::Widget::styled_text(\"\", {style})"),
                    None => write!(out, "brul_gui::Widget::text(\"\")"),
                };
                return Ok(Some(expression.clone()));
            };
            let _ = match text_style(style) {
                Some(style) => write!(out, "brul_gui::Widget::styled_text({content:?}, {style})"),
                None => write!(out, "brul_gui::Widget::text({content:?})"),
            };
            Ok(None)
        }
        _ => Err(error_at(
            element.position,
            "Text requires a single content expression",
        )),
    }
}

fn parameters(out: &mut String, element: &Element, name: &str, pad: &str) -> Result<(), Error> {
    let mut x = None;
    let mut y = None;
    for arg in &element.args {
        let Arg::Named {
            name: parameter,
            value,
        } = arg
        else {
            continue;
        };
        let target = match parameter.as_str() {
            "checked" => "brul::BindingTarget::Checked",
            "selected" => "brul::BindingTarget::Selected",
            "disabled" => "brul::BindingTarget::Disabled",
            "visible" => "brul::BindingTarget::Visible",
            "x" => {
                x = Some(value);
                continue;
            }
            "y" => {
                y = Some(value);
                continue;
            }
            _ => {
                return Err(error_at(
                    value.position,
                    format!("parameter '{parameter}' is not supported yet"),
                ));
            }
        };
        let _ = writeln!(
            out,
            "{pad}bindings.push(brul::Binding::new(component, {name}.id(), {target}({})));",
            expression(value)
        );
    }
    if x.is_some() || y.is_some() {
        let x = optional_expression(x);
        let y = optional_expression(y);
        let _ = writeln!(
            out,
            "{pad}bindings.push(brul::Binding::new(component, {name}.id(), brul::BindingTarget::Offset {{ x: {x}, y: {y} }}));"
        );
    }
    Ok(())
}

fn optional_expression(value: Option<&Expr>) -> String {
    match value {
        Some(item) => format!("Some({})", expression(item)),
        None => "None".to_owned(),
    }
}
