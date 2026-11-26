use brul_gui::{
    Color, Dimension, LayoutStyle, Paint, Shadow, StyleSet, TextEngine, TextStyle, TextStyleSet,
    Widget, WidgetStyle, WidgetTree,
};

use super::DisplayScene;

pub struct Scene;

fn fixed(width: u32, height: u32) -> LayoutStyle {
    LayoutStyle {
        width: Dimension::Px(width),
        height: Dimension::Px(height),
        ..LayoutStyle::default()
    }
}

fn text() -> TextStyleSet {
    let mut style = TextStyle::new(12.0);
    style.family = Some("Fira Mono".to_owned());
    TextStyleSet::new(style)
}

impl DisplayScene for Scene {
    fn root() -> Widget {
        let inner = Widget::row(vec![Widget::text_with_styles("clip", text())])
            .with_layout(fixed(44, 20))
            .with_styles(StyleSet::new(WidgetStyle {
                background: Paint::Color(Color::rgb(200, 90, 40)),
                text: Color::rgb(255, 255, 255),
                border_radius: 6,
                clip: true,
                ..WidgetStyle::default()
            }));
        let card = Widget::column(vec![inner])
            .with_layout(fixed(80, 48))
            .with_styles(StyleSet::new(WidgetStyle {
                background: Paint::Color(Color::rgb(20, 24, 34)),
                border_radius: 8,
                clip: true,
                shadow: Shadow {
                    x: 2,
                    y: 3,
                    blur: 4,
                    color: Color::rgba(0, 0, 0, 180),
                },
                ..WidgetStyle::default()
            }));
        Widget::column(vec![card]).with_layout(fixed(120, 80))
    }

    fn prepare(_tree: &mut WidgetTree, _engine: &mut TextEngine) {}
}
