use std::fmt::Write;

use brul_gui::{
    Align, Color, Dimension, Justify, LayoutStyle, StyleSet, TextAlign, TextStyle, WidgetState,
    WidgetStyle,
};
use brul_syntax::ast::ExprKind;
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, BlockItem, Element};
use brul_syntax::style::Stylesheet;

use crate::error_at;
use crate::styles;

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

fn layout(layout: LayoutStyle) -> Option<String> {
    let default = LayoutStyle::default();
    let mut fields = Vec::new();
    if layout.width != default.width {
        fields.push(format!("width: {}", dimension(layout.width)));
    }
    if layout.height != default.height {
        fields.push(format!("height: {}", dimension(layout.height)));
    }
    if layout.padding != default.padding {
        fields.push(format!(
            "padding: brul_gui::Padding {{ left: {}, top: {}, right: {}, bottom: {} }}",
            layout.padding.left, layout.padding.top, layout.padding.right, layout.padding.bottom
        ));
    }
    if layout.border_width != default.border_width {
        fields.push(format!("border_width: {}", layout.border_width));
    }
    if layout.spacing != default.spacing {
        fields.push(format!("spacing: {}", layout.spacing));
    }
    if layout.stretch != default.stretch {
        fields.push(format!("stretch: {:?}f32", layout.stretch));
    }
    if layout.justify != default.justify {
        fields.push(format!("justify: {}", justify(layout.justify)));
    }
    if layout.align != default.align {
        fields.push(format!("align: {}", align(layout.align)));
    }
    (!fields.is_empty()).then(|| format!("brul_gui::LayoutStyle {{ {} }}", fields.join(", ")))
}

fn style_set(set: StyleSet) -> Option<String> {
    let base = set.base();
    let mut states = Vec::new();
    for (name, state) in [
        (
            "Hovered",
            WidgetState {
                hovered: true,
                ..WidgetState::default()
            },
        ),
        (
            "Pressed",
            WidgetState {
                pressed: true,
                ..WidgetState::default()
            },
        ),
        (
            "Checked",
            WidgetState {
                checked: true,
                ..WidgetState::default()
            },
        ),
        (
            "Selected",
            WidgetState {
                selected: true,
                ..WidgetState::default()
            },
        ),
        (
            "Disabled",
            WidgetState {
                disabled: true,
                ..WidgetState::default()
            },
        ),
    ] {
        let style = set.resolve(state);
        if style != base {
            states.push(format!(
                ".with(brul_gui::StyleState::{name}, {})",
                widget_style(style)
            ));
        }
    }
    if base == WidgetStyle::default() && states.is_empty() {
        return None;
    }
    let mut out = format!("brul_gui::StyleSet::new({})", widget_style(base));
    for state in states {
        out.push_str(&state);
    }
    Some(out)
}

fn widget_style(style: WidgetStyle) -> String {
    let default = WidgetStyle::default();
    let mut fields = Vec::new();
    if style.background != default.background {
        fields.push(format!("background: {}", color(style.background)));
    }
    if style.border != default.border {
        fields.push(format!("border: {}", color(style.border)));
    }
    if style.text != default.text {
        fields.push(format!("text: {}", color(style.text)));
    }
    if style.radius != default.radius {
        fields.push(format!("radius: {}", style.radius));
    }
    if style.tint != default.tint {
        fields.push(format!(
            "tint: Some({})",
            color(style.tint.unwrap_or(Color::TRANSPARENT))
        ));
    }
    if style.opacity != default.opacity {
        fields.push(format!("opacity: {:?}f32", style.opacity));
    }
    if fields.is_empty() {
        "brul_gui::WidgetStyle::default()".to_owned()
    } else {
        format!("brul_gui::WidgetStyle {{ {} }}", fields.join(", "))
    }
}

fn text_style(style: TextStyle) -> Option<String> {
    let default = TextStyle::default();
    let mut fields = Vec::new();
    if style.family != default.family {
        let family = style.family.as_deref().unwrap_or_default();
        fields.push(format!("family: Some({family:?}.to_owned())"));
    }
    if style.font_size != default.font_size {
        fields.push(format!("font_size: {:?}f32", style.font_size));
    }
    if style.line_height != default.line_height {
        fields.push(format!("line_height: {:?}f32", style.line_height));
    }
    if style.weight != default.weight {
        fields.push(format!("weight: {}", style.weight));
    }
    if style.align != default.align {
        fields.push(format!("align: {}", text_align(style.align)));
    }
    (!fields.is_empty()).then(|| format!("brul_gui::TextStyle {{ {} }}", fields.join(", ")))
}

fn dimension(value: Dimension) -> String {
    match value {
        Dimension::Auto => "brul_gui::Dimension::Auto".to_owned(),
        Dimension::Px(value) => format!("brul_gui::Dimension::Px({value})"),
        Dimension::Percent(value) => format!("brul_gui::Dimension::Percent({value:?}f32)"),
    }
}

fn justify(value: Justify) -> String {
    match value {
        Justify::Start => "brul_gui::Justify::Start".to_owned(),
        Justify::Center => "brul_gui::Justify::Center".to_owned(),
        Justify::End => "brul_gui::Justify::End".to_owned(),
        Justify::SpaceBetween => "brul_gui::Justify::SpaceBetween".to_owned(),
    }
}

fn align(value: Align) -> String {
    match value {
        Align::Start => "brul_gui::Align::Start".to_owned(),
        Align::Center => "brul_gui::Align::Center".to_owned(),
        Align::End => "brul_gui::Align::End".to_owned(),
        Align::Stretch => "brul_gui::Align::Stretch".to_owned(),
    }
}

fn text_align(value: TextAlign) -> String {
    match value {
        TextAlign::Left => "brul_gui::TextAlign::Left".to_owned(),
        TextAlign::Center => "brul_gui::TextAlign::Center".to_owned(),
        TextAlign::Right => "brul_gui::TextAlign::Right".to_owned(),
    }
}

fn color(value: Color) -> String {
    format!(
        "brul_gui::Color::rgba({}, {}, {}, {})",
        value.r, value.g, value.b, value.a
    )
}
