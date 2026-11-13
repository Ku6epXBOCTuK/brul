use brul_gui::{
    Dimension, Direction, KeyModifiers, LayoutStyle, Size, TextEngine, Widget, WidgetEvent,
    WidgetTree,
};

fn fixed(width: u32, height: u32) -> LayoutStyle {
    LayoutStyle {
        width: Dimension::Px(width),
        height: Dimension::Px(height),
        ..LayoutStyle::default()
    }
}

fn laid_out(root: Widget) -> WidgetTree {
    let mut tree = WidgetTree::new(root);
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
    tree
}

fn field() -> Widget {
    Widget::view(Direction::Column, vec![])
        .with_layout(fixed(40, 20))
        .focusable(true)
}

fn focus_events(events: Vec<WidgetEvent>) -> Vec<WidgetEvent> {
    events
        .into_iter()
        .filter(|event| matches!(event, WidgetEvent::Focus(_) | WidgetEvent::Blur(_)))
        .collect()
}

#[test]
fn a_click_focuses_the_nearest_focusable_widget_and_background_blurs_it() {
    let (first, second, third) = (field(), field(), field());
    let (first_id, second_id) = (first.id(), second.id());
    let mut tree =
        laid_out(Widget::column(vec![first, second, third]).with_layout(fixed(100, 100)));

    assert_eq!(
        focus_events(tree.pointer_down(5, 5).expect("down")),
        vec![WidgetEvent::Focus(first_id)]
    );
    assert_eq!(tree.focused(), Some(first_id));

    assert_eq!(
        focus_events(tree.pointer_down(5, 25).expect("down")),
        vec![WidgetEvent::Blur(first_id), WidgetEvent::Focus(second_id)]
    );
    assert_eq!(tree.focused(), Some(second_id));

    assert_eq!(
        focus_events(tree.pointer_down(95, 95).expect("down")),
        vec![WidgetEvent::Blur(second_id)]
    );
    assert_eq!(tree.focused(), None);
}

#[test]
fn tab_walks_the_focusable_widgets_and_keeps_other_keys_for_the_window() {
    let (first, second, third) = (field(), field(), field());
    let (first_id, second_id, third_id) = (first.id(), second.id(), third.id());
    let mut tree =
        laid_out(Widget::column(vec![first, second, third]).with_layout(fixed(100, 100)));

    assert_eq!(tree.press_key("Tab"), vec![WidgetEvent::Focus(first_id)]);
    assert_eq!(
        tree.press_key("Tab"),
        vec![WidgetEvent::Blur(first_id), WidgetEvent::Focus(second_id)]
    );
    tree.press_key("Tab");
    assert_eq!(tree.focused(), Some(third_id));
    assert_eq!(
        tree.press_key("Tab"),
        vec![WidgetEvent::Blur(third_id), WidgetEvent::Focus(first_id)]
    );
    assert_eq!(
        tree.press_key("Escape"),
        vec![WidgetEvent::Key {
            code: "Escape".to_owned(),
            modifiers: KeyModifiers::default(),
        }]
    );
    assert_eq!(tree.focused(), Some(first_id));
}

#[test]
fn a_disabled_widget_is_not_focusable() {
    let (first, second, third) = (field(), field(), field());
    let (first_id, third_id) = (first.id(), third.id());
    let mut tree = laid_out(
        Widget::column(vec![first, second.disabled(true), third]).with_layout(fixed(100, 100)),
    );

    assert_eq!(tree.press_key("Tab"), vec![WidgetEvent::Focus(first_id)]);
    assert_eq!(
        tree.press_key("Tab"),
        vec![WidgetEvent::Blur(first_id), WidgetEvent::Focus(third_id)]
    );
}

#[test]
fn a_widget_without_focusable_takes_no_focus() {
    let plain = Widget::text("x").with_layout(fixed(40, 20));
    let mut tree = laid_out(Widget::column(vec![plain]).with_layout(fixed(100, 100)));

    assert!(tree.press_key("Tab").is_empty());
    assert_eq!(tree.focused(), None);
}
