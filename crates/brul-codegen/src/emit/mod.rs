use std::fmt::Write;
use std::fs;
use std::path::Path;

use brul_gui::TextStyle;
use brul_syntax::ast::{Expr, ExprKind, Position};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Block, BlockItem, Element, Member};
use brul_syntax::script::Program;
use brul_syntax::style::Stylesheet;

use crate::runtime::HandlerEvent;
use crate::{Definition, Import, Registry, error_at, styles};

mod events;
mod expression;
mod script;
mod structure;
mod style;

pub use expression::expression;
use style::{layout, style_set, text_style};

pub(super) struct Emit<'a> {
    registry: &'a Registry,
    imports: &'a [Import],
    stylesheet: &'a Stylesheet,
    program: Option<&'a Program>,
    functions: &'a mut Vec<String>,
    next_structure: &'a mut usize,
}

pub fn module(source: &str) -> Result<String, Error> {
    module_at(source, None)
}

pub fn module_file(path: &Path) -> Result<String, Error> {
    let position = Position { line: 1, column: 1 };
    let source = fs::read_to_string(path).map_err(|error| {
        error_at(
            position,
            format!("cannot read '{}': {error}", path.display()),
        )
    })?;
    module_at(&source, path.parent())
}

fn module_at(source: &str, base: Option<&Path>) -> Result<String, Error> {
    let parsed = crate::parse_source(source)?;
    let registry = crate::resolve(&parsed.uses, base)?;
    let (config, window_handlers) = crate::arguments(&parsed.window)?;
    let window_handlers = events::window(&window_handlers, parsed.program.as_ref())?;
    let root = crate::root(&parsed.window)?;
    let mut functions = Vec::new();
    let mut next_structure = 0;
    let mut out = String::new();
    out.push_str("#[allow(unused_mut, unused_variables, clippy::let_and_return)]\n");
    view(
        &mut out,
        "view",
        root,
        &window_handlers,
        &mut Emit {
            registry: &registry,
            imports: &registry.imports,
            stylesheet: &parsed.stylesheet,
            program: parsed.program.as_ref(),
            functions: &mut functions,
            next_structure: &mut next_structure,
        },
    )?;
    for index in 0..registry.definitions.len() {
        let definition = &registry.definitions[index];
        let root = definition_root(definition)?;
        out.push('\n');
        out.push_str("#[allow(dead_code, unused_mut, unused_variables, clippy::let_and_return)]\n");
        view(
            &mut out,
            &format!("component{index}_view"),
            root,
            &[],
            &mut Emit {
                registry: &registry,
                imports: &definition.imports,
                stylesheet: &definition.stylesheet,
                program: definition.program.as_ref(),
                functions: &mut functions,
                next_structure: &mut next_structure,
            },
        )?;
        out.push_str(&script::program(
            &format!("component{index}_script"),
            definition.program.as_ref(),
        ));
    }
    for function in &functions {
        out.push('\n');
        out.push_str(function);
    }
    out.push('\n');
    window_config(&mut out, &config);
    out.push_str(&script::program("script", parsed.program.as_ref()));
    Ok(out)
}

fn definition_root(definition: &Definition) -> Result<&Element, Error> {
    let position = Position { line: 1, column: 1 };
    let mut roots = definition
        .component
        .members
        .iter()
        .filter_map(|member| match member {
            Member::Element(element) => Some(element),
            Member::Prop(_) | Member::Event(_) => None,
        });
    let Some(root) = roots.next() else {
        return Err(error_at(
            position,
            format!("component '{}' has no root element", definition.name),
        ));
    };
    if roots.next().is_some() {
        return Err(error_at(
            position,
            format!(
                "component '{}' has more than one root element",
                definition.name
            ),
        ));
    }
    Ok(root)
}

fn view(
    out: &mut String,
    name: &str,
    root: &Element,
    handlers: &[(HandlerEvent, String)],
    emit: &mut Emit<'_>,
) -> Result<(), Error> {
    let _ = writeln!(
        out,
        "pub fn {name}(theme: Option<&str>, component: &brul::ComponentHandle) -> Result<brul::Built, String> {{"
    );
    out.push_str("    let mut bindings = Vec::new();\n");
    out.push_str("    let mut handlers = Vec::new();\n");
    out.push_str("    let mut structures = Vec::new();\n");
    out.push_str("    let mut props = Vec::new();\n");
    out.push_str("    let widget = match theme {\n");
    branch(out, "None", root, emit, None)?;
    let themes: Vec<String> = emit
        .stylesheet
        .themes
        .iter()
        .map(|theme| theme.name.clone())
        .collect();
    for theme in themes {
        let pattern = format!("Some({theme:?})");
        branch(out, &pattern, root, emit, Some(&theme))?;
    }
    out.push_str("        Some(theme) => return Err(format!(\"unknown theme '{theme}'\")),\n");
    out.push_str("    };\n");
    for (event, handler) in handlers {
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
    Ok(())
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
    if let Some(index) = emit
        .imports
        .iter()
        .position(|import| import.name == element.name)
    {
        return component(out, element, index, emit, theme, depth, counter);
    }
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
fn component(
    out: &mut String,
    element: &Element,
    index: usize,
    emit: &mut Emit<'_>,
    theme: Option<&str>,
    depth: usize,
    counter: &mut usize,
) -> Result<(), Error> {
    let name = &emit.registry.definitions[index].name;
    if element.args.iter().any(|arg| matches!(arg, Arg::Class(_))) {
        return Err(error_at(
            element.position,
            format!("component '{name}' does not accept classes yet"),
        ));
    }
    if !element.block.items.is_empty() {
        return Err(error_at(
            element.position,
            format!("component '{name}' does not accept children"),
        ));
    }
    for arg in &element.args {
        match arg {
            Arg::Class(_) => {}
            Arg::Named { value, .. } => {
                return Err(error_at(
                    value.position,
                    "component properties are not generated yet",
                ));
            }
            Arg::Handler { .. } => {
                return Err(error_at(
                    element.position,
                    "component events are not generated yet",
                ));
            }
        }
    }
    let widget = format!("widget{}", *counter);
    *counter += 1;
    let pad = "    ".repeat(depth);
    let inner = "    ".repeat(depth + 1);
    let theme = match theme {
        Some(theme) => format!("Some({theme:?})"),
        None => "None".to_owned(),
    };
    let _ = writeln!(out, "{pad}{{");
    let _ = writeln!(
        out,
        "{inner}let built = brul::instance({theme}, component{index}_view, component{index}_script().as_ref())?;"
    );
    for field in ["bindings", "handlers", "structures", "props"] {
        let _ = writeln!(out, "{inner}{field}.extend(built.{field});");
    }
    let _ = writeln!(out, "{inner}let {widget} = built.widget;");
    let _ = writeln!(out, "{inner}{widget}");
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
                structure::structure_each(
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
