use std::fmt::Write;

use brul_syntax::style::{
    Animation, ClassRule, Color, GradientColor, Kind, Property, Stylesheet, Theme, ThemeEntry,
    Value,
};

pub(super) fn stylesheet(out: &mut String, prefix: &str, sheet: &Stylesheet) {
    let themes = sheet
        .themes
        .iter()
        .map(theme_line)
        .collect::<Vec<_>>()
        .join("\n");
    let entries = sheet
        .rules
        .iter()
        .map(rule_block)
        .collect::<Vec<_>>()
        .join("\n");
    let _ = write!(
        out,
        "\nfn {prefix}styles() -> &'static brul::style::Stylesheet {{
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {{
        themes: {},
        rules: {},
    }});
    &STYLES
}}

fn {prefix}styled(
    widget: brul::gui::Widget,
    classes: &[&str],
    theme: Option<&str>,
) -> Result<brul::gui::Widget, String> {{
    brul::gui::style::styled(widget, {prefix}styles(), classes, theme)
        .map_err(|error| error.to_string())
}}
",
        list(&themes, "        "),
        list(&entries, "        ")
    );
}

fn theme_line(theme: &Theme) -> String {
    let entries = theme
        .properties
        .iter()
        .map(entry_line)
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "            s::Theme {{ name: {:?}.into(), properties: {} }},",
        theme.name,
        list(&entries, "            ")
    )
}

fn entry_line(entry: &ThemeEntry) -> String {
    format!(
        "                s::ThemeEntry {{ key: {:?}.into(), value: {} }},",
        entry.key,
        value(&entry.value)
    )
}

fn rule_block(rule: &ClassRule) -> String {
    let state = match rule.state {
        Some(state) => format!("Some(s::State::{state:?})"),
        None => "None".to_owned(),
    };
    let properties = rule
        .properties
        .iter()
        .map(property_line)
        .collect::<Vec<_>>()
        .join("\n");
    let animations = rule
        .animations
        .iter()
        .map(animation_line)
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "            s::ClassRule {{
                class: {:?}.into(),
                state: {state},
                properties: {},
                animations: {},
            }},",
        rule.class,
        list(&properties, "                "),
        list(&animations, "                ")
    )
}

fn property_line(property: &Property) -> String {
    format!(
        "                    s::Property {{ kind: {}, value: {} }},",
        kind(property.kind),
        value(&property.value)
    )
}

fn animation_line(animation: &Animation) -> String {
    format!(
        "                    s::Animation {{ property: {}, duration: {:?} }},",
        kind(animation.property),
        animation.duration
    )
}

fn list(entries: &str, indent: &str) -> String {
    if entries.is_empty() {
        return "vec![]".to_owned();
    }
    format!("vec![\n{entries}\n{indent}]")
}

fn kind(value: Kind) -> String {
    match value {
        Kind::PaddingSide(side) => format!("s::Kind::PaddingSide(s::Side::{side:?})"),
        value => format!("s::Kind::{value:?}"),
    }
}

fn value(value: &Value) -> String {
    match value {
        Value::Number(number) => format!("s::Value::Number({number:?})"),
        Value::Px(px) => format!("s::Value::Px({px:?})"),
        Value::Percent(percent) => format!("s::Value::Percent({percent:?})"),
        Value::String(text) => format!("s::Value::String({text:?}.into())"),
        Value::Keyword(keyword) => format!("s::Value::Keyword({keyword:?}.into())"),
        Value::Color(color) => format!("s::Value::Color({})", color_expr(*color)),
        Value::ThemeRef(key) => format!("s::Value::ThemeRef({key:?}.into())"),
        Value::Gradient(gradient) => format!(
            "s::Value::Gradient(s::Gradient {{ angle: {:?}, from: {}, to: {} }})",
            gradient.angle,
            gradient_color(&gradient.from),
            gradient_color(&gradient.to)
        ),
        Value::Bool(flag) => format!("s::Value::Bool({flag})"),
    }
}

fn gradient_color(value: &GradientColor) -> String {
    match value {
        GradientColor::Color(color) => format!("s::GradientColor::Color({})", color_expr(*color)),
        GradientColor::Theme(key) => format!("s::GradientColor::Theme({key:?}.into())"),
    }
}

fn color_expr(value: Color) -> String {
    format!(
        "s::Color::rgba({}, {}, {}, {})",
        value.r, value.g, value.b, value.a
    )
}

#[cfg(test)]
mod tests {
    use super::kind;
    use brul_syntax::style::{Kind, Side};

    #[test]
    fn a_padding_side_names_its_side() {
        assert_eq!(
            kind(Kind::PaddingSide(Side::Left)),
            "s::Kind::PaddingSide(s::Side::Left)"
        );
    }

    #[test]
    fn a_plain_kind_names_itself() {
        assert_eq!(kind(Kind::Background), "s::Kind::Background");
    }
}
