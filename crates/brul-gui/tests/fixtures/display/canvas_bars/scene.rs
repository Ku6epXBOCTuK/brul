use std::rc::Rc;

use brul_gui::{
    CanvasPainter, CanvasSpec, Color, Dimension, LayoutStyle, Paint, PaintCommand, PaintCtx,
    PaintRect, StyleSet, Widget, WidgetId, WidgetStyle, WidgetTree,
};

use super::DisplayScene;

const CANVAS: WidgetId = WidgetId::new(11);

pub struct Scene;

struct Bars;

impl CanvasPainter for Bars {
    fn paint(&self, ctx: &PaintCtx<'_>) -> Vec<PaintCommand> {
        let rect = ctx.rect;
        let height = rect.height.saturating_sub(4) as i32;
        (0..12)
            .map(|index| {
                PaintCommand::Rect(PaintRect {
                    rect: [
                        rect.x + 2 + index * 4,
                        rect.y + 2,
                        3,
                        height.saturating_sub(index * 2),
                    ],
                    color: Color::rgb(40 + index as u8 * 12, 160, 220),
                })
            })
            .collect()
    }
}

impl DisplayScene for Scene {
    fn root() -> Widget {
        let canvas = Widget::canvas(CanvasSpec::new())
            .with_id(CANVAS)
            .with_layout(LayoutStyle {
                width: Dimension::Px(100),
                height: Dimension::Px(40),
                ..LayoutStyle::default()
            })
            .with_styles(StyleSet::new(WidgetStyle {
                background: Paint::Color(Color::rgb(10, 12, 18)),
                clip: true,
                border_radius: 4,
                ..WidgetStyle::default()
            }));
        Widget::column(vec![canvas]).with_layout(LayoutStyle {
            width: Dimension::Px(120),
            height: Dimension::Px(60),
            ..LayoutStyle::default()
        })
    }

    fn prepare(tree: &mut WidgetTree, _engine: &mut brul_gui::TextEngine) {
        tree.set_painter(CANVAS, Rc::new(Bars));
    }
}
