use std::time::Duration;

use brul_style::{
    ColorValue, Kind, LayoutField, Property, Side, Snapshot, SnapshotError, State, StyleField,
    Stylesheet, Target, TextField, Type, Value,
};

use crate::widget::{
    Class, Layers, StyleSet, StyleState, TextStylePatch, TextStyleSet, Transitions, WidgetStyle,
    WidgetStylePatch,
};
use crate::{
    Align, Color, Dimension, Direction, Display, Gradient, Justify, LayoutPatch, LayoutStyle,
    Padding, Paint, TextAlign, TextStyle, VerticalAlign,
};

pub struct Resolved {
    pub layout: LayoutStyle,
    pub styles: StyleSet,
    pub text: TextStyleSet,
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum StyleError {
    #[error(transparent)]
    Snapshot(#[from] SnapshotError),
    #[error("expected {} for '{}'", .expected, .kind.name())]
    Value { kind: Kind, expected: &'static str },
}

pub fn check_theme(stylesheet: &Stylesheet, theme: Option<&str>) -> Result<(), StyleError> {
    match theme {
        Some(name) if !stylesheet.has_theme(name) => {
            Err(SnapshotError::UnknownTheme(name.to_owned()).into())
        }
        _ => Ok(()),
    }
}

pub fn styled(
    mut widget: crate::Widget,
    stylesheet: &Stylesheet,
    classes: &[&str],
    theme: Option<&str>,
) -> Result<crate::Widget, StyleError> {
    for name in classes {
        widget = widget.with_class(&class(stylesheet, name, theme)?);
    }
    Ok(widget)
}

pub fn resolve(
    stylesheet: &Stylesheet,
    classes: &[&str],
    theme: Option<&str>,
) -> Result<Resolved, StyleError> {
    let base = snapshot(stylesheet, classes, None, theme)?;
    let (layout, base_style, base_text) = base_styles(&base)?;
    let mut styles = StyleSet::new(base_style).with_transitions(transitions(&base)?);
    let mut text = TextStyleSet::new(base_text.clone());
    for &state in StyleState::ALL {
        let stated = stylesheet.stated(classes, syntax_state(state), theme)?;
        let (patch, text_patch) = state_patch(&stated)?;
        if patch != WidgetStylePatch::default() {
            styles = styles.with(state, patch);
        }
        if text_patch != TextStylePatch::default() {
            text = text.with(state, text_patch);
        }
    }
    Ok(Resolved {
        layout,
        styles,
        text,
    })
}

pub fn class(
    stylesheet: &Stylesheet,
    name: &str,
    theme: Option<&str>,
) -> Result<Class, StyleError> {
    let base = snapshot(stylesheet, &[name], None, theme)?;
    let (layout, _, _) = base_styles(&base)?;
    let (style_patch, text_patch) = state_patch(&base)?;
    let mut style = Layers::new(style_patch);
    let mut text = Layers::new(text_patch);
    for &state in StyleState::ALL {
        let stated = stylesheet.stated(&[name], syntax_state(state), theme)?;
        let (style_patch, text_patch) = state_patch(&stated)?;
        if style_patch != WidgetStylePatch::default() {
            style = style.with(state, style_patch);
        }
        if text_patch != TextStylePatch::default() {
            text = text.with(state, text_patch);
        }
    }
    Ok(Class {
        layout: LayoutPatch::from(&layout),
        style,
        text,
        transitions: transitions(&base)?,
    })
}

fn snapshot(
    stylesheet: &Stylesheet,
    classes: &[&str],
    state: Option<State>,
    theme: Option<&str>,
) -> Result<Snapshot, StyleError> {
    Ok(stylesheet.snapshot(classes, state, theme)?)
}

fn base_styles(snapshot: &Snapshot) -> Result<(LayoutStyle, WidgetStyle, TextStyle), StyleError> {
    let mut layout = LayoutStyle::default();
    let mut style = WidgetStyle::default();
    let mut text = TextStyle::default();
    for property in snapshot.properties() {
        apply(property, &mut layout, &mut style, &mut text)?;
    }
    Ok((layout, style, text))
}

fn state_patch(snapshot: &Snapshot) -> Result<(WidgetStylePatch, TextStylePatch), StyleError> {
    let mut style = WidgetStylePatch::default();
    let mut text = TextStylePatch::default();
    for property in snapshot.properties() {
        let kind = property.kind;
        let value = &property.value;
        match kind.target() {
            Target::Style(StyleField::Background) => {
                style.background = Some(read::<PaintTy>(value, kind)?)
            }
            Target::Style(StyleField::Border) => style.border = Some(read::<ColorTy>(value, kind)?),
            Target::Style(StyleField::Text) => style.text = Some(read::<ColorTy>(value, kind)?),
            Target::Style(StyleField::Tint) => style.tint = Some(read::<ColorTy>(value, kind)?),
            Target::Style(StyleField::Opacity) => {
                style.opacity = Some(read::<NumberTy>(value, kind)? as f32)
            }
            Target::Style(StyleField::Clip) => style.clip = Some(read::<FlagTy>(value, kind)?),
            Target::Style(StyleField::ShadowColor) => {
                style.shadow = Some(read::<ColorTy>(value, kind)?)
            }
            Target::Style(StyleField::BorderRadius) => {
                style.border_radius = Some(read::<PxTy>(value, kind)?)
            }
            Target::Style(StyleField::Scroll) => style.scroll = Some(read::<FlagTy>(value, kind)?),
            Target::Style(StyleField::ShadowX) => {
                style.shadow_x = Some(read::<SignedPxTy>(value, kind)?)
            }
            Target::Style(StyleField::ShadowY) => {
                style.shadow_y = Some(read::<SignedPxTy>(value, kind)?)
            }
            Target::Style(StyleField::ShadowBlur) => {
                style.shadow_blur = Some(read::<PxTy>(value, kind)?)
            }
            Target::Text(TextField::VerticalAlign) => {
                style.vertical_align = Some(vertical_align(read::<KeywordTy>(value, kind)?));
            }
            Target::Text(TextField::Align) => {
                text.align = Some(text_align(read::<KeywordTy>(value, kind)?));
            }
            Target::Text(TextField::Weight) => {
                text.weight = Some(read::<NumberTy>(value, kind)? as u16)
            }
            Target::Text(TextField::Size) => text.size = Some(read::<PxTy>(value, kind)? as f32),
            Target::Text(TextField::Family) => {
                text.family = Some(read::<TextTy>(value, kind)?.to_owned())
            }
            _ => {}
        }
    }
    Ok((style, text))
}

fn transitions(snapshot: &Snapshot) -> Result<Transitions, StyleError> {
    let mut transitions = Transitions::default();
    for animation in snapshot.animations() {
        let field = match animation.property.target() {
            Target::Style(StyleField::Background) => &mut transitions.background,
            Target::Style(StyleField::Border) => &mut transitions.border,
            Target::Style(StyleField::Text) => &mut transitions.text,
            Target::Style(StyleField::Tint) => &mut transitions.tint,
            Target::Style(StyleField::Opacity) => &mut transitions.opacity,
            Target::Style(StyleField::ShadowColor) => &mut transitions.shadow,
            _ => {
                return Err(StyleError::Value {
                    kind: animation.property,
                    expected: "an animatable property",
                });
            }
        };
        *field = Some(Duration::from_millis(animation.duration.round() as u64));
    }
    Ok(transitions)
}

trait Read<'a> {
    type Output;

    fn matches(ty: Type) -> bool;
    fn expected() -> &'static str;
    fn convert(value: &'a Value, kind: Kind) -> Result<Self::Output, StyleError>;
}

macro_rules! reads {
    (owned $name:ident, $out:ty, $ty:pat, $expected:literal, $convert:ident) => {
        struct $name;

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
        struct $name;

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
reads!(owned ColorTy, Color, Type::Color, "a color", paint);
reads!(owned PaintTy, Paint, Type::Paint, "a color or a gradient", background);
reads!(borrowed TextTy, Type::String, "a string", string);
reads!(borrowed KeywordTy, Type::Keywords(_), "a keyword", keyword);
reads!(owned FlagTy, bool, Type::Bool, "true or false", flag);

fn read<'a, T: Read<'a>>(value: &'a Value, kind: Kind) -> Result<T::Output, StyleError> {
    let ty = kind.spec().ty;
    if matches!(ty, Type::ThemeKey) {
        return Err(expected(kind, "a theme key"));
    }
    if !T::matches(ty) {
        return Err(expected(kind, T::expected()));
    }
    T::convert(value, kind)
}

fn apply(
    property: &Property,
    layout: &mut LayoutStyle,
    style: &mut WidgetStyle,
    text: &mut TextStyle,
) -> Result<(), StyleError> {
    let kind = property.kind;
    let value = &property.value;
    match kind.target() {
        Target::Layout(LayoutField::Width) => layout.width = read::<LengthTy>(value, kind)?,
        Target::Layout(LayoutField::Height) => layout.height = read::<LengthTy>(value, kind)?,
        Target::Layout(LayoutField::FlexGrow) => {
            layout.flex_grow = read::<NumberTy>(value, kind)? as f32
        }
        Target::Layout(LayoutField::FlexShrink) => {
            layout.flex_shrink = read::<NumberTy>(value, kind)? as f32
        }
        Target::Layout(LayoutField::FlexBasis) => {
            layout.flex_basis = read::<LengthTy>(value, kind)?
        }
        Target::Layout(LayoutField::Display) => {
            layout.display = read::<KeywordTy>(value, kind).map(|_| Display::Flex)?
        }
        Target::Layout(LayoutField::FlexDirection) => {
            layout.flex_direction = flex_direction(read::<KeywordTy>(value, kind)?)
        }
        Target::Layout(LayoutField::Padding(side)) => {
            *edge(&mut layout.padding, side) = read::<PxTy>(value, kind)?;
        }
        Target::Layout(LayoutField::Gap) => layout.gap = read::<PxTy>(value, kind)?,
        Target::Layout(LayoutField::BorderWidth) => {
            layout.border_width = read::<PxTy>(value, kind)?
        }
        Target::Layout(LayoutField::Justify) => {
            layout.justify = justify(read::<KeywordTy>(value, kind)?)
        }
        Target::Layout(
            LayoutField::GridTemplateColumns
            | LayoutField::GridTemplateRows
            | LayoutField::GridColumn
            | LayoutField::GridRow
            | LayoutField::GridAutoFlow,
        ) => return Err(expected(kind, "a supported layout property")),
        Target::Layout(LayoutField::Align) => layout.align = align(read::<KeywordTy>(value, kind)?),
        Target::Style(StyleField::Background) => style.background = read::<PaintTy>(value, kind)?,
        Target::Style(StyleField::Border) => style.border = read::<ColorTy>(value, kind)?,
        Target::Style(StyleField::Text) => style.text = read::<ColorTy>(value, kind)?,
        Target::Style(StyleField::Tint) => style.tint = Some(read::<ColorTy>(value, kind)?),
        Target::Style(StyleField::BorderRadius) => style.border_radius = read::<PxTy>(value, kind)?,
        Target::Style(StyleField::Opacity) => style.opacity = read::<NumberTy>(value, kind)? as f32,
        Target::Style(StyleField::Clip) => style.clip = read::<FlagTy>(value, kind)?,
        Target::Style(StyleField::Scroll) => style.scroll = read::<FlagTy>(value, kind)?,
        Target::Style(StyleField::ShadowX) => style.shadow.x = read::<SignedPxTy>(value, kind)?,
        Target::Style(StyleField::ShadowY) => style.shadow.y = read::<SignedPxTy>(value, kind)?,
        Target::Style(StyleField::ShadowBlur) => style.shadow.blur = read::<PxTy>(value, kind)?,
        Target::Style(StyleField::ShadowColor) => {
            style.shadow.color = read::<ColorTy>(value, kind)?
        }
        Target::Text(TextField::Size) => {
            text.font_size = read::<PxTy>(value, kind)? as f32;
            text.line_height = text.font_size * 1.2;
        }
        Target::Text(TextField::Family) => {
            text.family = Some(read::<TextTy>(value, kind)?.to_owned())
        }
        Target::Text(TextField::Align) => text.align = text_align(read::<KeywordTy>(value, kind)?),
        Target::Text(TextField::VerticalAlign) => {
            style.vertical_align = vertical_align(read::<KeywordTy>(value, kind)?)
        }
        Target::Text(TextField::Weight) => text.weight = read::<NumberTy>(value, kind)? as u16,
    }
    Ok(())
}

fn edge(padding: &mut Padding, side: Side) -> &mut u32 {
    match side {
        Side::Left => &mut padding.left,
        Side::Top => &mut padding.top,
        Side::Right => &mut padding.right,
        Side::Bottom => &mut padding.bottom,
    }
}

fn flex_direction(value: &str) -> Direction {
    match value {
        "column" => Direction::Column,
        _ => Direction::Row,
    }
}

fn justify(value: &str) -> Justify {
    match value {
        "center" => Justify::Center,
        "end" => Justify::End,
        "space-between" => Justify::SpaceBetween,
        _ => Justify::Start,
    }
}

fn align(value: &str) -> Align {
    match value {
        "center" => Align::Center,
        "end" => Align::End,
        "stretch" => Align::Stretch,
        _ => Align::Start,
    }
}

fn text_align(value: &str) -> TextAlign {
    match value {
        "center" => TextAlign::Center,
        "right" => TextAlign::Right,
        _ => TextAlign::Left,
    }
}

fn vertical_align(value: &str) -> VerticalAlign {
    match value {
        "center" => VerticalAlign::Center,
        "bottom" => VerticalAlign::Bottom,
        _ => VerticalAlign::Top,
    }
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

macro_rules! syntax_states {
    ($(($syntax:ident, $gui:ident, $field:ident, $keyword:literal)),+ $(,)?) => {
        fn syntax_state(state: StyleState) -> State {
            match state {
                $(StyleState::$gui => State::$syntax,)+
            }
        }
    };
}

brul_style::states!(syntax_states);
