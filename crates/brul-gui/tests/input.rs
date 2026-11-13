use brul_gui::{Dimension, LayoutStyle, Size, TextEngine, Widget, WidgetEvent, WidgetTree};

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

fn field(value: &str) -> Widget {
    Widget::input(value).with_layout(fixed(80, 20))
}

fn focused_field(value: &str) -> (WidgetTree, brul_gui::WidgetId) {
    let input = field(value);
    let id = input.id();
    let mut tree = laid_out(Widget::column(vec![input]).with_layout(fixed(100, 40)));
    tree.press_key("Tab");
    (tree, id)
}

#[test]
fn typing_inserts_at_the_caret_and_moves_it() {
    let (mut tree, id) = focused_field("ab");

    assert_eq!(tree.press_key("Home"), Vec::new());
    assert_eq!(
        tree.press_key("X"),
        vec![WidgetEvent::Input(id, "Xab".to_owned(), 1)]
    );
    assert_eq!(tree.press_key("ArrowRight"), Vec::new());
    assert_eq!(
        tree.press_key("Y"),
        vec![WidgetEvent::Input(id, "XaYb".to_owned(), 3)]
    );
}

#[test]
fn backspace_and_delete_remove_whole_characters() {
    let word = "\u{43f}\u{440}\u{438}\u{432}\u{435}\u{442}";
    let (mut tree, id) = focused_field(word);

    assert_eq!(tree.press_key("Home"), Vec::new());
    assert_eq!(tree.press_key("End"), Vec::new());
    assert_eq!(
        tree.press_key("Backspace"),
        vec![WidgetEvent::Input(
            id,
            "\u{43f}\u{440}\u{438}\u{432}\u{435}".to_owned(),
            10
        )]
    );
    assert_eq!(tree.press_key("Home"), Vec::new());
    assert_eq!(
        tree.press_key("Delete"),
        vec![WidgetEvent::Input(
            id,
            "\u{440}\u{438}\u{432}\u{435}".to_owned(),
            0
        )]
    );
}

#[test]
fn enter_submits_and_escape_and_tab_leave_the_field() {
    let (first, second) = (field("a"), field("b"));
    let (first_id, second_id) = (first.id(), second.id());
    let mut tree = laid_out(Widget::column(vec![first, second]).with_layout(fixed(100, 100)));
    tree.press_key("Tab");

    assert_eq!(tree.press_key("Enter"), vec![WidgetEvent::Submit(first_id)]);
    assert_eq!(
        tree.press_key("Escape"),
        vec![WidgetEvent::Key("Escape".to_owned())]
    );
    assert_eq!(
        tree.press_key("Tab"),
        vec![WidgetEvent::Blur(first_id), WidgetEvent::Focus(second_id)]
    );
}

#[test]
fn an_external_write_replaces_the_text_and_clamps_the_caret() {
    let (mut tree, id) = focused_field("abc");
    assert_eq!(tree.press_key("End"), Vec::new());

    assert!(tree.set_input_text(id, "xy".to_owned()));
    assert_eq!(tree.press_key("ArrowRight"), Vec::new());
    assert_eq!(
        tree.press_key("Z"),
        vec![WidgetEvent::Input(id, "xyZ".to_owned(), 3)]
    );
    assert!(!tree.set_input_text(id, "xyZ".to_owned()));

    assert!(tree.set_input_text(id, "q".to_owned()));
    assert_eq!(
        tree.press_key("W"),
        vec![WidgetEvent::Input(id, "qW".to_owned(), 2)]
    );
}

#[test]
fn the_caret_is_snapshot_only_while_the_field_is_focused() {
    let input = field("ab");
    let id = input.id();
    let mut tree = laid_out(Widget::column(vec![input]).with_layout(fixed(100, 40)));

    let caret = tree
        .snapshot()
        .expect("snapshot")
        .node(id)
        .expect("node")
        .caret;
    assert_eq!(caret, None);

    tree.press_key("Tab");
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
    let caret = tree
        .snapshot()
        .expect("snapshot")
        .node(id)
        .expect("node")
        .caret;
    assert!(caret.is_some_and(|x| x >= 0));
}

#[test]
fn a_click_places_the_caret() {
    let input = field("ab");
    let id = input.id();
    let mut tree = laid_out(Widget::column(vec![input]).with_layout(fixed(100, 40)));

    let _ = tree.pointer_down(60, 5).expect("down");
    assert_eq!(
        tree.press_key("Z"),
        vec![WidgetEvent::Input(id, "abZ".to_owned(), 3)]
    );
}

#[test]
fn a_key_press_without_a_focused_field_still_reaches_the_window() {
    let plain = Widget::text("x").with_layout(fixed(40, 20));
    let mut tree = laid_out(Widget::column(vec![plain]).with_layout(fixed(100, 40)));

    assert_eq!(tree.press_key("a"), vec![WidgetEvent::Key("a".to_owned())]);
}
