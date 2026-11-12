use brul_gui::{
    Align, Class, Color, Dimension, Gradient, Justify, LayoutPatch, LayoutStyle, Paint, Shadow,
    StyleSet, StyleState, TextAlign, TextStyle, TextStylePatch, TextStyleSet, Transitions,
    VerticalAlign, WidgetStyle, WidgetStylePatch,
};

pub(super) fn layout(layout: LayoutStyle) -> Option<String> {
    let LayoutStyle {
        width,
        height,
        padding,
        border_width,
        spacing,
        stretch,
        justify: justify_value,
        align: align_value,
    } = layout;
    let default = LayoutStyle::default();
    let mut fields = Vec::new();
    if width != default.width {
        fields.push(format!("width: {}", dimension(width)));
    }
    if height != default.height {
        fields.push(format!("height: {}", dimension(height)));
    }
    if padding != default.padding {
        fields.push(format!(
            "padding: brul_gui::Padding {{ left: {}, top: {}, right: {}, bottom: {} }}",
            padding.left, padding.top, padding.right, padding.bottom
        ));
    }
    if border_width != default.border_width {
        fields.push(format!("border_width: {border_width}"));
    }
    if spacing != default.spacing {
        fields.push(format!("spacing: {spacing}"));
    }
    if stretch != default.stretch {
        fields.push(format!("stretch: {stretch:?}f32"));
    }
    if justify_value != default.justify {
        fields.push(format!("justify: {}", justify(justify_value)));
    }
    if align_value != default.align {
        fields.push(format!("align: {}", align(align_value)));
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
    let transitions = set.transitions();
    let mut states = Vec::new();
    for state in StyleState::ALL {
        let Some(patch) = set.patch(state) else {
            continue;
        };
        states.push(format!(
            ".with(brul_gui::StyleState::{state:?}, {})",
            widget_style_patch(patch)
        ));
    }
    if base == WidgetStyle::default() && states.is_empty() && transitions.is_empty() {
        return None;
    }
    let mut out = format!("brul_gui::StyleSet::new({})", widget_style(base));
    if !transitions.is_empty() {
        out.push_str(&format!(
            ".with_transitions({})",
            transitions_expr(transitions)
        ));
    }
    for state in states {
        out.push_str(&state);
    }
    Some(out)
}

fn transitions_expr(value: Transitions) -> String {
    let fields = [
        ("background", value.background),
        ("border", value.border),
        ("text", value.text),
        ("tint", value.tint),
        ("opacity", value.opacity),
        ("shadow", value.shadow),
    ];
    let fields: Vec<String> = fields
        .into_iter()
        .filter_map(|(name, duration)| {
            duration.map(|duration| {
                format!(
                    "{name}: Some(std::time::Duration::from_millis({}))",
                    duration.as_millis()
                )
            })
        })
        .collect();
    if fields.is_empty() {
        "brul_gui::Transitions::default()".to_owned()
    } else {
        format!(
            "brul_gui::Transitions {{ {}, ..brul_gui::Transitions::default() }}",
            fields.join(", ")
        )
    }
}

fn widget_style(style: WidgetStyle) -> String {
    let WidgetStyle {
        background,
        border,
        text,
        radius,
        tint,
        opacity,
        vertical_align: vertical_align_value,
        clip,
        scroll,
        shadow: shadow_value,
    } = style;
    let default = WidgetStyle::default();
    let mut fields = Vec::new();
    if background != default.background {
        fields.push(format!("background: {}", paint(background)));
    }
    if border != default.border {
        fields.push(format!("border: {}", color(border)));
    }
    if text != default.text {
        fields.push(format!("text: {}", color(text)));
    }
    if radius != default.radius {
        fields.push(format!("radius: {radius}"));
    }
    if tint != default.tint {
        fields.push(format!(
            "tint: Some({})",
            color(tint.unwrap_or(Color::TRANSPARENT))
        ));
    }
    if opacity != default.opacity {
        fields.push(format!("opacity: {opacity:?}f32"));
    }
    if vertical_align_value != default.vertical_align {
        fields.push(format!(
            "vertical_align: {}",
            vertical_align(vertical_align_value)
        ));
    }
    if clip != default.clip {
        fields.push(format!("clip: {clip}"));
    }
    if scroll != default.scroll {
        fields.push(format!("scroll: {scroll}"));
    }
    if shadow_value != default.shadow {
        fields.push(format!("shadow: {}", shadow(shadow_value)));
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

pub(super) fn class(value: &Class) -> String {
    format!(
        "brul_gui::Class {{ layout: {}, style: {}, text: {} }}",
        layout_patch(&value.layout),
        widget_style_patch(&value.style),
        text_style_patch(&value.text)
    )
}

fn layout_patch(patch: &LayoutPatch) -> String {
    let mut fields = Vec::new();
    if let Some(width) = patch.width {
        fields.push(format!("width: Some({})", dimension(width)));
    }
    if let Some(height) = patch.height {
        fields.push(format!("height: Some({})", dimension(height)));
    }
    if let Some(padding) = patch.padding {
        fields.push(format!(
            "padding: Some(brul_gui::Padding {{ left: {}, top: {}, right: {}, bottom: {} }})",
            padding.left, padding.top, padding.right, padding.bottom
        ));
    }
    if let Some(border_width) = patch.border_width {
        fields.push(format!("border_width: Some({border_width})"));
    }
    if let Some(spacing) = patch.spacing {
        fields.push(format!("spacing: Some({spacing})"));
    }
    if let Some(stretch) = patch.stretch {
        fields.push(format!("stretch: Some({stretch:?}f32)"));
    }
    if let Some(value) = patch.justify {
        fields.push(format!("justify: Some({})", justify(value)));
    }
    if let Some(value) = patch.align {
        fields.push(format!("align: Some({})", align(value)));
    }
    if fields.is_empty() {
        "brul_gui::LayoutPatch::default()".to_owned()
    } else {
        format!(
            "brul_gui::LayoutPatch {{ {}, ..brul_gui::LayoutPatch::default() }}",
            fields.join(", ")
        )
    }
}

fn widget_style_patch(patch: &WidgetStylePatch) -> String {
    let mut fields = Vec::new();
    if let Some(background) = patch.background {
        fields.push(format!("background: Some({})", paint(background)));
    }
    if let Some(border) = patch.border {
        fields.push(format!("border: Some({})", color(border)));
    }
    if let Some(text) = patch.text {
        fields.push(format!("text: Some({})", color(text)));
    }
    if let Some(tint) = patch.tint {
        fields.push(format!("tint: Some({})", color(tint)));
    }
    if let Some(opacity) = patch.opacity {
        fields.push(format!("opacity: Some({opacity:?}f32)"));
    }
    if let Some(align) = patch.vertical_align {
        fields.push(format!("vertical_align: Some({})", vertical_align(align)));
    }
    if let Some(clip) = patch.clip {
        fields.push(format!("clip: Some({clip})"));
    }
    if let Some(shadow) = patch.shadow {
        fields.push(format!("shadow: Some({})", color(shadow)));
    }
    if fields.is_empty() {
        "brul_gui::WidgetStylePatch::default()".to_owned()
    } else {
        format!(
            "brul_gui::WidgetStylePatch {{ {}, ..brul_gui::WidgetStylePatch::default() }}",
            fields.join(", ")
        )
    }
}

fn text_style_patch(patch: &TextStylePatch) -> String {
    let mut fields = Vec::new();
    if let Some(weight) = patch.weight {
        fields.push(format!("weight: Some({weight})"));
    }
    if let Some(align) = patch.align {
        fields.push(format!("align: Some({})", text_align(align)));
    }
    if fields.is_empty() {
        "brul_gui::TextStylePatch::default()".to_owned()
    } else {
        format!(
            "brul_gui::TextStylePatch {{ {}, ..brul_gui::TextStylePatch::default() }}",
            fields.join(", ")
        )
    }
}

pub(super) fn text_style_set(set: &TextStyleSet) -> Option<String> {
    let base = set.base();
    let mut states = Vec::new();
    for state in StyleState::ALL {
        let Some(patch) = set.patch(state) else {
            continue;
        };
        states.push(format!(
            ".with(brul_gui::StyleState::{state:?}, {})",
            text_style_patch(patch)
        ));
    }
    if *base == TextStyle::default() && states.is_empty() {
        return None;
    }
    let mut out = format!("brul_gui::TextStyleSet::new({})", text_style_expr(base));
    for state in states {
        out.push_str(&state);
    }
    Some(out)
}

fn text_style_expr(style: &TextStyle) -> String {
    let TextStyle {
        family,
        font_size,
        line_height,
        weight,
        align: align_value,
    } = style;
    let default = TextStyle::default();
    let mut fields = Vec::new();
    if *family != default.family {
        let family = family.as_deref().unwrap_or_default();
        fields.push(format!("family: Some({family:?}.to_owned())"));
    }
    if *font_size != default.font_size {
        fields.push(format!("font_size: {font_size:?}f32"));
    }
    if *line_height != default.line_height {
        fields.push(format!("line_height: {line_height:?}f32"));
    }
    if *weight != default.weight {
        fields.push(format!("weight: {weight}"));
    }
    if *align_value != default.align {
        fields.push(format!("align: {}", text_align(*align_value)));
    }
    if fields.is_empty() {
        "brul_gui::TextStyle::default()".to_owned()
    } else {
        format!(
            "brul_gui::TextStyle {{ {}, ..brul_gui::TextStyle::default() }}",
            fields.join(", ")
        )
    }
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

fn vertical_align(value: VerticalAlign) -> String {
    match value {
        VerticalAlign::Top => "brul_gui::VerticalAlign::Top".to_owned(),
        VerticalAlign::Center => "brul_gui::VerticalAlign::Center".to_owned(),
        VerticalAlign::Bottom => "brul_gui::VerticalAlign::Bottom".to_owned(),
    }
}

fn shadow(value: Shadow) -> String {
    format!(
        "brul_gui::Shadow {{ x: {}, y: {}, blur: {}, color: {} }}",
        value.x,
        value.y,
        value.blur,
        color(value.color)
    )
}

fn paint(value: Paint) -> String {
    match value {
        Paint::Color(value) => format!("brul_gui::Paint::Color({})", color(value)),
        Paint::Gradient(Gradient::Linear { angle, from, to }) => format!(
            "brul_gui::Paint::Gradient(brul_gui::Gradient::Linear {{ angle: {angle:?}f32, from: {}, to: {} }})",
            color(from),
            color(to)
        ),
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
    use std::time::Duration;

    use super::{justify, style_set, widget_style};
    use brul_gui::{Color, Justify, Shadow, StyleSet, Transitions, WidgetStyle};

    #[test]
    fn transitions_are_emitted_next_to_the_base_style() {
        let set = StyleSet::new(WidgetStyle::default()).with_transitions(Transitions {
            background: Some(Duration::from_millis(300)),
            ..Transitions::default()
        });

        assert_eq!(
            style_set(set).as_deref(),
            Some(
                "brul_gui::StyleSet::new(brul_gui::WidgetStyle::default()).with_transitions(brul_gui::Transitions { background: Some(std::time::Duration::from_millis(300)), ..brul_gui::Transitions::default() })"
            )
        );
    }

    #[test]
    fn an_empty_style_set_stays_empty() {
        assert_eq!(style_set(StyleSet::new(WidgetStyle::default())), None);
    }

    #[test]
    fn shadow_is_emitted_only_when_it_differs_from_default() {
        assert_eq!(
            widget_style(WidgetStyle::default()),
            "brul_gui::WidgetStyle::default()"
        );
        let style = WidgetStyle {
            shadow: Shadow {
                x: -2,
                y: 10,
                blur: 15,
                color: Color::rgba(59, 130, 246, 64),
            },
            ..WidgetStyle::default()
        };

        assert_eq!(
            widget_style(style),
            "brul_gui::WidgetStyle { shadow: brul_gui::Shadow { x: -2, y: 10, blur: 15, color: brul_gui::Color::rgba(59, 130, 246, 64) }, ..brul_gui::WidgetStyle::default() }"
        );
    }

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
