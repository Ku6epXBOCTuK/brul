use brul_gui::{
    Align, Color, Dimension, Justify, LayoutStyle, Padding, Paint, StyleSet, StyleState, TextStyle,
    TextStyleSet, VerticalAlign, WidgetStyle,
};
use brul_syntax::style::{Kind, Property, Side, Snapshot, State, Stylesheet, Value};

pub struct Resolved {
    pub layout: LayoutStyle,
    pub styles: StyleSet,
    pub text: TextStyleSet,
}

pub fn resolve(
    stylesheet: &Stylesheet,
    classes: &[&str],
    theme: Option<&str>,
) -> Result<Resolved, String> {
    let base = snapshot(stylesheet, classes, None, theme)?;
    let (layout, base_style, base_text) = base_styles(&base)?;
    let mut styles = StyleSet::new(base_style);
    let mut text = TextStyleSet::new(base_text.clone());
    for state in StyleState::ALL {
        let state_snapshot = snapshot(stylesheet, classes, Some(syntax_state(state)), theme)?;
        if state_snapshot == base {
            continue;
        }
        let (style, state_text) = state_styles(&state_snapshot, base_style, &base_text)?;
        if style != base_style {
            styles = styles.with(state, style);
        }
        if state_text != base_text {
            text = text.with(state, state_text);
        }
    }
    Ok(Resolved {
        layout,
        styles,
        text,
    })
}

fn snapshot(
    stylesheet: &Stylesheet,
    classes: &[&str],
    state: Option<State>,
    theme: Option<&str>,
) -> Result<Snapshot, String> {
    stylesheet
        .snapshot(classes, state, theme)
        .map_err(|error| error.to_string())
}

fn base_styles(snapshot: &Snapshot) -> Result<(LayoutStyle, WidgetStyle, TextStyle), String> {
    let mut layout = LayoutStyle::default();
    let mut style = WidgetStyle::default();
    let mut text = TextStyle::default();
    for property in snapshot.properties() {
        apply(property, &mut layout, &mut style, &mut text)?;
    }
    Ok((layout, style, text))
}

fn state_styles(
    snapshot: &Snapshot,
    style: WidgetStyle,
    text: &TextStyle,
) -> Result<(WidgetStyle, TextStyle), String> {
    let mut layout = LayoutStyle::default();
    let mut style = style;
    let mut text = text.clone();
    for property in snapshot.properties() {
        apply(property, &mut layout, &mut style, &mut text)?;
    }
    Ok((style, text))
}

fn apply(
    property: &Property,
    layout: &mut LayoutStyle,
    style: &mut WidgetStyle,
    text: &mut TextStyle,
) -> Result<(), String> {
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

fn text_align(value: &str) -> brul_gui::TextAlign {
    match value {
        "center" => brul_gui::TextAlign::Center,
        "right" => brul_gui::TextAlign::Right,
        _ => brul_gui::TextAlign::Left,
    }
}

fn vertical_align(value: &str) -> VerticalAlign {
    match value {
        "center" => VerticalAlign::Center,
        "bottom" => VerticalAlign::Bottom,
        _ => VerticalAlign::Top,
    }
}

fn background(value: &Value) -> Result<Paint, String> {
    match value {
        Value::Color(value) => Ok(Paint::Color(color(*value))),
        Value::Gradient(value) => Ok(Paint::Gradient(brul_gui::Gradient::Linear {
            angle: value.angle as f32,
            from: color(value.from),
            to: color(value.to),
        })),
        _ => Err("expected a color or a gradient for 'background'".to_owned()),
    }
}

fn dimension(value: &Value, kind: Kind) -> Result<Dimension, String> {
    match value {
        Value::Px(value) => Ok(Dimension::Px(px(*value))),
        Value::Percent(value) => Ok(Dimension::Percent(*value as f32)),
        _ => Err(expected(kind, "px or %")),
    }
}

fn number(value: &Value, kind: Kind) -> Result<f64, String> {
    match value {
        Value::Number(value) => Ok(*value),
        _ => Err(expected(kind, "a number")),
    }
}

fn px_value(value: &Value, kind: Kind) -> Result<u32, String> {
    match value {
        Value::Px(value) => Ok(px(*value)),
        _ => Err(expected(kind, "px")),
    }
}

fn signed_px(value: &Value, kind: Kind) -> Result<i32, String> {
    match value {
        Value::Px(value) => Ok(value.round() as i32),
        _ => Err(expected(kind, "px")),
    }
}

fn flag(value: &Value, kind: Kind) -> Result<bool, String> {
    match value {
        Value::Bool(value) => Ok(*value),
        _ => Err(expected(kind, "true or false")),
    }
}

fn keyword(value: &Value, kind: Kind) -> Result<&str, String> {
    match value {
        Value::Keyword(value) => Ok(value),
        _ => Err(expected(kind, "a keyword")),
    }
}

fn string(value: &Value, kind: Kind) -> Result<&str, String> {
    match value {
        Value::String(value) => Ok(value),
        _ => Err(expected(kind, "a string")),
    }
}

fn paint(value: &Value, kind: Kind) -> Result<Color, String> {
    match value {
        Value::Color(value) => Ok(color(*value)),
        _ => Err(expected(kind, "a color")),
    }
}

fn expected(kind: Kind, ty: &str) -> String {
    format!("expected {ty} for '{}'", kind.name())
}

fn color(value: brul_syntax::style::Color) -> Color {
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
