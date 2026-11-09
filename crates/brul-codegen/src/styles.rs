use brul_gui::{
    Align, Color, Dimension, Justify, LayoutStyle, Padding, StyleSet, StyleState, TextStyle,
    WidgetStyle,
};
use brul_syntax::style::{BorderColor, Property, Snapshot, State, Stylesheet, Value};

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
    let layout = layout(&base)?;
    let base_style = widget_style(&base, WidgetStyle::default())?;
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
        let style = widget_style(&state_snapshot, base_style)?;
        styles = styles.with(style_state(state), style);
    }
    Ok(Resolved {
        layout,
        styles,
        text: text_style(&base)?,
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

fn layout(snapshot: &Snapshot) -> Result<LayoutStyle, String> {
    let mut layout = LayoutStyle::default();
    for property in snapshot.properties() {
        match property.name.as_str() {
            "width" | "height" | "stretch" => size(&mut layout, property)?,
            "padding" | "padding-left" | "padding-right" | "padding-top" | "padding-bottom"
            | "spacing" => padding(&mut layout, property)?,
            "border" | "border-width" => border(&mut layout, property)?,
            _ => alignment(&mut layout, property),
        }
    }
    Ok(layout)
}

fn size(layout: &mut LayoutStyle, property: &Property) -> Result<(), String> {
    match (property.name.as_str(), &property.value) {
        ("width", value) => layout.width = dimension(value, "width")?,
        ("height", value) => layout.height = dimension(value, "height")?,
        ("stretch", value) => layout.stretch = number(value, "stretch")? as f32,
        _ => {}
    }
    Ok(())
}

fn padding(layout: &mut LayoutStyle, property: &Property) -> Result<(), String> {
    match (property.name.as_str(), &property.value) {
        ("padding", Value::Px(value)) => layout.padding = Padding::all(px(*value)),
        ("padding-left", Value::Px(value)) => layout.padding.left = px(*value),
        ("padding-right", Value::Px(value)) => layout.padding.right = px(*value),
        ("padding-top", Value::Px(value)) => layout.padding.top = px(*value),
        ("padding-bottom", Value::Px(value)) => layout.padding.bottom = px(*value),
        ("spacing", Value::Px(value)) => layout.spacing = px(*value),
        _ => {}
    }
    Ok(())
}

fn border(layout: &mut LayoutStyle, property: &Property) -> Result<(), String> {
    match (property.name.as_str(), &property.value) {
        ("border", Value::Border(border)) => layout.border_width = px(border.width),
        ("border-width", Value::Px(value)) => layout.border_width = px(*value),
        _ => {}
    }
    Ok(())
}

fn alignment(layout: &mut LayoutStyle, property: &Property) {
    if property.name == "justify-content" {
        layout.justify = justify(&property.value);
    } else {
        layout.align = align(&property.value);
    }
}

fn justify(value: &Value) -> Justify {
    let Value::Keyword(value) = value else {
        return Justify::Start;
    };
    match value.as_str() {
        "center" => Justify::Center,
        "end" => Justify::End,
        "space-between" => Justify::SpaceBetween,
        _ => Justify::Start,
    }
}

fn align(value: &Value) -> Align {
    let Value::Keyword(value) = value else {
        return Align::Start;
    };
    match value.as_str() {
        "center" => Align::Center,
        "end" => Align::End,
        "stretch" => Align::Stretch,
        _ => Align::Start,
    }
}

fn widget_style(snapshot: &Snapshot, mut style: WidgetStyle) -> Result<WidgetStyle, String> {
    for property in snapshot.properties() {
        paint(&mut style, property)?;
        shape(&mut style, property)?;
    }
    Ok(style)
}

fn paint(style: &mut WidgetStyle, property: &Property) -> Result<(), String> {
    match (property.name.as_str(), &property.value) {
        ("background", Value::Color(value)) => style.background = color(*value),
        ("background", Value::Gradient(_)) => {
            return Err("gradient is not supported yet".to_owned());
        }
        ("border", Value::Border(border)) => style.border = border_color(&border.color),
        ("border-color", Value::Color(value)) => style.border = color(*value),
        ("color", Value::Color(value)) => style.text = color(*value),
        ("tint", Value::Color(value)) => style.tint = Some(color(*value)),
        _ => {}
    }
    Ok(())
}

fn shape(style: &mut WidgetStyle, property: &Property) -> Result<(), String> {
    match (property.name.as_str(), &property.value) {
        ("radius", Value::Px(value)) => style.radius = px(*value),
        ("opacity", value) => style.opacity = number(value, "opacity")? as f32,
        _ => {}
    }
    Ok(())
}

fn text_style(snapshot: &Snapshot) -> Result<TextStyle, String> {
    let mut style = TextStyle::default();
    for property in snapshot.properties() {
        font(&mut style, property)?;
    }
    Ok(style)
}

fn font(style: &mut TextStyle, property: &Property) -> Result<(), String> {
    match (property.name.as_str(), &property.value) {
        ("font-size", Value::Px(value)) => {
            style.font_size = *value as f32;
            style.line_height = style.font_size * 1.2;
        }
        ("font-family", Value::String(value)) => style.family = Some(value.clone()),
        ("font-weight", value) => style.weight = number(value, "font-weight")? as u16,
        ("text-align", Value::Keyword(value)) => style.align = text_align(value),
        _ => {}
    }
    Ok(())
}

fn text_align(value: &str) -> brul_gui::TextAlign {
    match value {
        "center" => brul_gui::TextAlign::Center,
        "right" => brul_gui::TextAlign::Right,
        _ => brul_gui::TextAlign::Left,
    }
}

fn dimension(value: &Value, property: &str) -> Result<Dimension, String> {
    match value {
        Value::Px(value) => Ok(Dimension::Px(px(*value))),
        Value::Percent(value) => Ok(Dimension::Percent(*value as f32)),
        _ => Err(format!("expected px or % for '{property}'")),
    }
}

fn number(value: &Value, property: &str) -> Result<f64, String> {
    match value {
        Value::Number(value) => Ok(*value),
        _ => Err(format!("expected number for '{property}'")),
    }
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
