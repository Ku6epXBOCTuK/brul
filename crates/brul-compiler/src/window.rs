use brul_gui::Widget;
use brul_gui::window::WindowConfig;
use brul_script::{Component, execute_program};
use brul_syntax::ast::{Expr, ExprKind, Position};
use brul_syntax::error::Error;
use brul_syntax::markup::{self, Arg, Declaration, Element, Member, Window};
use brul_syntax::script::{self, Program};
use brul_syntax::sfc;
use brul_syntax::style::{self, Stylesheet};
use brul_syntax::uses::{self, Use};

use crate::build_bound_tree;
use crate::tree::{Binding, script_error};

pub struct Loaded {
    pub config: WindowConfig,
    pub tree: Widget,
    pub bindings: Vec<Binding>,
    pub component: Component,
    pub uses: Vec<Use>,
    pub program: Option<Program>,
}

pub fn load(source: &str, theme: Option<&str>) -> Result<Loaded, Error> {
    let sfc = sfc::parse(source).map_err(sfc_error)?;
    let markup = markup::parse_file(&padded(source, sfc.markup))?;
    let window = markup
        .declarations
        .into_iter()
        .find_map(|declaration| match declaration {
            Declaration::Window(window) => Some(window),
            Declaration::Component(_) => None,
        })
        .ok_or_else(|| {
            error_at(
                Position { line: 1, column: 1 },
                "missing window declaration",
            )
        })?;

    let stylesheet = match sfc.style {
        Some(block) => style::parse_file(&padded(source, block))?,
        None => Stylesheet {
            themes: Vec::new(),
            rules: Vec::new(),
        },
    };
    let uses = match sfc.uses {
        Some(block) => uses::parse(&padded(source, block))?,
        None => Vec::new(),
    };
    let program = match sfc.script {
        Some(block) => Some(script::parse(&padded(source, block))?),
        None => None,
    };

    let mut component = Component::new();
    if let Some(program) = &program {
        execute_program(&mut component, program).map_err(script_error)?;
    }

    let root = root(&window)?;
    let config = config(&window)?;
    let built = build_bound_tree(root, &stylesheet, theme, &mut component)?;

    Ok(Loaded {
        config,
        tree: built.widget,
        bindings: built.bindings,
        component,
        uses,
        program,
    })
}

fn config(window: &Window) -> Result<WindowConfig, Error> {
    let mut config = WindowConfig::default();
    for arg in &window.args {
        match arg {
            Arg::Named { name, value } => match name.as_str() {
                "title" => config.title = string(value, "title")?,
                "width" => config.width = size(value, "width")?,
                "height" => config.height = size(value, "height")?,
                "frameless" => flag(value, "frameless")?,
                "transparent" => flag(value, "transparent")?,
                _ => {
                    return Err(error_at(
                        value.position,
                        format!("unknown window argument '{name}'"),
                    ));
                }
            },
            Arg::Handler { .. } => {
                return Err(error_at(
                    window.position,
                    "window events are not supported yet",
                ));
            }
            Arg::Class(_) => {
                return Err(error_at(window.position, "window arguments must be named"));
            }
        }
    }
    Ok(config)
}

fn root(window: &Window) -> Result<&Element, Error> {
    let mut roots = window.members.iter().filter_map(|member| match member {
        Member::Element(element) => Some(element),
        Member::Prop(_) | Member::Event(_) => None,
    });
    let root = roots
        .next()
        .ok_or_else(|| error_at(window.position, "window has no root element"))?;
    if roots.next().is_some() {
        return Err(error_at(
            window.position,
            "window has more than one root element",
        ));
    }
    Ok(root)
}

fn string(value: &Expr, name: &str) -> Result<String, Error> {
    match &value.kind {
        ExprKind::String(text) => Ok(text.clone()),
        _ => Err(error_at(
            value.position,
            format!("window argument '{name}' must be a string"),
        )),
    }
}

fn size(value: &Expr, name: &str) -> Result<u32, Error> {
    match &value.kind {
        ExprKind::Number(number)
            if number.is_finite()
                && *number > 0.0
                && number.fract() == 0.0
                && *number <= f64::from(u32::MAX) =>
        {
            Ok(*number as u32)
        }
        _ => Err(error_at(
            value.position,
            format!("window argument '{name}' must be a positive integer"),
        )),
    }
}

fn flag(value: &Expr, name: &str) -> Result<(), Error> {
    match &value.kind {
        ExprKind::Bool(true) => Ok(()),
        ExprKind::Bool(false) => Err(error_at(value.position, format!("window is always {name}"))),
        _ => Err(error_at(
            value.position,
            format!("window argument '{name}' must be a bool"),
        )),
    }
}

fn padded(source: &str, block: sfc::Block<'_>) -> String {
    let mut text = String::with_capacity(block.offset + block.content.len());
    for character in source[..block.offset].chars() {
        text.push(if character == '\n' { '\n' } else { ' ' });
    }
    text.push_str(block.content);
    text
}

fn sfc_error(error: sfc::Error) -> Error {
    match error {
        sfc::Error::Syntax {
            message,
            line,
            column,
        } => Error {
            message,
            line,
            column,
        },
        sfc::Error::Duplicate { name, line, column } => Error {
            message: format!("duplicate <{name}> block"),
            line,
            column,
        },
        sfc::Error::MissingMarkup => {
            error_at(Position { line: 1, column: 1 }, "missing <markup> block")
        }
    }
}

fn error_at(position: Position, message: impl Into<String>) -> Error {
    Error {
        message: message.into(),
        line: position.line,
        column: position.column,
    }
}
