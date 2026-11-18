use brul_gui::{
    Color, Dimension, Direction, KeyModifiers, LayoutStyle, Paint, Size, StyleSet, StyleState,
    TextEngine, Widget, WidgetEvent, WidgetStyle, WidgetStylePatch, WidgetTree,
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
fn focus_applies_the_focused_style_and_blur_removes_it() {
    let base = Paint::Color(Color::rgb(1, 2, 3));
    let focused = Paint::Color(Color::rgb(4, 5, 6));
    let field = field().with_styles(
        StyleSet::new(WidgetStyle {
            background: base,
            ..WidgetStyle::default()
        })
        .with(
            StyleState::Focused,
            WidgetStylePatch {
                background: Some(focused),
                ..WidgetStylePatch::default()
            },
        ),
    );
    let id = field.id();
    let mut tree = laid_out(Widget::column(vec![field]).with_layout(fixed(100, 100)));

    assert_eq!(
        focus_events(tree.press_key("Tab")),
        vec![WidgetEvent::Focus(id)]
    );
    {
        let snapshot = tree.snapshot().expect("snapshot");
        let node = snapshot.node(id).expect("field");
        assert!(node.state.focused);
        assert_eq!(node.style.background, focused);
    }

    focus_events(tree.pointer_down(80, 80).expect("down"));
    let snapshot = tree.snapshot().expect("snapshot after blur");
    let node = snapshot.node(id).expect("field after blur");
    assert!(!node.state.focused);
    assert_eq!(node.style.background, base);
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
fn enter_and_space_click_the_focused_widget_and_its_ancestors() {
    let button = field();
    let button_id = button.id();
    let root = Widget::column(vec![button]).with_layout(fixed(100, 100));
    let root_id = root.id();
    let mut tree = laid_out(root);

    assert_eq!(tree.press_key("Tab"), vec![WidgetEvent::Focus(button_id)]);
    for key in ["Enter", " "] {
        assert_eq!(
            tree.press_key(key),
            vec![WidgetEvent::Click(button_id), WidgetEvent::Click(root_id),]
        );
    }
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
fn tab_stays_inside_a_visible_modal_layer() {
    let trigger = field();
    let inside = field();
    let (trigger_id, inside_id) = (trigger.id(), inside.id());
    let dialog = Widget::layer(vec![inside]).modal(true);
    let dialog_id = dialog.id();
    let mut tree = laid_out(Widget::column(vec![trigger, dialog]).with_layout(fixed(100, 100)));

    assert_eq!(tree.press_key("Tab"), vec![WidgetEvent::Focus(inside_id)]);
    assert!(tree.press_key("Tab").is_empty());
    assert_eq!(tree.focused(), Some(inside_id));

    tree.set_modifiers(KeyModifiers {
        shift: true,
        ..KeyModifiers::default()
    });
    assert!(tree.press_key("Tab").is_empty());
    assert_eq!(tree.focused(), Some(inside_id));
    assert_ne!(tree.focused(), Some(trigger_id));
    assert_eq!(
        tree.press_key("Escape"),
        vec![WidgetEvent::Dismiss(dialog_id)]
    );
}

#[test]
fn a_hidden_modal_layer_gives_the_focus_back() {
    let trigger = field();
    let inside = field();
    let (trigger_id, inside_id) = (trigger.id(), inside.id());
    let dialog = Widget::layer(vec![inside]).modal(true);
    let dialog_id = dialog.id();
    let mut tree = laid_out(Widget::column(vec![trigger, dialog]).with_layout(fixed(100, 100)));
    tree.set_visible(dialog_id, false);

    assert_eq!(tree.press_key("Tab"), vec![WidgetEvent::Focus(trigger_id)]);
    tree.set_visible(dialog_id, true);
    assert_eq!(tree.focused(), Some(inside_id));
    tree.set_visible(dialog_id, false);
    assert_eq!(tree.focused(), Some(trigger_id));
}

#[test]
fn a_widget_without_focusable_takes_no_focus() {
    let plain = Widget::text("x").with_layout(fixed(40, 20));
    let mut tree = laid_out(Widget::column(vec![plain]).with_layout(fixed(100, 100)));

    assert!(tree.press_key("Tab").is_empty());
    assert_eq!(tree.focused(), None);
}

#[test]
fn replacing_the_same_children_keeps_the_focus_on_its_widget() {
    let (first, second) = (field(), field());
    let (first_id, second_id) = (first.id(), second.id());
    let root = Widget::column(vec![first, second]).with_layout(fixed(100, 100));
    let root_id = root.id();
    let mut tree = laid_out(root);

    assert_eq!(tree.press_key("Tab"), vec![WidgetEvent::Focus(first_id)]);
    let children = tree.take_children(root_id).expect("children");
    assert!(tree.replace_all_children(root_id, children));

    assert_eq!(tree.focused(), Some(first_id));
    {
        let snapshot = tree.snapshot().expect("snapshot");
        assert!(snapshot.node(first_id).expect("first").state.focused);
        assert!(!snapshot.node(second_id).expect("second").state.focused);
    }
    assert_eq!(
        tree.press_key("Tab"),
        vec![WidgetEvent::Blur(first_id), WidgetEvent::Focus(second_id)]
    );
}
