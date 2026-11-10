use brul_gui::{
    Align, Color, Dimension, Justify, LayoutStyle, Padding, StyleSet, StyleState, TextStyle,
    WidgetStyle,
};
use brul_syntax::style::{BorderColor, Kind, Property, Side, Snapshot, State, Stylesheet, Value};

pub struct Resolved {
    pub layout: LayoutStyle,
    pub styles: StyleSet,
    pub text: TextStyle,
}

pub fn resolve(
    stylesheet: &Stylesheet,
    classes: &[&str],
    theme: Option<&str>,
) -> Result<Resolved, String> {
    let base = snapshot(stylesheet, classes, None, theme)?;
    let (layout, base_style, text) = base_styles(&base)?;
    let mut styles = StyleSet::new(base_style);
    for state in [
        State::Hover,
        State::Pressed,
        State::Checked,
        State::Selected,
        State::Disabled,
    ] {
        let state_snapshot = snapshot(stylesheet, classes, Some(state), theme)?;
        if state_snapshot == base {
            continue;
        }
        let style = state_style(&state_snapshot, base_style)?;
        styles = styles.with(style_state(state), style);
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

fn state_style(snapshot: &Snapshot, value: WidgetStyle) -> Result<WidgetStyle, String> {
    let mut layout = LayoutStyle::default();
    let mut style = value;
    let mut text = TextStyle::default();
    for property in snapshot.properties() {
        apply(property, &mut layout, &mut style, &mut text)?;
    }
    Ok(style)
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
        Kind::Padding => layout.padding = Padding::all(px_value(value, kind)?),
        Kind::PaddingSide(side) => {
            *edge(&mut layout.padding, side) = px_value(value, kind)?;
        }
        Kind::Spacing => layout.spacing = px_value(value, kind)?,
        Kind::BorderWidth => layout.border_width = px_value(value, kind)?,
        Kind::Border => {
            let border = border(value, kind)?;
            layout.border_width = px(border.width);
            style.border = border_color(&border.color);
        }
        Kind::JustifyContent => layout.justify = justify(keyword(value, kind)?),
        Kind::AlignItems => layout.align = align(keyword(value, kind)?),
        Kind::Background => style.background = background(value)?,
        Kind::BorderColor => style.border = paint(value, kind)?,
        Kind::Color => style.text = paint(value, kind)?,
        Kind::Tint => style.tint = Some(paint(value, kind)?),
        Kind::Radius => style.radius = px_value(value, kind)?,
        Kind::Opacity => style.opacity = number(value, kind)? as f32,
        Kind::FontSize => {
            text.font_size = px_value(value, kind)? as f32;
            text.line_height = text.font_size * 1.2;
        }
        Kind::FontFamily => text.family = Some(string(value, kind)?.to_owned()),
        Kind::TextAlign => text.align = text_align(keyword(value, kind)?),
        Kind::VerticalAlign => {}
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

fn background(value: &Value) -> Result<Color, String> {
    match value {
        Value::Color(value) => Ok(color(*value)),
        Value::Gradient(_) => Err("gradient is not supported yet".to_owned()),
        _ => Err("expected a color or a gradient for 'background'".to_owned()),
    }
}

fn border(value: &Value, kind: Kind) -> Result<&brul_syntax::style::Border, String> {
    match value {
        Value::Border(value) => Ok(value),
        _ => Err(expected(kind, "a border")),
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

fn border_color(value: &BorderColor) -> Color {
    match value {
        BorderColor::Color(value) => color(*value),
        BorderColor::ThemeRef(_) => Color::TRANSPARENT,
    }
}

fn color(value: brul_syntax::style::Color) -> Color {
    Color::rgba(value.r, value.g, value.b, value.a)
}

fn px(value: f64) -> u32 {
    value.max(0.0).round() as u32
}

fn style_state(state: State) -> StyleState {
    match state {
        State::Hover => StyleState::Hovered,
        State::Pressed => StyleState::Pressed,
        State::Checked => StyleState::Checked,
        State::Selected => StyleState::Selected,
        State::Disabled => StyleState::Disabled,
    }
}
