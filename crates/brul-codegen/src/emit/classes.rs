use std::fmt::Write;

use brul_syntax::ast::Position;
use brul_syntax::style::Stylesheet;

use crate::emit::class;
use crate::error_at;

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
    out: &mut String,
    prefix: &str,
    stylesheet: &Stylesheet,
    names: &ClassNames,
) -> Result<(), brul_syntax::error::Error> {
    for theme in themes(stylesheet) {
        let mut body = String::new();
        for name in &names.names {
            let class = brul_gui::style::class(stylesheet, name, theme).map_err(|error| {
                error_at(
                    Position { line: 1, column: 1 },
                    format!("class '{name}': {error}"),
                )
            })?;
            body.push_str(&indent(&class::literal(&class), "    "));
            body.push_str(",\n");
        }
        let _ = writeln!(
            out,
            "\n#[allow(dead_code)]\nstatic {}: [brul::gui::Class; {}] = [\n{body}];",
            constant(prefix, theme),
            names.names.len()
        );
    }
    let (parameter, body) = match stylesheet.themes.first() {
        Some(first) => {
            let mut arms = vec![format!(
                "None | Some({:?}) => &{}",
                first.name,
                constant(prefix, Some(&first.name))
            )];
            for theme in &stylesheet.themes[1..] {
                arms.push(format!(
                    "Some({:?}) => &{}",
                    theme.name,
                    constant(prefix, Some(&theme.name))
                ));
            }
            arms.push(format!(
                "Some(_) => &{}",
                constant(prefix, Some(&first.name))
            ));
            (
                "theme",
                format!(
                    "match theme {{\n        {},\n    }}",
                    arms.join(",\n        ")
                ),
            )
        }
        None => ("_theme", format!("&{}", constant(prefix, None))),
    };
    let _ = writeln!(
        out,
        "\nfn {prefix}classes({parameter}: Option<&str>) -> &'static [brul::gui::Class] {{\n    {body}\n}}"
    );
    Ok(())
}

fn indent(text: &str, prefix: &str) -> String {
    text.lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
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

pub(super) fn tables(out: &mut String, prefix: &str, stylesheet: &Stylesheet) {
    let mut themes: Vec<Option<&str>> = stylesheet
        .themes
        .iter()
        .map(|theme| Some(theme.name.as_str()))
        .collect();
    if themes.is_empty() {
        themes.push(None);
    }
    for theme in themes {
        let _ = writeln!(
            out,
            "\n#[allow(dead_code)]\nfn {}(name: &str) -> Option<brul::gui::Class> {{\n    brul::gui::style::class({prefix}styles(), name, {:?}).ok()\n}}",
            name(prefix, theme),
            theme
        );
    }
    let (parameter, body) = match stylesheet.themes.first() {
        Some(first) => {
            let first_name = name(prefix, Some(&first.name));
            let mut arms = format!("None | Some({:?}) => {first_name}", first.name);
            for theme in &stylesheet.themes[1..] {
                let _ = write!(
                    arms,
                    ", Some({:?}) => {}",
                    theme.name,
                    name(prefix, Some(&theme.name))
                );
            }
            let _ = write!(arms, ", Some(_) => {first_name}");
            ("theme", format!("match theme {{ {arms} }}"))
        }
        None => ("_theme", name(prefix, None)),
    };
    let _ = writeln!(
        out,
        "\nfn {prefix}class_table({parameter}: Option<&str>) -> brul::ClassTable {{\n    {body}\n}}"
    );
}
