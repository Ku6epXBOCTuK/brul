use std::fmt::Write;

use brul_gui::Color;
use brul_syntax::style::{Stylesheet, Value};

// NOTE: a snapshot resolves theme references into colours, so the key is recovered
// by value here; two keys with the same colour pick the first one, which paints the same.
pub(crate) struct Palette<'a> {
    entries: Vec<Entry<'a>>,
}

struct Entry<'a> {
    key: &'a str,
    color: Color,
}

struct Field<'a> {
    key: &'a str,
    name: String,
}

impl<'a> Palette<'a> {
    pub(super) fn new(stylesheet: &'a Stylesheet, theme: Option<&str>) -> Self {
        let theme = match theme {
            Some(name) => stylesheet.themes.iter().find(|theme| theme.name == name),
            None => stylesheet.themes.first(),
        };
        let entries = theme
            .map(|theme| {
                theme
                    .properties
                    .iter()
                    .filter_map(|entry| match entry.value {
                        Value::Color(color) => Some(Entry {
                            key: entry.key.as_str(),
                            color: Color::rgba(color.r, color.g, color.b, color.a),
                        }),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self { entries }
    }

    pub(super) fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub(super) fn lookup(&self, color: Color) -> Option<&'a str> {
        self.entries
            .iter()
            .find(|entry| entry.color == color)
            .map(|entry| entry.key)
    }
}

pub(super) fn functions(
    out: &mut String,
    structure: &str,
    function: &str,
    stylesheet: &Stylesheet,
) -> Result<(), String> {
    let mut fields: Vec<Field> = Vec::new();
    for key in keys(stylesheet) {
        let name = palette_field(key);
        if fields.iter().any(|field| field.name == name) {
            return Err(format!(
                "theme key '{key}' shares the palette field '{name}'"
            ));
        }
        fields.push(Field { key, name });
    }
    if stylesheet.themes.is_empty() {
        return Ok(());
    }
    let _ = write!(out, "\n#[allow(dead_code)]\npub struct {structure} {{\n");
    for field in &fields {
        let name = &field.name;
        let _ = writeln!(out, "    {name}: brul_gui::Color,");
    }
    out.push_str("}\n");
    let _ = writeln!(
        out,
        "fn {function}(theme: Option<&str>) -> Result<{structure}, String> {{\n    match theme {{"
    );
    for (position, theme) in stylesheet.themes.iter().enumerate() {
        let pattern = if position == 0 {
            format!("None | Some({:?})", theme.name)
        } else {
            format!("Some({:?})", theme.name)
        };
        let values: Vec<String> = fields
            .iter()
            .filter_map(|field| {
                let name = &field.name;
                theme
                    .properties
                    .iter()
                    .find(|entry| entry.key == field.key)
                    .and_then(|entry| match entry.value {
                        Value::Color(color) => Some(format!(
                            "{name}: brul_gui::Color::rgba({}, {}, {}, {})",
                            color.r, color.g, color.b, color.a
                        )),
                        _ => None,
                    })
            })
            .collect();
        let _ = writeln!(
            out,
            "        {pattern} => Ok({structure} {{ {} }}),",
            values.join(", ")
        );
    }
    out.push_str("        Some(theme) => Err(format!(\"unknown theme '{theme}'\")),\n    }\n}\n");
    Ok(())
}

fn keys(stylesheet: &Stylesheet) -> Vec<&str> {
    let Some(first) = stylesheet.themes.first() else {
        return Vec::new();
    };
    first
        .properties
        .iter()
        .filter(|entry| matches!(entry.value, Value::Color(_)))
        .filter(|entry| {
            stylesheet.themes.iter().all(|theme| {
                theme
                    .properties
                    .iter()
                    .any(|other| other.key == entry.key && matches!(other.value, Value::Color(_)))
            })
        })
        .map(|entry| entry.key.as_str())
        .collect()
}

pub(super) fn palette_field(key: &str) -> String {
    key.replace(|letter: char| !letter.is_ascii_alphanumeric(), "_")
}
