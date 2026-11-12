use std::fmt::Write;

use brul_syntax::style::Stylesheet;

pub(super) fn name(prefix: &str, theme: Option<&str>) -> String {
    match theme {
        Some(theme) => format!(
            "{prefix}class_{}",
            theme.replace(|letter: char| !letter.is_ascii_alphanumeric(), "_")
        ),
        None => format!("{prefix}class_none"),
    }
}

pub(super) fn table(prefix: &str, stylesheet: &Stylesheet) -> String {
    let Some(first) = stylesheet.themes.first() else {
        return name(prefix, None);
    };
    let first_name = name(prefix, Some(&first.name));
    let mut out = format!(
        "match theme {{ None | Some({:?}) => {first_name}",
        first.name
    );
    for theme in &stylesheet.themes[1..] {
        let _ = write!(
            out,
            ", Some({:?}) => {}",
            theme.name,
            name(prefix, Some(&theme.name))
        );
    }
    let _ = write!(out, ", Some(_) => {first_name} }}");
    out
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
}
