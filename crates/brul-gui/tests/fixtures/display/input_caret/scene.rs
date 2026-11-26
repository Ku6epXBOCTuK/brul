use brul_gui::{
    Color, Dimension, LayoutStyle, Paint, StyleSet, TextEngine, TextStyle, TextStyleSet, Widget,
    WidgetId, WidgetStyle, WidgetTree,
};

use super::DisplayScene;

const INPUT: WidgetId = WidgetId::new(7);

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
        let input = Widget::input("ab")
            .with_id(INPUT)
            .with_layout(fixed(80, 24))
            .with_text_styles(text())
            .with_styles(StyleSet::new(WidgetStyle {
                background: Paint::Color(Color::rgb(18, 20, 26)),
                text: Color::rgb(240, 240, 240),
                border_radius: 2,
                ..WidgetStyle::default()
            }));
        Widget::column(vec![input]).with_layout(fixed(120, 48))
    }

    fn prepare(_tree: &mut WidgetTree, _engine: &mut TextEngine) {}
}
