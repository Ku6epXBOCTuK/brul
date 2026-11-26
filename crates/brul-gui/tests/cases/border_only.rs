use brul_gui::{
    BorderColors, BorderWidths, Color, Dimension, Direction, Dirty, DrawCommand, LayoutStyle, Size,
    StyleSet, TextEngine, Widget, WidgetStyle, WidgetTree, draw_commands,
};

#[test]
fn a_border_only_widget_is_drawn() {
    let view = Widget::view(Direction::Row, Vec::new())
        .with_layout(LayoutStyle {
            width: Dimension::Px(10),
            height: Dimension::Px(10),
            border_width: BorderWidths::all(2),
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            border: BorderColors::all(Color::rgb(0, 0, 255)),
            ..WidgetStyle::default()
        }));

    let mut tree = WidgetTree::new(view);
    let mut engine = TextEngine::new();
    tree.layout(
        &mut engine,
        Size {
            width: 100,
            height: 100,
        },
        1.0,
    )
    .expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");
    let mut commands = Vec::new();
    draw_commands(snapshot, &Dirty::all(), &mut commands);

    assert!(
        commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Shape(_)))
    );
}
