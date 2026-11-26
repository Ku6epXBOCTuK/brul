use brul_gui::{
    Color, Dimension, LayoutStyle, Paint, StyleSet, StyleState, TextEngine, TextStyle,
    TextStyleSet, Widget, WidgetId, WidgetStyle, WidgetStylePatch, WidgetTree,
};

use super::DisplayScene;

const BUTTON: WidgetId = WidgetId::new(7);

pub struct Scene;

fn fixed(width: u32, height: u32) -> LayoutStyle {
    LayoutStyle {
        width: Dimension::Px(width),
        height: Dimension::Px(height),
        ..LayoutStyle::default()
    }
}

fn text() -> TextStyleSet {
    let mut style = TextStyle::new(13.0);
    style.family = Some("Fira Mono".to_owned());
    TextStyleSet::new(style)
}

impl DisplayScene for Scene {
    fn root() -> Widget {
        let button = Widget::row(vec![Widget::text_with_styles("OK", text())])
            .with_id(BUTTON)
            .focusable(true)
            .with_layout(fixed(64, 24))
            .with_styles(
                StyleSet::new(WidgetStyle {
                    background: Paint::Color(Color::rgb(30, 40, 50)),
                    text: Color::rgb(230, 230, 230),
                    border_radius: 4,
                    ..WidgetStyle::default()
                })
                .with(
                    StyleState::Hovered,
                    WidgetStylePatch {
                        background: Some(Paint::Color(Color::rgb(40, 70, 100))),
                        ..WidgetStylePatch::default()
                    },
                )
                .with(
                    StyleState::Pressed,
                    WidgetStylePatch {
                        background: Some(Paint::Color(Color::rgb(20, 30, 40))),
                        ..WidgetStylePatch::default()
                    },
                )
                .with(
                    StyleState::Focused,
                    WidgetStylePatch {
                        background: Some(Paint::Color(Color::rgb(45, 50, 65))),
                        ..WidgetStylePatch::default()
                    },
                )
                .with(
                    StyleState::Checked,
                    WidgetStylePatch {
                        background: Some(Paint::Color(Color::rgb(60, 90, 40))),
                        ..WidgetStylePatch::default()
                    },
                )
                .with(
                    StyleState::Disabled,
                    WidgetStylePatch {
                        background: Some(Paint::Color(Color::rgb(24, 24, 24))),
                        ..WidgetStylePatch::default()
                    },
                ),
            );
        Widget::column(vec![button]).with_layout(fixed(120, 48))
    }

    fn prepare(_tree: &mut WidgetTree, _engine: &mut TextEngine) {}
}
