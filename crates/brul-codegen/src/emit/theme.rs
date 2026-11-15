use std::collections::HashMap;
use std::fmt::Write;

use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::style::Stylesheet;

use crate::error_at;

pub(super) fn id(out: &mut String, stylesheet: &Stylesheet) -> Result<(), Error> {
    let mut variants = Vec::new();
    let mut seen: HashMap<String, &str> = HashMap::new();
    for theme in &stylesheet.themes {
        let name = variant(&theme.name);
        if let Some(other) = seen.insert(name.clone(), &theme.name) {
            return Err(error_at(
                Position { line: 1, column: 1 },
                format!(
                    "themes '{other}' and '{}' share the variant ThemeId::{name}",
                    theme.name
                ),
            ));
        }
        variants.push((name, theme.name.as_str()));
    }
    let mut declarations = String::new();
    let mut list = String::new();
    let mut arms = String::new();
    for (name, theme) in &variants {
        let _ = writeln!(declarations, "    {name},");
        let _ = writeln!(list, "        ThemeId::{name},");
        let _ = writeln!(arms, "            Self::{name} => {theme:?},");
    }
    let names = if variants.is_empty() {
        "        match self {}\n".to_owned()
    } else {
        format!("        match self {{\n{arms}        }}\n")
    };
    let _ = write!(
        out,
        "\n#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeId {{
{declarations}}}

impl ThemeId {{
    pub const ALL: &'static [ThemeId] = &[
{list}    ];
}}

impl brul::Theme for ThemeId {{
    fn name(self) -> &'static str {{
{names}    }}

    fn parse(name: &str) -> Option<Self> {{
        Self::ALL
            .iter()
            .copied()
            .find(|theme| brul::Theme::name(*theme) == name)
    }}
}}

impl std::str::FromStr for ThemeId {{
    type Err = brul::UnknownTheme;

    fn from_str(name: &str) -> Result<Self, Self::Err> {{
        <Self as brul::Theme>::parse(name).ok_or_else(|| brul::UnknownTheme::new(name))
    }}
}}
"
    );
    Ok(())
}

fn variant(name: &str) -> String {
    let mut variant = String::new();
    for part in name.split(|letter: char| !letter.is_ascii_alphanumeric()) {
        let mut letters = part.chars();
        if let Some(first) = letters.next() {
            variant.push(first.to_ascii_uppercase());
            variant.extend(letters);
        }
    }
    variant
}

#[cfg(test)]
mod tests {
    use super::variant;

    #[test]
    fn a_theme_name_becomes_a_variant() {
        assert_eq!(variant("dark"), "Dark");
        assert_eq!(variant("high-contrast"), "HighContrast");
    }
}
