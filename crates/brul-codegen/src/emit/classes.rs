use brul_syntax::ast::Position;
use brul_syntax::style::Stylesheet;

use crate::emit::class;
use crate::error_at;

use super::Out;

#[derive(Default)]
pub(super) struct ClassNames {
    names: Vec<String>,
}

pub(super) fn index(names: &mut ClassNames, name: &str) -> usize {
    match names.names.iter().position(|item| item == name) {
        Some(index) => index,
        None => {
            names.names.push(name.to_owned());
            names.names.len() - 1
        }
    }
}

pub(super) fn statics(
    out: &mut Out<'_>,
    prefix: &str,
    stylesheet: &Stylesheet,
    names: &ClassNames,
) -> Result<(), brul_syntax::error::Error> {
    for theme in themes(stylesheet) {
        out.end();
        out.line(format_args!("#[allow(dead_code)]"));
        out.line(format_args!(
            "static {}: [brul::gui::Class; {}] = [",
            constant(prefix, theme),
            names.names.len()
        ));
        out.indented(|out| -> Result<(), brul_syntax::error::Error> {
            for name in &names.names {
                let class = brul_gui::style::class(stylesheet, name, theme).map_err(|error| {
                    error_at(
                        Position { line: 1, column: 1 },
                        format!("class '{name}': {error}"),
                    )
                })?;
                class::literal(out, &class);
                out.push(format_args!(","));
                out.end();
            }
            Ok(())
        })?;
        out.line(format_args!("];"));
    }
    out.end();
    out.line(format_args!(
        "fn {prefix}classes({}: Option<&str>) -> &'static [brul::gui::Class] {{",
        parameter(stylesheet)
    ));
    out.indented(|out| match stylesheet.themes.first() {
        Some(first) => {
            out.line(format_args!("match theme {{"));
            out.indented(|out| {
                out.line(format_args!(
                    "None | Some({:?}) => &{},",
                    first.name,
                    constant(prefix, Some(&first.name))
                ));
                for theme in &stylesheet.themes[1..] {
                    out.line(format_args!(
                        "Some({:?}) => &{},",
                        theme.name,
                        constant(prefix, Some(&theme.name))
                    ));
                }
                out.line(format_args!(
                    "Some(_) => &{},",
                    constant(prefix, Some(&first.name))
                ));
            });
            out.line(format_args!("}}"));
        }
        None => out.line(format_args!("&{}", constant(prefix, None))),
    });
    out.line(format_args!("}}"));
    Ok(())
}

fn parameter(stylesheet: &Stylesheet) -> &'static str {
    if stylesheet.themes.is_empty() {
        "_theme"
    } else {
        "theme"
    }
}

fn themes(stylesheet: &Stylesheet) -> Vec<Option<&str>> {
    let mut themes: Vec<Option<&str>> = stylesheet
        .themes
        .iter()
        .map(|theme| Some(theme.name.as_str()))
        .collect();
    if themes.is_empty() {
        themes.push(None);
    }
    themes
}

fn constant(prefix: &str, theme: Option<&str>) -> String {
    let suffix = match theme {
        Some(theme) => theme
            .replace(|letter: char| !letter.is_ascii_alphanumeric(), "_")
            .to_uppercase(),
        None => "NONE".to_owned(),
    };
    format!("{}CLASSES_{suffix}", prefix.to_uppercase())
}

pub(super) fn name(prefix: &str, theme: Option<&str>) -> String {
    match theme {
        Some(theme) => format!(
            "{prefix}class_{}",
            theme.replace(|letter: char| !letter.is_ascii_alphanumeric(), "_")
        ),
        None => format!("{prefix}class_none"),
    }
}

pub(super) fn table(prefix: &str) -> String {
    format!("{prefix}class_table(theme)")
}

pub(super) fn tables(out: &mut Out<'_>, prefix: &str, stylesheet: &Stylesheet) {
    for theme in themes(stylesheet) {
        out.end();
        out.line(format_args!("#[allow(dead_code)]"));
        out.line(format_args!(
            "fn {}(name: &str) -> Option<brul::gui::Class> {{",
            name(prefix, theme)
        ));
        out.indented(|out| {
            out.line(format_args!(
                "brul::gui::style::class({prefix}styles(), name, {:?}).ok()",
                theme
            ));
        });
        out.line(format_args!("}}"));
    }
    let (parameter, body) = match stylesheet.themes.first() {
        Some(first) => {
            let first_name = name(prefix, Some(&first.name));
            let mut arms = format!("None | Some({:?}) => {first_name}", first.name);
            for theme in &stylesheet.themes[1..] {
                arms.push_str(&format!(
                    ", Some({:?}) => {}",
                    theme.name,
                    name(prefix, Some(&theme.name))
                ));
            }
            arms.push_str(&format!(", Some(_) => {first_name}"));
            ("theme", format!("match theme {{ {arms} }}"))
        }
        None => ("_theme", name(prefix, None)),
    };
    out.end();
    out.line(format_args!(
        "fn {prefix}class_table({parameter}: Option<&str>) -> brul::ClassTable {{"
    ));
    out.indented(|out| out.line(format_args!("{body}")));
    out.line(format_args!("}}"));
}
