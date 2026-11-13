use brul_gui::{
    Dimension, KeyModifiers, LayoutStyle, Size, TextEngine, Widget, WidgetEvent, WidgetTree,
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
    assert!(caret.is_some_and(|caret| caret.x >= 0));
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

fn multiline(value: &str) -> Widget {
    Widget::input(value)
        .multiline(true)
        .with_layout(fixed(80, 20))
}

fn focused(tree: WidgetTree) -> WidgetTree {
    let mut tree = tree;
    tree.press_key("Tab");
    tree
}

fn relayout(tree: &mut WidgetTree) {
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
}

#[test]
fn a_multiline_field_inserts_a_newline_and_keeps_the_submit_for_one_line() {
    let input = multiline("ab");
    let id = input.id();
    let mut tree = focused(laid_out(
        Widget::column(vec![input]).with_layout(fixed(100, 40)),
    ));

    assert_eq!(
        tree.press_key("Enter"),
        vec![WidgetEvent::Input(id, "ab\n".to_owned(), 3)]
    );
    assert_eq!(
        tree.press_key("Z"),
        vec![WidgetEvent::Input(id, "ab\nZ".to_owned(), 4)]
    );
}

#[test]
fn vertical_arrows_move_the_caret_between_the_lines() {
    let input = multiline("ab\ncd");
    let id = input.id();
    let mut tree = focused(laid_out(
        Widget::column(vec![input]).with_layout(fixed(100, 40)),
    ));

    assert_eq!(tree.press_key("ArrowUp"), Vec::new());
    assert_eq!(
        tree.press_key("X"),
        vec![WidgetEvent::Input(id, "abX\ncd".to_owned(), 3)]
    );
    relayout(&mut tree);
    assert_eq!(tree.press_key("ArrowDown"), Vec::new());
    assert_eq!(
        tree.press_key("Y"),
        vec![WidgetEvent::Input(id, "abX\ncdY".to_owned(), 7)]
    );
}

#[test]
fn a_long_line_does_not_wrap_in_a_single_line_field() {
    let input = field("abcdefghijklmnopqrstuvwxyz");
    let id = input.id();
    let mut tree = laid_out(Widget::column(vec![input]).with_layout(fixed(100, 40)));
    let text = tree
        .snapshot()
        .expect("snapshot")
        .node(id)
        .and_then(|node| node.text.clone())
        .expect("text");

    let first = text.carets.first().map(|stop| stop.y);
    assert!(text.carets.iter().all(|stop| Some(stop.y) == first));
}

#[test]
fn a_multiline_field_wraps_its_text_and_clips_it() {
    let input = multiline("aaaa bbbb cccc dddd");
    let id = input.id();
    let mut tree = laid_out(Widget::column(vec![input]).with_layout(fixed(100, 40)));
    let node = tree.snapshot().expect("snapshot").node(id).expect("node");
    let text = node.text.as_ref().expect("text");

    assert!(
        text.carets
            .iter()
            .map(|stop| stop.y)
            .collect::<Vec<_>>()
            .windows(2)
            .any(|pair| pair[0] != pair[1])
    );
    assert_eq!(node.clips.len(), 1);
}

#[test]
fn the_field_scrolls_to_keep_the_caret_visible() {
    let input = multiline("a\nb\nc\nd\ne\nf");
    let id = input.id();
    let mut tree = focused(laid_out(
        Widget::column(vec![input]).with_layout(fixed(100, 40)),
    ));
    let node = tree.snapshot().expect("snapshot").node(id).expect("node");

    assert!(node.scroll > 0);
}

#[test]
fn a_click_on_the_second_line_places_the_caret_there() {
    let input = multiline("ab\ncd");
    let id = input.id();
    let mut tree = laid_out(Widget::column(vec![input]).with_layout(fixed(100, 40)));

    let _ = tree.pointer_down(60, 18).expect("down");
    assert_eq!(
        tree.press_key("Z"),
        vec![WidgetEvent::Input(id, "ab\ncdZ".to_owned(), 6)]
    );
}

fn shift() -> KeyModifiers {
    KeyModifiers {
        shift: true,
        ..KeyModifiers::default()
    }
}

#[test]
fn shift_and_arrow_select_text_that_an_arrow_collapses() {
    let input = field("ab");
    let id = input.id();
    let mut tree = laid_out(Widget::column(vec![input]).with_layout(fixed(100, 40)));
    tree.press_key("Tab");
    tree.set_modifiers(shift());

    assert_eq!(tree.press_key("ArrowLeft"), Vec::new());
    let selection = tree
        .snapshot()
        .expect("snapshot")
        .node(id)
        .expect("node")
        .selection
        .clone();
    assert_eq!(selection.len(), 1);
    assert!(selection[0].width > 0);

    tree.set_modifiers(KeyModifiers::default());
    assert_eq!(tree.press_key("ArrowLeft"), Vec::new());
    let selection = tree
        .snapshot()
        .expect("snapshot")
        .node(id)
        .expect("node")
        .selection
        .clone();
    assert!(selection.is_empty());
}

#[test]
fn control_a_selects_everything_and_typing_replaces_it() {
    let (mut tree, id) = focused_field("abc");
    assert_eq!(tree.press_key("End"), Vec::new());
    tree.set_modifiers(KeyModifiers {
        control: true,
        ..KeyModifiers::default()
    });

    assert_eq!(tree.press_key("a"), Vec::new());
    tree.set_modifiers(KeyModifiers::default());
    assert_eq!(
        tree.press_key("Z"),
        vec![WidgetEvent::Input(id, "Z".to_owned(), 1)]
    );
}

#[test]
fn dragging_across_the_field_selects_the_text() {
    let input = field("abc");
    let id = input.id();
    let mut tree = laid_out(Widget::column(vec![input]).with_layout(fixed(100, 40)));

    let _ = tree.pointer_down(60, 5).expect("down");
    let _ = tree.pointer_moved(0, 5).expect("move");
    assert_eq!(
        tree.press_key("Backspace"),
        vec![WidgetEvent::Input(id, String::new(), 0)]
    );
}

#[test]
fn home_and_end_stay_on_the_current_line() {
    let input = multiline("ab\ncd");
    let id = input.id();
    let mut tree = focused(laid_out(
        Widget::column(vec![input]).with_layout(fixed(100, 40)),
    ));

    assert_eq!(tree.press_key("Home"), Vec::new());
    assert_eq!(
        tree.press_key("X"),
        vec![WidgetEvent::Input(id, "ab\nXcd".to_owned(), 4)]
    );
    relayout(&mut tree);
    assert_eq!(tree.press_key("End"), Vec::new());
    assert_eq!(
        tree.press_key("Y"),
        vec![WidgetEvent::Input(id, "ab\nXcdY".to_owned(), 7)]
    );
}

#[test]
fn shift_tab_moves_the_focus_back() {
    let (first, second, third) = (field("a"), field("b"), field("c"));
    let (first_id, second_id) = (first.id(), second.id());
    let mut tree =
        laid_out(Widget::column(vec![first, second, third]).with_layout(fixed(100, 100)));
    tree.press_key("Tab");
    tree.press_key("Tab");
    tree.set_modifiers(shift());

    assert_eq!(
        tree.press_key("Tab"),
        vec![WidgetEvent::Blur(second_id), WidgetEvent::Focus(first_id)]
    );
}
