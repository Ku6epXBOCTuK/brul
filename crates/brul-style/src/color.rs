use material_colors::{blend, color::Rgb, hct::Hct};

use crate::ast::{Color, ColorExpr, ColorValue, Theme, Value};
use crate::snapshot::SnapshotError;

pub(crate) fn resolve(value: &ColorValue, theme: Option<&Theme>) -> Result<Color, SnapshotError> {
    walk(value, theme, &mut Vec::new())
}

pub(crate) fn resolve_expr(
    expr: &ColorExpr,
    theme: Option<&Theme>,
) -> Result<Color, SnapshotError> {
    expr_color(expr, theme, &mut Vec::new())
}

fn walk(
    value: &ColorValue,
    theme: Option<&Theme>,
    stack: &mut Vec<String>,
) -> Result<Color, SnapshotError> {
    match value {
        ColorValue::Color(color) => Ok(*color),
        ColorValue::Theme(key) => theme_color(key, theme, stack),
        ColorValue::Expr(expr) => expr_color(expr, theme, stack),
    }
}

fn expr_color(
    expr: &ColorExpr,
    theme: Option<&Theme>,
    stack: &mut Vec<String>,
) -> Result<Color, SnapshotError> {
    match expr {
        ColorExpr::Alpha { color, amount } => Ok(alpha(walk(color, theme, stack)?, *amount)),
        ColorExpr::Mix { from, to, amount } => Ok(mix(
            walk(from, theme, stack)?,
            walk(to, theme, stack)?,
            *amount,
        )),
        ColorExpr::Lighten { color, amount } => Ok(tone(walk(color, theme, stack)?, *amount)),
        ColorExpr::Darken { color, amount } => Ok(tone(walk(color, theme, stack)?, -*amount)),
    }
}

fn theme_color(
    key: &str,
    theme: Option<&Theme>,
    stack: &mut Vec<String>,
) -> Result<Color, SnapshotError> {
    if stack.iter().any(|item| item == key) {
        let mut path = stack.clone();
        path.push(key.to_owned());
        return Err(SnapshotError::ThemeCycle(path.join(" -> ")));
    }
    let value = theme
        .and_then(|theme| theme.properties.iter().find(|entry| entry.key == key))
        .map(|entry| &entry.value)
        .ok_or_else(|| SnapshotError::UnknownThemeKey(key.to_owned()))?;
    match value {
        Value::Color(color) => Ok(*color),
        Value::ColorExpr(expr) => {
            stack.push(key.to_owned());
            let color = expr_color(expr, theme, stack);
            stack.pop();
            color
        }
        _ => Err(SnapshotError::NotAColor(key.to_owned())),
    }
}

fn alpha(color: Color, amount: f64) -> Color {
    Color {
        a: alpha_channel(amount),
        ..color
    }
}

fn mix(from: Color, to: Color, amount: f64) -> Color {
    let share = amount / 100.0;
    let to_weight = share * f64::from(to.a);
    let from_weight = (1.0 - share) * f64::from(from.a);
    let total = to_weight + from_weight;
    let color_share = if total > 0.0 {
        to_weight / total
    } else {
        share
    };
    let mixed = if color_share <= 0.0 {
        from
    } else if color_share >= 1.0 {
        to
    } else {
        let mixed = blend::cam16_ucs(rgb(from), rgb(to), color_share);
        Color {
            r: mixed.red,
            g: mixed.green,
            b: mixed.blue,
            a: 0,
        }
    };
    Color {
        a: (f64::from(from.a) * (1.0 - share) + f64::from(to.a) * share).round() as u8,
        ..mixed
    }
}

fn tone(color: Color, shift: f64) -> Color {
    let mut hct = Hct::new(rgb(color));
    hct.set_tone((hct.get_tone() + shift).clamp(0.0, 100.0));
    let mapped = Rgb::from(hct);
    Color {
        r: mapped.red,
        g: mapped.green,
        b: mapped.blue,
        a: color.a,
    }
}

fn alpha_channel(amount: f64) -> u8 {
    (amount / 100.0 * 255.0).round().clamp(0.0, 255.0) as u8
}

fn rgb(color: Color) -> Rgb {
    Rgb::new(color.r, color.g, color.b)
}
