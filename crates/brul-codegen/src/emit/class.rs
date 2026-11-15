use std::fmt::Write;

use brul_gui::{
    Class, Layers, LayoutPatch, StyleState, TextStylePatch, Transitions, WidgetStylePatch,
};

macro_rules! fields {
    ($out:expr, $patch:expr, $type:ident, { $($field:ident $(=> $convert:path)?),+ $(,)? }) => {{
        let brul_gui::$type { $($field),+ } = *$patch;
        let _ = write!($out, concat!("brul::gui::", stringify!($type), " {{"));
        $(
            if let Some(value) = $field {
                let _ = write!(
                    $out,
                    concat!(" ", stringify!($field), ": Some({}),"),
                    fields!(@value value $($convert)?)
                );
            }
        )+
        let _ = write!(
            $out,
            concat!(" ..brul::gui::", stringify!($type), "::DEFAULT }}")
        );
    }};
    (@value $value:ident) => {
        $value
    };
    (@value $value:ident $convert:path) => {
        $convert($value)
    };
}

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
    let mut out = String::new();
    fields!(out, patch, LayoutPatch, {
        width => convert::dimension,
        height => convert::dimension,
        padding => convert::padding,
        border_width => convert::decimal,
        gap => convert::decimal,
        flex_grow => convert::debug,
        flex_shrink => convert::debug,
        flex_basis => convert::dimension,
        justify => convert::justify,
        align => convert::align,
    });
    out
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
    let mut out = String::new();
    fields!(out, patch, WidgetStylePatch, {
        background => convert::paint,
        border => convert::color,
        text => convert::color,
        tint => convert::color,
        opacity => convert::debug,
        vertical_align => convert::vertical_align,
        clip,
        shadow => convert::color,
    });
    out
}

fn text_patch(patch: &TextStylePatch) -> String {
    let mut out = String::new();
    fields!(out, patch, TextStylePatch, {
        weight,
        align => convert::text_align,
    });
    out
}

fn transitions(transitions: &Transitions) -> String {
    let mut out = String::new();
    fields!(out, transitions, Transitions, {
        background => convert::millis,
        border => convert::millis,
        text => convert::millis,
        tint => convert::millis,
        opacity => convert::millis,
        shadow => convert::millis,
    });
    out
}

mod convert {
    use std::time::Duration;

    use brul_gui::{
        Align, Color, Dimension, Gradient, Justify, Padding, Paint, TextAlign, VerticalAlign,
    };

    pub(super) fn padding(padding: Padding) -> String {
        format!(
            "brul::gui::Padding {{ left: {}, top: {}, right: {}, bottom: {} }}",
            padding.left, padding.top, padding.right, padding.bottom
        )
    }

    pub(super) fn dimension(value: Dimension) -> String {
        match value {
            Dimension::Auto => "brul::gui::Dimension::Auto".to_owned(),
            Dimension::Px(px) => format!("brul::gui::Dimension::Px({px})"),
            Dimension::Percent(percent) => format!("brul::gui::Dimension::Percent({percent:?})"),
        }
    }

    pub(super) fn justify(value: Justify) -> String {
        format!("brul::gui::Justify::{value:?}")
    }

    pub(super) fn align(value: Align) -> String {
        format!("brul::gui::Align::{value:?}")
    }

    pub(super) fn vertical_align(value: VerticalAlign) -> String {
        format!("brul::gui::VerticalAlign::{value:?}")
    }

    pub(super) fn text_align(value: TextAlign) -> String {
        format!("brul::gui::TextAlign::{value:?}")
    }

    pub(super) fn paint(value: Paint) -> String {
        match value {
            Paint::Color(value) => format!("brul::gui::Paint::Color({})", color(value)),
            Paint::Gradient(Gradient::Linear { angle, from, to }) => format!(
                "brul::gui::Paint::Gradient(brul::gui::Gradient::Linear {{ angle: {angle:?}, from: {}, to: {} }})",
                color(from),
                color(to)
            ),
        }
    }

    pub(super) fn color(value: Color) -> String {
        format!(
            "brul::gui::Color {{ r: {}, g: {}, b: {}, a: {} }}",
            value.r, value.g, value.b, value.a
        )
    }

    pub(super) fn decimal(value: u32) -> String {
        value.to_string()
    }

    pub(super) fn debug<T: std::fmt::Debug>(value: T) -> String {
        format!("{value:?}")
    }

    pub(super) fn millis(value: Duration) -> String {
        format!("std::time::Duration::from_millis({})", value.as_millis())
    }
}
