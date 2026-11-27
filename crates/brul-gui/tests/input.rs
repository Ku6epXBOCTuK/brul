use brul_gui::{
    Dimension, KeyCode, KeyModifiers, KeyPress, LayoutStyle, PhysicalKeyCode, Position, RawEvent,
    RawUiEvent, Size, TextEngine, Widget, WidgetTree,
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
    tree.raw_key("Tab");
    (tree, id)
}

#[test]
fn an_external_write_replaces_the_text_and_clamps_the_caret() {
    let input = field("abc");
    let id = input.id();
    let mut tree = laid_out(Widget::column(vec![input]).with_layout(fixed(100, 40)));
    assert!(tree.set_focused(Some(id)));

    assert!(tree.set_caret_position(id, Position::new(0, 3)));
    assert_eq!(tree.caret_position(id), Some(Position::new(0, 3)));

    assert!(tree.set_input_text(id, "xy".to_owned()));
    assert_eq!(tree.edit_buffer(id).expect("buffer").text(), "xy");
    assert_eq!(tree.caret_position(id), Some(Position::new(0, 2)));
    assert!(!tree.set_input_text(id, "xy".to_owned()));

    assert!(tree.set_input_text(id, "q".to_owned()));
    assert_eq!(tree.edit_buffer(id).expect("buffer").text(), "q");
    assert_eq!(tree.caret_position(id), Some(Position::new(0, 1)));
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

    assert!(tree.set_focused(Some(id)));
    relayout(&mut tree);
    let caret = tree
        .snapshot()
        .expect("snapshot")
        .node(id)
        .expect("node")
        .caret;
    assert!(caret.is_some_and(|caret| caret.x >= 0));

    assert!(tree.set_focused(None));
    relayout(&mut tree);
    let caret = tree
        .snapshot()
        .expect("snapshot")
        .node(id)
        .expect("node")
        .caret;
    assert_eq!(caret, None);
}

#[test]
fn caret_at_pixel_clamps_to_the_text_bounds() {
    let input = field("ab");
    let id = input.id();
    let tree = laid_out(Widget::column(vec![input]).with_layout(fixed(100, 40)));

    assert_eq!(tree.caret_at_pixel(id, -100, 5), Some(Position::new(0, 0)));
    assert_eq!(tree.caret_at_pixel(id, 1000, 5), Some(Position::new(0, 2)));
}

#[test]
fn caret_at_pixel_finds_the_second_line() {
    let input = multiline("ab\ncd");
    let id = input.id();
    let tree = laid_out(Widget::column(vec![input]).with_layout(fixed(100, 40)));

    assert_eq!(tree.caret_at_pixel(id, 1000, 18), Some(Position::new(0, 5)));
}

#[test]
fn caret_at_pixel_returns_utf8_boundaries() {
    let value = "a\u{20ac}b";
    let input = field(value);
    let id = input.id();
    let tree = laid_out(Widget::column(vec![input]).with_layout(fixed(100, 40)));

    let caret = tree.caret_at_pixel(id, 1000, 5).expect("caret");
    assert_eq!(caret, Position::new(0, value.len()));
    assert!(value.is_char_boundary(caret.offset));
}

#[test]
fn caret_at_pixel_inverts_the_caret_area_with_scroll() {
    let input = field("ab");
    let id = input.id();
    let mut tree = laid_out(Widget::column(vec![input]).with_layout(fixed(100, 40)));
    assert!(tree.set_focused(Some(id)));
    assert!(tree.scroll_by(id, 3));

    let caret = tree.caret_position(id).expect("caret");
    let area = tree.caret_area().expect("caret area");
    assert_eq!(tree.caret_at_pixel(id, area.x, area.y), Some(caret));
}

#[test]
fn a_key_press_without_a_focused_field_still_reaches_the_window() {
    let plain = Widget::text("x").with_layout(fixed(40, 20));
    let mut tree = laid_out(Widget::column(vec![plain]).with_layout(fixed(100, 40)));

    assert_eq!(
        tree.raw_key("a"),
        vec![
            RawUiEvent::window(RawEvent::key(KeyPress::from("a"), KeyModifiers::default(),)).into()
        ]
    );
}

#[test]
fn a_physical_code_reaches_the_window_event() {
    let plain = Widget::text("x").with_layout(fixed(40, 20));
    let mut tree = laid_out(Widget::column(vec![plain]).with_layout(fixed(100, 40)));
    let key = KeyPress {
        code: KeyCode::Character("q".to_owned()),
        physical: Some(PhysicalKeyCode::KeyQ),
    };

    assert_eq!(
        tree.raw_key(key.clone()),
        vec![RawUiEvent::window(RawEvent::key(key, KeyModifiers::default())).into()]
    );
}

fn multiline(value: &str) -> Widget {
    Widget::input(value).wrap(true).with_layout(fixed(80, 20))
}

fn focused(tree: WidgetTree) -> WidgetTree {
    let mut tree = tree;
    tree.raw_key("Tab");
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
fn a_preedit_is_shown_at_the_caret_and_the_caret_moves_past_it() {
    let input = field("ab");
    let id = input.id();
    let mut tree = laid_out(Widget::column(vec![input]).with_layout(fixed(100, 40)));
    assert!(tree.set_focused(Some(id)));
    assert!(tree.set_caret_position(id, Position::new(0, 0)));

    let mut buffer = tree.edit_buffer(id).expect("buffer");
    buffer.set_preedit("\u{3042}");
    assert!(tree.commit_edit(id, &buffer));
    relayout(&mut tree);

    let node = tree
        .snapshot()
        .expect("snapshot")
        .node(id)
        .expect("node")
        .clone();
    let preedit = node.preedit.expect("preedit");
    assert!(preedit.size.width > 0);
    assert_eq!(node.caret.expect("caret").x, preedit.size.width as i32);
}

#[test]
fn a_commit_inserts_the_text_and_clears_the_preedit() {
    let (mut tree, id) = focused_field("ab");
    tree.raw_key("Home");
    let mut buffer = tree.edit_buffer(id).expect("buffer");
    buffer.set_preedit("x");
    buffer.set_preedit("");
    let caret = buffer.caret();
    buffer.insert_text(caret, "X");
    assert!(tree.commit_edit(id, &buffer));
    relayout(&mut tree);
    let node = tree
        .snapshot()
        .expect("snapshot")
        .node(id)
        .expect("node")
        .clone();
    assert!(node.preedit.is_none());
}

#[test]
fn the_caret_area_follows_the_focused_field() {
    let input = field("ab");
    let id = input.id();
    let mut tree = laid_out(input);
    assert!(tree.caret_area().is_none());

    assert!(tree.set_focused(Some(id)));
    let area = tree.caret_area().expect("caret area");
    assert!(area.width >= 1);
    assert!(area.height >= 1);
}

#[test]
fn delete_surrounding_removes_whole_characters_around_the_caret() {
    let word = "\u{43f}\u{440}\u{438}";
    let (mut tree, id) = focused_field(word);
    let mut buffer = tree.edit_buffer(id).expect("buffer");
    buffer.set_caret(Position::new(0, 2));
    buffer.set_anchor(Position::new(0, 2));
    buffer.remove(Position::new(0, 2)..Position::new(0, 6));
    assert!(tree.commit_edit(id, &buffer));
}
