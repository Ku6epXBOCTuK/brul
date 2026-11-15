use std::fmt::Write;

use brul_gui::{
    Align, Class, Color, Dimension, Gradient, Justify, Layers, LayoutPatch, Padding, Paint,
    StyleState, TextStylePatch, Transitions, VerticalAlign, WidgetStylePatch,
};

pub(super) fn literal(class: &Class) -> String {
    format!(
        "brul::gui::Class {{\n\
         \x20   layout: {},\n\
         \x20   style: {},\n\
         \x20   text: {},\n\
         \x20   transitions: {},\n\
         }}",
        layout(&class.layout),
        style(&class.style),
        text(&class.text),
        transitions(&class.transitions)
    )
}

fn layout(patch: &LayoutPatch) -> String {
    let mut out = String::from("brul::gui::LayoutPatch {");
    let field = |out: &mut String, name: &str, value: String| {
        let _ = write!(out, " {name}: Some({value}),");
    };
    if let Some(value) = patch.width {
        field(&mut out, "width", dimension(value));
    }
    if let Some(value) = patch.height {
        field(&mut out, "height", dimension(value));
    }
    if let Some(value) = patch.padding {
        field(&mut out, "padding", padding(value));
    }
    if let Some(value) = patch.border_width {
        field(&mut out, "border_width", value.to_string());
    }
    if let Some(value) = patch.gap {
        field(&mut out, "gap", value.to_string());
    }
    if let Some(value) = patch.flex_grow {
        field(&mut out, "flex_grow", format!("{value:?}"));
    }
    if let Some(value) = patch.flex_shrink {
        field(&mut out, "flex_shrink", format!("{value:?}"));
    }
    if let Some(value) = patch.flex_basis {
        field(&mut out, "flex_basis", dimension(value));
    }
    if let Some(value) = patch.justify {
        field(&mut out, "justify", justify(value));
    }
    if let Some(value) = patch.align {
        field(&mut out, "align", align(value));
    }
    out.push_str(" ..brul::gui::LayoutPatch::DEFAULT }");
    out
}

fn padding(padding: Padding) -> String {
    format!(
        "brul::gui::Padding {{ left: {}, top: {}, right: {}, bottom: {} }}",
        padding.left, padding.top, padding.right, padding.bottom
    )
}

fn dimension(value: Dimension) -> String {
    match value {
        Dimension::Auto => "brul::gui::Dimension::Auto".to_owned(),
        Dimension::Px(px) => format!("brul::gui::Dimension::Px({px})"),
        Dimension::Percent(percent) => format!("brul::gui::Dimension::Percent({percent:?})"),
    }
}

fn justify(value: Justify) -> String {
    format!("brul::gui::Justify::{value:?}")
}

fn align(value: Align) -> String {
    format!("brul::gui::Align::{value:?}")
}

fn style(layers: &Layers<WidgetStylePatch>) -> String {
    let mut out = format!("brul::gui::Layers::new({})", widget_patch(&layers.base()));
    for state in StyleState::ALL {
        if let Some(patch) = layers.patch(state) {
            let _ = write!(
                out,
                "\n        .with(brul::gui::StyleState::{state:?}, {})",
                widget_patch(&patch)
            );
        }
    }
    out
}

fn text(layers: &Layers<TextStylePatch>) -> String {
    let mut out = format!("brul::gui::Layers::new({})", text_patch(&layers.base()));
    for state in StyleState::ALL {
        if let Some(patch) = layers.patch(state) {
            let _ = write!(
                out,
                "\n        .with(brul::gui::StyleState::{state:?}, {})",
                text_patch(&patch)
            );
        }
    }
    out
}

fn widget_patch(patch: &WidgetStylePatch) -> String {
    let mut out = String::from("brul::gui::WidgetStylePatch {");
    if let Some(value) = &patch.background {
        let _ = write!(out, " background: Some({}),", paint(value));
    }
    if let Some(value) = patch.border {
        let _ = write!(out, " border: Some({}),", color(value));
    }
    if let Some(value) = patch.text {
        let _ = write!(out, " text: Some({}),", color(value));
    }
    if let Some(value) = patch.tint {
        let _ = write!(out, " tint: Some({}),", color(value));
    }
    if let Some(value) = patch.opacity {
        let _ = write!(out, " opacity: Some({value:?}),");
    }
    if let Some(value) = patch.vertical_align {
        let _ = write!(out, " vertical_align: Some({}),", vertical_align(value));
    }
    if let Some(value) = patch.clip {
        let _ = write!(out, " clip: Some({value}),");
    }
    if let Some(value) = patch.shadow {
        let _ = write!(out, " shadow: Some({}),", color(value));
    }
    out.push_str(" ..brul::gui::WidgetStylePatch::DEFAULT }");
    out
}

fn text_patch(patch: &TextStylePatch) -> String {
    let mut out = String::from("brul::gui::TextStylePatch {");
    if let Some(value) = patch.weight {
        let _ = write!(out, " weight: Some({value}),");
    }
    if let Some(value) = patch.align {
        let _ = write!(out, " align: Some(brul::gui::TextAlign::{value:?}),");
    }
    out.push_str(" ..brul::gui::TextStylePatch::DEFAULT }");
    out
}

fn transitions(transitions: &Transitions) -> String {
    let mut out = String::from("brul::gui::Transitions {");
    let fields = [
        ("background", transitions.background),
        ("border", transitions.border),
        ("text", transitions.text),
        ("tint", transitions.tint),
        ("opacity", transitions.opacity),
        ("shadow", transitions.shadow),
    ];
    for (name, duration) in fields {
        if let Some(duration) = duration {
            let _ = write!(
                out,
                " {name}: Some(std::time::Duration::from_millis({})),",
                duration.as_millis()
            );
        }
    }
    out.push_str(" ..brul::gui::Transitions::DEFAULT }");
    out
}

fn paint(value: &Paint) -> String {
    match value {
        Paint::Color(value) => format!("brul::gui::Paint::Color({})", color(*value)),
        Paint::Gradient(Gradient::Linear { angle, from, to }) => format!(
            "brul::gui::Paint::Gradient(brul::gui::Gradient::Linear {{ angle: {angle:?}, from: {}, to: {} }})",
            color(*from),
            color(*to)
        ),
    }
}

fn color(value: Color) -> String {
    format!(
        "brul::gui::Color {{ r: {}, g: {}, b: {}, a: {} }}",
        value.r, value.g, value.b, value.a
    )
}

fn vertical_align(value: VerticalAlign) -> String {
    format!("brul::gui::VerticalAlign::{value:?}")
}
