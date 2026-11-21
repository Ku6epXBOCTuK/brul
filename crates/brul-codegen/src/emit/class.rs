use std::fmt::Write;

use brul_gui::{
    Class, Layers, LayoutPatch, StyleState, TextStylePatch, Transitions, WidgetStylePatch,
};

use super::Out;

macro_rules! fields {
    ($out:expr, $patch:expr, $type:ident, { $($field:ident $(=> $convert:path)?),+ $(,)? }) => {{
        let brul_gui::$type { $($field),+ } = $patch;
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

pub(super) fn literal(out: &mut Out<'_>, class: &Class) {
    out.line(format_args!("brul::gui::Class {{"));
    out.indented(|out| {
        out.line(format_args!("layout: {},", layout(&class.layout)));
        out.start(format_args!("style: "));
        style(out, &class.style);
        out.push(format_args!(","));
        out.end();
        out.start(format_args!("text: "));
        text(out, &class.text);
        out.push(format_args!(","));
        out.end();
        out.line(format_args!(
            "transitions: {},",
            transitions(&class.transitions)
        ));
    });
    out.start(format_args!("}}"));
}

fn layout(patch: &LayoutPatch) -> String {
    let mut out = String::new();
    fields!(out, patch, LayoutPatch, {
        width => convert::dimension,
        height => convert::dimension,
        padding => convert::padding,
        border_width => convert::border_widths,
        gap => convert::decimal,
        row_gap => convert::decimal,
        column_gap => convert::decimal,
        flex_grow => convert::debug,
        flex_shrink => convert::debug,
        flex_basis => convert::dimension,
        display => convert::display,
        flex_direction => convert::direction,
        justify => convert::justify,
        align => convert::align,
        grid_template_columns => convert::tracks,
        grid_template_rows => convert::tracks,
        grid_column => convert::grid_line,
        grid_row => convert::grid_line,
        grid_auto_flow => convert::auto_flow,
        grid_template_areas => convert::area_template,
        grid_area => convert::family,
    });
    out
}

fn style(out: &mut Out<'_>, layers: &Layers<WidgetStylePatch>) {
    out.push(format_args!(
        "brul::gui::Layers::new({})",
        widget_patch(layers.base())
    ));
    states(out, layers, widget_patch);
}

fn text(out: &mut Out<'_>, layers: &Layers<TextStylePatch>) {
    out.push(format_args!(
        "brul::gui::Layers::new({})",
        text_patch(layers.base())
    ));
    states(out, layers, text_patch);
}

fn states<T>(out: &mut Out<'_>, layers: &Layers<T>, patch_text: impl Fn(&T) -> String) {
    for &state in StyleState::ALL {
        if let Some(patch) = layers.patch(state) {
            out.end();
            out.indented(|out| {
                out.start(format_args!(
                    ".with(brul::gui::StyleState::{state:?}, {})",
                    patch_text(patch)
                ));
            });
        }
    }
}

fn widget_patch(patch: &WidgetStylePatch) -> String {
    let mut out = String::new();
    fields!(out, patch, WidgetStylePatch, {
        background => convert::paint,
        border_left => convert::color,
        border_top => convert::color,
        border_right => convert::color,
        border_bottom => convert::color,
        text => convert::color,
        tint => convert::color,
        opacity => convert::debug,
        vertical_align => convert::vertical_align,
        clip,
        shadow => convert::color,
        border_radius,
        scroll,
        shadow_x,
        shadow_y,
        shadow_blur,
    });
    out
}

fn text_patch(patch: &TextStylePatch) -> String {
    let mut out = String::new();
    fields!(out, patch, TextStylePatch, {
        weight,
        align => convert::text_align,
        size => convert::float,
        family => convert::family,
    });
    out
}

fn transitions(transitions: &Transitions) -> String {
    let mut out = String::new();
    fields!(out, transitions, Transitions, {
        background => convert::millis,
        border_left => convert::millis,
        border_top => convert::millis,
        border_right => convert::millis,
        border_bottom => convert::millis,
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
        Align, AreaTemplate, Color, Dimension, Direction, Display, Gradient, GridAutoFlow,
        GridLine, Justify, LinePlacement, Padding, Paint, RepeatCount, TextAlign, TrackSize,
        VerticalAlign,
    };

    pub(super) fn padding(value: &Padding) -> String {
        format!(
            "brul::gui::Padding {{ left: {}, top: {}, right: {}, bottom: {} }}",
            value.left, value.top, value.right, value.bottom
        )
    }

    pub(super) fn dimension(value: &Dimension) -> String {
        match value {
            Dimension::Auto => "brul::gui::Dimension::Auto".to_owned(),
            Dimension::Px(px) => format!("brul::gui::Dimension::Px({px})"),
            Dimension::Percent(percent) => format!("brul::gui::Dimension::Percent({percent:?})"),
        }
    }

    pub(super) fn display(value: &Display) -> String {
        format!("brul::gui::Display::{value:?}")
    }

    pub(super) fn direction(value: &Direction) -> String {
        format!("brul::gui::Direction::{value:?}")
    }

    pub(super) fn justify(value: &Justify) -> String {
        format!("brul::gui::Justify::{value:?}")
    }

    pub(super) fn align(value: &Align) -> String {
        format!("brul::gui::Align::{value:?}")
    }

    pub(super) fn vertical_align(value: &VerticalAlign) -> String {
        format!("brul::gui::VerticalAlign::{value:?}")
    }

    pub(super) fn text_align(value: &TextAlign) -> String {
        format!("brul::gui::TextAlign::{value:?}")
    }

    pub(super) fn paint(value: &Paint) -> String {
        match value {
            Paint::Color(value) => format!("brul::gui::Paint::Color({})", color(value)),
            Paint::Gradient(Gradient::Linear { angle, from, to }) => format!(
                "brul::gui::Paint::Gradient(brul::gui::Gradient::Linear {{ angle: {angle:?}, from: {}, to: {} }})",
                color(from),
                color(to)
            ),
        }
    }

    pub(super) fn tracks(value: &[TrackSize]) -> String {
        format!(
            "vec![{}]",
            value.iter().map(track_size).collect::<Vec<_>>().join(", ")
        )
    }

    fn track_size(value: &TrackSize) -> String {
        match value {
            TrackSize::Px(px) => format!("brul::gui::TrackSize::Px({px:?})"),
            TrackSize::Percent(percent) => {
                format!("brul::gui::TrackSize::Percent({percent:?})")
            }
            TrackSize::Fr(fr) => format!("brul::gui::TrackSize::Fr({fr:?})"),
            TrackSize::Minmax(min, max) => format!(
                "brul::gui::TrackSize::Minmax(Box::new({}), Box::new({}))",
                track_size(min),
                track_size(max)
            ),
            TrackSize::Repeat(count, tracks) => format!(
                "brul::gui::TrackSize::Repeat({}, vec![{}])",
                repeat_count(count),
                tracks.iter().map(track_size).collect::<Vec<_>>().join(", ")
            ),
        }
    }

    fn repeat_count(count: &RepeatCount) -> String {
        match count {
            RepeatCount::Count(value) => format!("brul::gui::RepeatCount::Count({value})"),
            RepeatCount::AutoFill => "brul::gui::RepeatCount::AutoFill".to_owned(),
            RepeatCount::AutoFit => "brul::gui::RepeatCount::AutoFit".to_owned(),
        }
    }

    pub(super) fn grid_line(value: &GridLine) -> String {
        format!(
            "brul::gui::GridLine {{ start: {}, end: {} }}",
            placement(&value.start),
            value.end.as_ref().map_or_else(
                || "None".to_owned(),
                |end| format!("Some({})", placement(end))
            )
        )
    }

    fn placement(value: &LinePlacement) -> String {
        match value {
            LinePlacement::Auto => "brul::gui::LinePlacement::Auto".to_owned(),
            LinePlacement::Line(line) => format!("brul::gui::LinePlacement::Line({line})"),
            LinePlacement::Span(span) => format!("brul::gui::LinePlacement::Span({span})"),
        }
    }

    pub(super) fn auto_flow(value: &GridAutoFlow) -> String {
        format!("brul::gui::GridAutoFlow::{value:?}")
    }

    pub(super) fn area_template(value: &AreaTemplate) -> String {
        let rows = value.rows.iter().map(|row| {
            let cells = row
                .iter()
                .map(|cell| match cell {
                    Some(name) => format!("Some(String::from({name:?}))"),
                    None => "None".to_owned(),
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("vec![{cells}]")
        });
        format!(
            "brul::gui::AreaTemplate {{ rows: vec![{}] }}",
            rows.collect::<Vec<_>>().join(", ")
        )
    }

    pub(super) fn color(value: &Color) -> String {
        format!(
            "brul::gui::Color {{ r: {}, g: {}, b: {}, a: {} }}",
            value.r, value.g, value.b, value.a
        )
    }

    pub(super) fn decimal(value: &u32) -> String {
        value.to_string()
    }

    pub(super) fn border_widths(value: &brul_gui::BorderWidths) -> String {
        format!(
            "brul::gui::BorderWidths {{ left: {}, top: {}, right: {}, bottom: {} }}",
            value.left, value.top, value.right, value.bottom
        )
    }

    pub(super) fn float(value: &f32) -> String {
        format!("{value:?}")
    }

    pub(super) fn family(value: &String) -> String {
        format!("String::from({value:?})")
    }

    pub(super) fn debug<T: std::fmt::Debug>(value: &T) -> String {
        format!("{value:?}")
    }

    pub(super) fn millis(value: &Duration) -> String {
        format!("std::time::Duration::from_millis({})", value.as_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::layout;
    use brul_gui::{
        AreaTemplate, Display, GridAutoFlow, GridLine, LayoutPatch, LinePlacement, RepeatCount,
        TrackSize,
    };

    #[test]
    fn a_layout_patch_emits_its_grid_fields() {
        let patch = LayoutPatch {
            display: Some(Display::Grid),
            row_gap: Some(6),
            column_gap: Some(7),
            grid_template_columns: Some(vec![
                TrackSize::Fr(1.0),
                TrackSize::Repeat(RepeatCount::AutoFill, vec![TrackSize::Px(40.0)]),
            ]),
            grid_column: Some(GridLine {
                start: LinePlacement::Span(2),
                end: None,
            }),
            grid_auto_flow: Some(GridAutoFlow::Column),
            grid_template_areas: Some(AreaTemplate {
                rows: vec![vec![Some("head".to_owned()), None]],
            }),
            grid_area: Some("head".to_owned()),
            ..LayoutPatch::DEFAULT
        };
        let emitted = layout(&patch);
        assert!(emitted.contains("display: Some(brul::gui::Display::Grid)"));
        assert!(emitted.contains("row_gap: Some(6)"));
        assert!(emitted.contains("column_gap: Some(7)"));
        assert!(emitted.contains(
            "grid_template_columns: Some(vec![brul::gui::TrackSize::Fr(1.0), brul::gui::TrackSize::Repeat(brul::gui::RepeatCount::AutoFill, vec![brul::gui::TrackSize::Px(40.0)])])"
        ));
        assert!(emitted.contains(
            "grid_column: Some(brul::gui::GridLine { start: brul::gui::LinePlacement::Span(2), end: None })"
        ));
        assert!(emitted.contains("grid_auto_flow: Some(brul::gui::GridAutoFlow::Column)"));
        assert!(emitted.contains(
            "grid_template_areas: Some(brul::gui::AreaTemplate { rows: vec![vec![Some(String::from(\"head\")), None]] })"
        ));
        assert!(emitted.contains("grid_area: Some(String::from(\"head\"))"));
        syn::parse_str::<syn::Expr>(&emitted).unwrap();
    }
}
