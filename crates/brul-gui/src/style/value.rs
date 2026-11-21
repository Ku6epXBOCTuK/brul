use brul_style::{AreaTemplate, ColorValue, GridLine, Kind, TrackSize, Type, Value};

use super::StyleError;
use crate::{Color, Dimension, Gradient, Paint};

pub(super) trait Read<'a> {
    type Output;

    fn matches(ty: Type) -> bool;
    fn expected() -> &'static str;
    fn convert(value: &'a Value, kind: Kind) -> Result<Self::Output, StyleError>;
}

macro_rules! reads {
    (owned $name:ident, $out:ty, $ty:pat, $expected:literal, $convert:ident) => {
        pub(super) struct $name;

        impl<'a> Read<'a> for $name {
            type Output = $out;

            fn matches(ty: Type) -> bool {
                matches!(ty, $ty)
            }

            fn expected() -> &'static str {
                $expected
            }

            fn convert(value: &'a Value, kind: Kind) -> Result<Self::Output, StyleError> {
                $convert(value, kind)
            }
        }
    };
    (borrowed $name:ident, $ty:pat, $expected:literal, $convert:ident) => {
        pub(super) struct $name;

        impl<'a> Read<'a> for $name {
            type Output = &'a str;

            fn matches(ty: Type) -> bool {
                matches!(ty, $ty)
            }

            fn expected() -> &'static str {
                $expected
            }

            fn convert(value: &'a Value, kind: Kind) -> Result<Self::Output, StyleError> {
                $convert(value, kind)
            }
        }
    };
}

reads!(owned PxTy, u32, Type::Px, "px", px_value);
reads!(owned SignedPxTy, i32, Type::SignedPx, "px", signed_px);
reads!(
    owned NumberTy,
    f64,
    (Type::Number | Type::NumberRange(..) | Type::NumberIn(..)),
    "a number",
    number
);
reads!(owned LengthTy, Dimension, Type::Length, "px or %", dimension);
reads!(owned TrackListTy, Vec<TrackSize>, Type::TrackList, "a track list", track_list);
reads!(owned GridLineTy, GridLine, Type::GridLine, "a grid line", grid_line);
reads!(owned AreaTemplateTy, AreaTemplate, Type::AreaTemplate, "an area template", area_template);
reads!(owned GridAreaTy, String, Type::GridArea, "a grid area", grid_area);
reads!(owned ColorTy, Color, Type::Color, "a color", paint);
reads!(owned PaintTy, Paint, Type::Paint, "a color or a gradient", background);
reads!(borrowed TextTy, Type::String, "a string", string);
reads!(borrowed KeywordTy, Type::Keywords(_), "a keyword", keyword);
reads!(owned FlagTy, bool, Type::Bool, "true or false", flag);

pub(super) fn read<'a, T: Read<'a>>(value: &'a Value, kind: Kind) -> Result<T::Output, StyleError> {
    let ty = kind.spec().ty;
    if matches!(ty, Type::ThemeKey) {
        return Err(expected(kind, "a theme key"));
    }
    if !T::matches(ty) {
        return Err(expected(kind, T::expected()));
    }
    T::convert(value, kind)
}

fn background(value: &Value, _kind: Kind) -> Result<Paint, StyleError> {
    match value {
        Value::Color(value) => Ok(Paint::Color(color(*value))),
        Value::Gradient(value) => Ok(Paint::Gradient(Gradient::Linear {
            angle: value.angle as f32,
            from: gradient_color(&value.from),
            to: gradient_color(&value.to),
        })),
        _ => Err(StyleError::Value {
            kind: Kind::Background,
            expected: "a color or a gradient",
        }),
    }
}

fn dimension(value: &Value, kind: Kind) -> Result<Dimension, StyleError> {
    match value {
        Value::Px(value) => Ok(Dimension::Px(px(*value))),
        Value::Percent(value) => Ok(Dimension::Percent(*value as f32)),
        _ => Err(expected(kind, "px or %")),
    }
}

fn track_list(value: &Value, kind: Kind) -> Result<Vec<TrackSize>, StyleError> {
    match value {
        Value::TrackList(tracks) => Ok(tracks.clone()),
        _ => Err(expected(kind, "a track list")),
    }
}

fn grid_line(value: &Value, kind: Kind) -> Result<GridLine, StyleError> {
    match value {
        Value::GridLine(line) => Ok(*line),
        _ => Err(expected(kind, "a grid line")),
    }
}

fn area_template(value: &Value, kind: Kind) -> Result<AreaTemplate, StyleError> {
    match value {
        Value::AreaTemplate(template) => Ok(template.clone()),
        _ => Err(expected(kind, "an area template")),
    }
}

fn grid_area(value: &Value, kind: Kind) -> Result<String, StyleError> {
    match value {
        Value::GridArea(area) => Ok(area.clone()),
        _ => Err(expected(kind, "a grid area")),
    }
}

fn number(value: &Value, kind: Kind) -> Result<f64, StyleError> {
    match value {
        Value::Number(value) => Ok(*value),
        _ => Err(expected(kind, "a number")),
    }
}

fn px_value(value: &Value, kind: Kind) -> Result<u32, StyleError> {
    match value {
        Value::Px(value) => Ok(px(*value)),
        _ => Err(expected(kind, "px")),
    }
}

fn signed_px(value: &Value, kind: Kind) -> Result<i32, StyleError> {
    match value {
        Value::Px(value) => Ok(value.round() as i32),
        _ => Err(expected(kind, "px")),
    }
}

fn flag(value: &Value, kind: Kind) -> Result<bool, StyleError> {
    match value {
        Value::Bool(value) => Ok(*value),
        _ => Err(expected(kind, "true or false")),
    }
}

fn keyword(value: &Value, kind: Kind) -> Result<&str, StyleError> {
    match value {
        Value::Keyword(value) => Ok(value),
        _ => Err(expected(kind, "a keyword")),
    }
}

fn string(value: &Value, kind: Kind) -> Result<&str, StyleError> {
    match value {
        Value::String(value) => Ok(value),
        _ => Err(expected(kind, "a string")),
    }
}

fn paint(value: &Value, kind: Kind) -> Result<Color, StyleError> {
    match value {
        Value::Color(value) => Ok(color(*value)),
        _ => Err(expected(kind, "a color")),
    }
}

fn expected(kind: Kind, expected: &'static str) -> StyleError {
    StyleError::Value { kind, expected }
}

fn gradient_color(value: &ColorValue) -> Color {
    match value {
        ColorValue::Color(stop) => color(*stop),
        ColorValue::Theme(_) | ColorValue::Expr(_) => {
            unreachable!("a snapshot resolves gradient stops")
        }
    }
}

fn color(value: brul_style::Color) -> Color {
    Color::rgba(value.r, value.g, value.b, value.a)
}

fn px(value: f64) -> u32 {
    value.max(0.0).round() as u32
}
