use brul_gui::{
    Align, Color, Dimension, Justify, LayoutStyle, StyleSet, StyleState, TextAlign, TextStyle,
    WidgetStyle,
};

pub(super) fn layout(layout: LayoutStyle) -> Option<String> {
    let default = LayoutStyle::default();
    let mut fields = Vec::new();
    if layout.width != default.width {
        fields.push(format!("width: {}", dimension(layout.width)));
    }
    if layout.height != default.height {
        fields.push(format!("height: {}", dimension(layout.height)));
    }
    if layout.padding != default.padding {
        fields.push(format!(
            "padding: brul_gui::Padding {{ left: {}, top: {}, right: {}, bottom: {} }}",
            layout.padding.left, layout.padding.top, layout.padding.right, layout.padding.bottom
        ));
    }
    if layout.border_width != default.border_width {
        fields.push(format!("border_width: {}", layout.border_width));
    }
    if layout.spacing != default.spacing {
        fields.push(format!("spacing: {}", layout.spacing));
    }
    if layout.stretch != default.stretch {
        fields.push(format!("stretch: {:?}f32", layout.stretch));
    }
    if layout.justify != default.justify {
        fields.push(format!("justify: {}", justify(layout.justify)));
    }
    if layout.align != default.align {
        fields.push(format!("align: {}", align(layout.align)));
    }
    (!fields.is_empty()).then(|| {
        format!(
            "brul_gui::LayoutStyle {{ {}, ..brul_gui::LayoutStyle::default() }}",
            fields.join(", ")
        )
    })
}

pub(super) fn style_set(set: StyleSet) -> Option<String> {
    let base = set.base();
    let mut states = Vec::new();
    for state in StyleState::ALL {
        let style = set.resolve(state.flags());
        if style != base {
            states.push(format!(
                ".with(brul_gui::StyleState::{state:?}, {})",
                widget_style(style)
            ));
        }
    }
    if base == WidgetStyle::default() && states.is_empty() {
        return None;
    }
    let mut out = format!("brul_gui::StyleSet::new({})", widget_style(base));
    for state in states {
        out.push_str(&state);
    }
    Some(out)
}

fn widget_style(style: WidgetStyle) -> String {
    let default = WidgetStyle::default();
    let mut fields = Vec::new();
    if style.background != default.background {
        fields.push(format!("background: {}", color(style.background)));
    }
    if style.border != default.border {
        fields.push(format!("border: {}", color(style.border)));
    }
    if style.text != default.text {
        fields.push(format!("text: {}", color(style.text)));
    }
    if style.radius != default.radius {
        fields.push(format!("radius: {}", style.radius));
    }
    if style.tint != default.tint {
        fields.push(format!(
            "tint: Some({})",
            color(style.tint.unwrap_or(Color::TRANSPARENT))
        ));
    }
    if style.opacity != default.opacity {
        fields.push(format!("opacity: {:?}f32", style.opacity));
    }
    if fields.is_empty() {
        "brul_gui::WidgetStyle::default()".to_owned()
    } else {
        format!(
            "brul_gui::WidgetStyle {{ {}, ..brul_gui::WidgetStyle::default() }}",
            fields.join(", ")
        )
    }
}

pub(super) fn text_style(style: TextStyle) -> Option<String> {
    let default = TextStyle::default();
    let mut fields = Vec::new();
    if style.family != default.family {
        let family = style.family.as_deref().unwrap_or_default();
        fields.push(format!("family: Some({family:?}.to_owned())"));
    }
    if style.font_size != default.font_size {
        fields.push(format!("font_size: {:?}f32", style.font_size));
    }
    if style.line_height != default.line_height {
        fields.push(format!("line_height: {:?}f32", style.line_height));
    }
    if style.weight != default.weight {
        fields.push(format!("weight: {}", style.weight));
    }
    if style.align != default.align {
        fields.push(format!("align: {}", text_align(style.align)));
    }
    (!fields.is_empty()).then(|| {
        format!(
            "brul_gui::TextStyle {{ {}, ..brul_gui::TextStyle::default() }}",
            fields.join(", ")
        )
    })
}

fn dimension(value: Dimension) -> String {
    match value {
        Dimension::Auto => "brul_gui::Dimension::Auto".to_owned(),
        Dimension::Px(value) => format!("brul_gui::Dimension::Px({value})"),
        Dimension::Percent(value) => format!("brul_gui::Dimension::Percent({value:?}f32)"),
    }
}

fn justify(value: Justify) -> String {
    match value {
        Justify::Start => "brul_gui::Justify::Start".to_owned(),
        Justify::Center => "brul_gui::Justify::Center".to_owned(),
        Justify::End => "brul_gui::Justify::End".to_owned(),
        Justify::SpaceBetween => "brul_gui::Justify::SpaceBetween".to_owned(),
    }
}

fn align(value: Align) -> String {
    match value {
        Align::Start => "brul_gui::Align::Start".to_owned(),
        Align::Center => "brul_gui::Align::Center".to_owned(),
        Align::End => "brul_gui::Align::End".to_owned(),
        Align::Stretch => "brul_gui::Align::Stretch".to_owned(),
    }
}

fn text_align(value: TextAlign) -> String {
    match value {
        TextAlign::Left => "brul_gui::TextAlign::Left".to_owned(),
        TextAlign::Center => "brul_gui::TextAlign::Center".to_owned(),
        TextAlign::Right => "brul_gui::TextAlign::Right".to_owned(),
    }
}

fn color(value: Color) -> String {
    format!(
        "brul_gui::Color::rgba({}, {}, {}, {})",
        value.r, value.g, value.b, value.a
    )
}

#[cfg(test)]
mod tests {
    use super::justify;
    use brul_gui::Justify;

    #[test]
    fn every_justify_maps_to_its_own_constant() {
        let mut constants: Vec<String> = [
            Justify::Start,
            Justify::Center,
            Justify::End,
            Justify::SpaceBetween,
        ]
        .iter()
        .map(|value| justify(*value))
        .collect();
        assert!(
            constants
                .iter()
                .all(|constant| constant.starts_with("brul_gui::Justify::"))
        );
        constants.sort_unstable();
        constants.dedup();
        assert_eq!(constants.len(), 4);
    }
}
