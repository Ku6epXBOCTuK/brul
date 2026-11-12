use std::fmt;
use std::time::Duration;

use brul_style::{
    GradientColor, Kind, Property, Side, Snapshot, SnapshotError, State, Stylesheet, Value,
};

use crate::widget::{
    Class, Layers, StyleSet, StyleState, TextStylePatch, TextStyleSet, Transitions, WidgetStyle,
    WidgetStylePatch,
};
use crate::{
    Align, Color, Dimension, Gradient, Justify, LayoutPatch, LayoutStyle, Padding, Paint,
    TextAlign, TextStyle, VerticalAlign,
};

pub struct Resolved {
    pub layout: LayoutStyle,
    pub styles: StyleSet,
    pub text: TextStyleSet,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StyleError {
    Snapshot(SnapshotError),
    Value { kind: Kind, expected: &'static str },
}

impl fmt::Display for StyleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StyleError::Snapshot(error) => error.fmt(f),
            StyleError::Value { kind, expected } => {
                write!(f, "expected {expected} for '{}'", kind.name())
            }
        }
    }
}

impl std::error::Error for StyleError {}

impl From<SnapshotError> for StyleError {
    fn from(error: SnapshotError) -> Self {
        Self::Snapshot(error)
    }
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
    widget: crate::Widget,
    stylesheet: &Stylesheet,
    classes: &[&str],
    theme: Option<&str>,
) -> Result<crate::Widget, StyleError> {
    let resolved = resolve(stylesheet, classes, theme)?;
    Ok(widget
        .with_layout(resolved.layout)
        .with_styles(resolved.styles)
        .with_text_styles(resolved.text))
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
    for state in StyleState::ALL {
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
    for state in StyleState::ALL {
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
        layout: layout_patch(&layout),
        style,
        text,
        transitions: transitions(&base)?,
    })
}

fn layout_patch(layout: &LayoutStyle) -> LayoutPatch {
    let default = LayoutStyle::default();
    LayoutPatch {
        width: (layout.width != default.width).then_some(layout.width),
        height: (layout.height != default.height).then_some(layout.height),
        padding: (layout.padding != default.padding).then_some(layout.padding),
        border_width: (layout.border_width != default.border_width).then_some(layout.border_width),
        spacing: (layout.spacing != default.spacing).then_some(layout.spacing),
        stretch: (layout.stretch != default.stretch).then_some(layout.stretch),
        justify: (layout.justify != default.justify).then_some(layout.justify),
        align: (layout.align != default.align).then_some(layout.align),
    }
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
        let value = &property.value;
        let kind = property.kind;
        match kind {
            Kind::Background => style.background = Some(background(value)?),
            Kind::BorderColor => style.border = Some(paint(value, kind)?),
            Kind::Color => style.text = Some(paint(value, kind)?),
            Kind::Tint => style.tint = Some(paint(value, kind)?),
            Kind::Opacity => style.opacity = Some(number(value, kind)? as f32),
            Kind::Clip => style.clip = Some(flag(value, kind)?),
            Kind::ShadowColor => style.shadow = Some(paint(value, kind)?),
            Kind::VerticalAlign => {
                style.vertical_align = Some(vertical_align(keyword(value, kind)?))
            }
            Kind::TextAlign => text.align = Some(text_align(keyword(value, kind)?)),
            Kind::FontWeight => text.weight = Some(number(value, kind)? as u16),
            _ => {}
        }
    }
    Ok((style, text))
}

fn transitions(snapshot: &Snapshot) -> Result<Transitions, StyleError> {
    let mut transitions = Transitions::default();
    for animation in snapshot.animations() {
        let field = match animation.property {
            Kind::Background => &mut transitions.background,
            Kind::BorderColor => &mut transitions.border,
            Kind::Color => &mut transitions.text,
            Kind::Tint => &mut transitions.tint,
            Kind::Opacity => &mut transitions.opacity,
            Kind::ShadowColor => &mut transitions.shadow,
            kind => {
                return Err(StyleError::Value {
                    kind,
                    expected: "an animatable property",
                });
            }
        };
        *field = Some(Duration::from_millis(animation.duration.round() as u64));
    }
    Ok(transitions)
}

fn apply(
    property: &Property,
    layout: &mut LayoutStyle,
    style: &mut WidgetStyle,
    text: &mut TextStyle,
) -> Result<(), StyleError> {
    let value = &property.value;
    let kind = property.kind;
    match kind {
        Kind::Width => layout.width = dimension(value, kind)?,
        Kind::Height => layout.height = dimension(value, kind)?,
        Kind::Stretch => layout.stretch = number(value, kind)? as f32,
        Kind::PaddingSide(side) => {
            *edge(&mut layout.padding, side) = px_value(value, kind)?;
        }
        Kind::Spacing => layout.spacing = px_value(value, kind)?,
        Kind::BorderWidth => layout.border_width = px_value(value, kind)?,
        Kind::JustifyContent => layout.justify = justify(keyword(value, kind)?),
        Kind::AlignItems => layout.align = align(keyword(value, kind)?),
        Kind::Background => style.background = background(value)?,
        Kind::BorderColor => style.border = paint(value, kind)?,
        Kind::Color => style.text = paint(value, kind)?,
        Kind::Tint => style.tint = Some(paint(value, kind)?),
        Kind::Radius => style.radius = px_value(value, kind)?,
        Kind::Opacity => style.opacity = number(value, kind)? as f32,
        Kind::Clip => style.clip = flag(value, kind)?,
        Kind::Scroll => style.scroll = flag(value, kind)?,
        Kind::ShadowOffsetX => style.shadow.x = signed_px(value, kind)?,
        Kind::ShadowOffsetY => style.shadow.y = signed_px(value, kind)?,
        Kind::ShadowBlur => style.shadow.blur = px_value(value, kind)?,
        Kind::ShadowColor => style.shadow.color = paint(value, kind)?,
        Kind::FontSize => {
            text.font_size = px_value(value, kind)? as f32;
            text.line_height = text.font_size * 1.2;
        }
        Kind::FontFamily => text.family = Some(string(value, kind)?.to_owned()),
        Kind::TextAlign => text.align = text_align(keyword(value, kind)?),
        Kind::VerticalAlign => style.vertical_align = vertical_align(keyword(value, kind)?),
        Kind::FontWeight => text.weight = number(value, kind)? as u16,
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

fn background(value: &Value) -> Result<Paint, StyleError> {
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

fn gradient_color(value: &GradientColor) -> Color {
    match value {
        GradientColor::Color(stop) => color(*stop),
        GradientColor::Theme(_) => {
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

fn syntax_state(state: StyleState) -> State {
    match state {
        StyleState::Hovered => State::Hover,
        StyleState::Pressed => State::Pressed,
        StyleState::Checked => State::Checked,
        StyleState::Selected => State::Selected,
        StyleState::Disabled => State::Disabled,
    }
}
