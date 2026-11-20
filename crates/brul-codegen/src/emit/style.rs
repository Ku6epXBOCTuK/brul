use brul_syntax::style::{ClassRule, Color, ColorExpr, ColorValue, Kind, Stylesheet, Theme, Value};

use super::Out;

pub(super) fn stylesheet(out: &mut Out<'_>, prefix: &str, sheet: &Stylesheet) {
    out.end();
    out.line(format_args!("#[allow(dead_code)]"));
    out.line(format_args!(
        "fn {prefix}styles() -> &'static brul::style::Stylesheet {{"
    ));
    out.indented(|out| {
        out.line(format_args!("use brul::style as s;"));
        out.end();
        out.line(format_args!(
            "static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {{"
        ));
        out.indented(|out| {
            out.start(format_args!("themes: "));
            list(out, sheet.themes.is_empty(), |out| {
                for theme in &sheet.themes {
                    theme_line(out, theme);
                }
            });
            out.push(format_args!(","));
            out.end();
            out.start(format_args!("rules: "));
            list(out, sheet.rules.is_empty(), |out| {
                for rule in &sheet.rules {
                    rule_block(out, rule);
                }
            });
            out.push(format_args!(","));
            out.end();
        });
        out.line(format_args!("}});"));
        out.line(format_args!("&STYLES"));
    });
    out.line(format_args!("}}"));
    out.end();
    out.line(format_args!("fn {prefix}styled("));
    out.indented(|out| {
        out.line(format_args!("mut widget: brul::gui::Widget,"));
        out.line(format_args!("indices: &[usize],"));
        out.line(format_args!("theme: Option<&str>,"));
    });
    out.line(format_args!(") -> brul::gui::Widget {{"));
    out.indented(|out| {
        out.line(format_args!("let table = {prefix}classes(theme);"));
        out.line(format_args!("for index in indices {{"));
        out.indented(|out| {
            out.line(format_args!("widget = widget.with_class(&table[*index]);"));
        });
        out.line(format_args!("}}"));
        out.line(format_args!("widget"));
    });
    out.line(format_args!("}}"));
}

fn theme_line(out: &mut Out<'_>, theme: &Theme) {
    out.start(format_args!(
        "s::Theme {{ name: {:?}.into(), properties: ",
        theme.name
    ));
    list(out, theme.properties.is_empty(), |out| {
        for entry in &theme.properties {
            out.line(format_args!(
                "s::ThemeEntry {{ key: {:?}.into(), value: {} }},",
                entry.key,
                value(&entry.value)
            ));
        }
    });
    out.push(format_args!(" }},"));
    out.end();
}

fn rule_block(out: &mut Out<'_>, rule: &ClassRule) {
    let state = match rule.state {
        Some(state) => format!("Some(s::State::{state:?})"),
        None => "None".to_owned(),
    };
    out.line(format_args!("s::ClassRule {{"));
    out.indented(|out| {
        out.line(format_args!("class: {:?}.into(),", rule.class));
        out.line(format_args!("state: {state},"));
        out.start(format_args!("properties: "));
        list(out, rule.properties.is_empty(), |out| {
            for property in &rule.properties {
                out.line(format_args!(
                    "s::Property {{ kind: {}, value: {} }},",
                    kind(property.kind),
                    value(&property.value)
                ));
            }
        });
        out.push(format_args!(","));
        out.end();
        out.start(format_args!("animations: "));
        list(out, rule.animations.is_empty(), |out| {
            for animation in &rule.animations {
                out.line(format_args!(
                    "s::Animation {{ property: {}, duration: {:?} }},",
                    kind(animation.property),
                    animation.duration
                ));
            }
        });
        out.push(format_args!(","));
        out.end();
    });
    out.line(format_args!("}},"));
}

fn list(out: &mut Out<'_>, empty: bool, body: impl FnOnce(&mut Out<'_>)) {
    if empty {
        out.push(format_args!("vec![]"));
        return;
    }
    out.push(format_args!("vec!["));
    out.end();
    out.indented(body);
    out.start(format_args!("]"));
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
        Value::Color(color) => format!("s::Value::Color({})", literal(*color)),
        Value::ThemeRef(key) => format!("s::Value::ThemeRef({key:?}.into())"),
        Value::ColorExpr(expr) => format!("s::Value::ColorExpr({})", color_expr(expr)),
        Value::Gradient(gradient) => format!(
            "s::Value::Gradient(s::Gradient {{ angle: {:?}, from: {}, to: {} }})",
            gradient.angle,
            operand(&gradient.from),
            operand(&gradient.to)
        ),
        Value::Bool(flag) => format!("s::Value::Bool({flag})"),
        value => unreachable!("grid values are not implemented yet: {value:?}"),
    }
}

fn operand(value: &ColorValue) -> String {
    match value {
        ColorValue::Color(color) => format!("s::ColorValue::Color({})", literal(*color)),
        ColorValue::Theme(key) => format!("s::ColorValue::Theme({key:?}.into())"),
        ColorValue::Expr(expr) => format!("s::ColorValue::Expr(Box::new({}))", color_expr(expr)),
    }
}

fn color_expr(expr: &ColorExpr) -> String {
    match expr {
        ColorExpr::Alpha { color, amount } => format!(
            "s::ColorExpr::Alpha {{ color: {}, amount: {amount:?} }}",
            operand(color)
        ),
        ColorExpr::Mix { from, to, amount } => format!(
            "s::ColorExpr::Mix {{ from: {}, to: {}, amount: {amount:?} }}",
            operand(from),
            operand(to)
        ),
        ColorExpr::Lighten { color, amount } => format!(
            "s::ColorExpr::Lighten {{ color: {}, amount: {amount:?} }}",
            operand(color)
        ),
        ColorExpr::Darken { color, amount } => format!(
            "s::ColorExpr::Darken {{ color: {}, amount: {amount:?} }}",
            operand(color)
        ),
    }
}

fn literal(value: Color) -> String {
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
