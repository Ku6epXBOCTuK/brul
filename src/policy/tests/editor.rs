use super::*;
#[test]
pub(super) fn typing_inserts_at_the_caret_and_emits_input() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "X", KeyModifiers::default())]);

    assert_eq!(text_of(&ui, id), "abX");
    assert_eq!(caret_of(&ui, id), 3);
    assert_eq!(seen(), ["input:abX@3"]);
}

#[test]
pub(super) fn backspace_reports_a_noop_when_the_field_is_empty() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "Backspace", KeyModifiers::default())]);
    assert_eq!(text_of(&ui, id), "a");
    assert_eq!(seen(), ["input:a@1"]);

    SEEN.with(|seen| seen.borrow_mut().clear());
    ui.dispatch_raw(&[key_at(id, "Backspace", KeyModifiers::default())]);
    assert_eq!(text_of(&ui, id), "");
    assert_eq!(seen(), ["input:@0"]);

    SEEN.with(|seen| seen.borrow_mut().clear());
    ui.dispatch_raw(&[key_at(id, "Backspace", KeyModifiers::default())]);
    assert_eq!(seen(), ["noop:Backspace", "key:Backspace"]);
}

#[test]
pub(super) fn control_a_then_typing_replaces_the_selection() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "a", control())]);
    ui.dispatch_raw(&[key_at(id, "Z", KeyModifiers::default())]);

    assert_eq!(text_of(&ui, id), "Z");
    assert_eq!(seen(), ["input:Z@1"]);
}

#[test]
pub(super) fn control_c_copies_only_a_selection() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "a", control())]);
    ui.dispatch_raw(&[key_at(id, "c", control())]);
    assert_eq!(seen(), ["copy:ab"]);

    ui.dispatch_raw(&[key_at(id, "ArrowRight", KeyModifiers::default())]);
    SEEN.with(|seen| seen.borrow_mut().clear());
    ui.dispatch_raw(&[key_at(id, "c", control())]);
    assert!(seen().is_empty());
}

#[test]
pub(super) fn control_x_emits_input_before_copy() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "a", control())]);
    ui.dispatch_raw(&[key_at(id, "x", control())]);

    assert_eq!(text_of(&ui, id), "");
    assert_eq!(seen(), ["input:@0", "copy:ab"]);
}

#[test]
pub(super) fn control_v_asks_for_the_clipboard() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "v", control())]);

    assert_eq!(seen(), ["paste_request"]);
}

#[test]
pub(super) fn control_v_pastes_text_from_the_clipboard_once() {
    let (mut ui, id) = editor_ui("ab", false);
    seed_clipboard(&ui, "xy");

    ui.dispatch_raw(&[key_at(id, "v", control())]);

    assert_eq!(text_of(&ui, id), "abxy");
    assert_eq!(caret_of(&ui, id), 4);
    assert_eq!(seen(), ["paste_request", "paste:xy", "input:abxy@4"]);
}

#[test]
pub(super) fn a_paste_replaces_the_selection() {
    let (mut ui, id) = editor_ui("ab", false);
    seed_clipboard(&ui, "Z");

    ui.dispatch_raw(&[key_at(id, "a", control())]);
    SEEN.with(|seen| seen.borrow_mut().clear());
    ui.dispatch_raw(&[key_at(id, "v", control())]);

    assert_eq!(text_of(&ui, id), "Z");
    assert_eq!(seen(), ["paste_request", "paste:Z", "input:Z@1"]);
}

#[test]
pub(super) fn pasting_normalizes_newlines_by_wrap() {
    let (mut ui, id) = editor_ui("a", true);
    seed_clipboard(&ui, "b\r\nc");
    ui.dispatch_raw(&[key_at(id, "v", control())]);
    assert_eq!(text_of(&ui, id), "ab\nc");
    assert_eq!(seen(), ["paste_request", "paste:b\r\nc", "input:ab\nc@4"]);

    let (mut ui, id) = editor_ui("a", false);
    seed_clipboard(&ui, "b\r\nc");
    ui.dispatch_raw(&[key_at(id, "v", control())]);
    assert_eq!(text_of(&ui, id), "abc");
    assert_eq!(seen(), ["paste_request", "paste:b\r\nc", "input:abc@3"]);
}

#[test]
pub(super) fn a_nonempty_paste_is_inserted_without_a_second_answer() {
    let (mut ui, id) = editor_ui("a", false);

    ui.dispatch_raw(&[paste(id, "q")]);

    assert_eq!(text_of(&ui, id), "aq");
    assert_eq!(seen(), ["paste:q", "input:aq@2"]);
}

#[test]
pub(super) fn ime_preedit_commit_and_delete_surrounding_change_the_buffer() {
    let (mut ui, id) = editor_ui("a", false);

    ui.dispatch_raw(&[ime(id, ImeKind::Preedit("\u{3042}".into()))]);
    assert_eq!(preedit_of(&ui, id), "\u{3042}");
    assert_eq!(text_of(&ui, id), "a");
    assert_eq!(seen(), ["ime:preedit"]);

    SEEN.with(|seen| seen.borrow_mut().clear());
    ui.dispatch_raw(&[ime(id, ImeKind::Commit("X".into()))]);
    assert_eq!(preedit_of(&ui, id), "");
    assert_eq!(text_of(&ui, id), "aX");
    assert_eq!(seen(), ["ime:commit", "input:aX@2"]);

    SEEN.with(|seen| seen.borrow_mut().clear());
    ui.dispatch_raw(&[ime(
        id,
        ImeKind::DeleteSurrounding {
            before_bytes: 1,
            after_bytes: 0,
        },
    )]);
    assert_eq!(text_of(&ui, id), "a");
    assert_eq!(seen(), ["ime:delete_surrounding", "input:a@1"]);
}

#[test]
pub(super) fn ime_enabled_and_disabled_pass_through() {
    let (mut ui, id) = editor_ui("a", false);

    ui.dispatch_raw(&[ime(id, ImeKind::Enabled), ime(id, ImeKind::Disabled)]);

    assert_eq!(text_of(&ui, id), "a");
    assert_eq!(seen(), ["ime:enabled", "ime:disabled"]);
}

#[test]
pub(super) fn ime_delete_surrounding_respects_utf8_boundaries() {
    let (mut ui, id) = editor_ui("a\u{20ac}b", false);

    ui.dispatch_raw(&[ime(
        id,
        ImeKind::DeleteSurrounding {
            before_bytes: 3,
            after_bytes: 0,
        },
    )]);

    assert_eq!(text_of(&ui, id), "a");
    assert_eq!(seen(), ["ime:delete_surrounding", "input:a@1"]);
}

#[test]
pub(super) fn enter_submits_a_single_line_and_inserts_a_newline_when_wrapped() {
    let (mut ui, id) = editor_ui("a", false);
    ui.dispatch_raw(&[key_at(id, "Enter", KeyModifiers::default())]);
    assert_eq!(seen(), ["submit"]);

    let (mut ui, id) = editor_ui("a", true);
    ui.dispatch_raw(&[key_at(id, "Enter", KeyModifiers::default())]);
    assert_eq!(text_of(&ui, id), "a\n");
    assert_eq!(seen(), ["input:a\n@2".to_string()]);
}

#[test]
pub(super) fn arrows_move_the_caret_and_extend_the_selection() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "ArrowLeft", KeyModifiers::default())]);
    assert_eq!(caret_of(&ui, id), 1);
    ui.dispatch_raw(&[key_at(id, "ArrowRight", shift())]);
    let selection = ui.tree.borrow().edit_buffer(id).expect("input").selection();
    assert_eq!((selection.start.offset, selection.end.offset), (1, 2));
    assert!(seen().is_empty());
}

#[test]
pub(super) fn unconsumed_keys_reach_the_keyboard_policy() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "ArrowUp", KeyModifiers::default())]);

    assert_eq!(seen(), ["key:ArrowUp"]);
}

#[test]
pub(super) fn a_handled_key_skips_the_editor() {
    let input = Widget::input("ab").focusable(true);
    let id = input.id();
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble(
        input,
        vec![
            first_handled(&component),
            editor(&component),
            keyboard(&component),
            observer(&component),
        ],
    );
    {
        let mut tree = ui.tree.borrow_mut();
        let mut engine = TextEngine::new();
        tree.layout(
            &mut engine,
            Size {
                width: 200,
                height: 100,
            },
            1.0,
        )
        .expect("layout");
        tree.set_focused(Some(id));
    }
    SEEN.with(|seen| seen.borrow_mut().clear());

    ui.dispatch_raw(&[key_at(id, "X", KeyModifiers::default())]);

    assert_eq!(text_of(&ui, id), "ab");
    assert!(seen().is_empty());
}
#[test]
pub(super) fn ctrl_enter_leaves_a_multiline_field_for_the_window() {
    let (mut ui, id) = editor_ui("a", true);

    ui.dispatch_raw(&[key_at(id, "Enter", control())]);

    assert_eq!(text_of(&ui, id), "a");
    assert_eq!(seen(), ["key:Enter"]);
}

#[test]
pub(super) fn the_caret_area_appears_only_while_the_input_is_focused() {
    let input = Widget::input("ab");
    let id = input.id();
    let ui = assemble_laid_out(
        input,
        Vec::new(),
        Size {
            width: 100,
            height: 40,
        },
    );
    assert!(ui.tree.borrow().caret_area().is_none());

    assert!(ui.tree.borrow_mut().set_focused(Some(id)));
    assert!(ui.tree.borrow().caret_area().is_some());
}

#[test]
pub(super) fn backspace_and_delete_remove_utf8_characters() {
    let (mut ui, id) = editor_ui("\u{43f}\u{440}\u{438}\u{432}\u{435}\u{442}", false);

    ui.dispatch_raw(&[key_at(id, "Backspace", KeyModifiers::default())]);
    assert_eq!(text_of(&ui, id), "\u{43f}\u{440}\u{438}\u{432}\u{435}");
    assert_eq!(caret_of(&ui, id), 10);

    SEEN.with(|seen| seen.borrow_mut().clear());
    ui.dispatch_raw(&[key_at(id, "Home", KeyModifiers::default())]);
    ui.dispatch_raw(&[key_at(id, "Delete", KeyModifiers::default())]);
    assert_eq!(text_of(&ui, id), "\u{440}\u{438}\u{432}\u{435}");
    assert_eq!(caret_of(&ui, id), 0);
    assert_eq!(seen(), ["input:\u{440}\u{438}\u{432}\u{435}@0"]);
}

#[test]
pub(super) fn delete_at_the_end_reports_a_noop_with_modifiers() {
    let (mut ui, id) = editor_ui("a", false);

    ui.dispatch_raw(&[key_at(id, "Delete", shift())]);

    assert_eq!(text_of(&ui, id), "a");
    assert_eq!(seen(), ["noop:Delete", "key:Delete"]);
}

#[test]
pub(super) fn space_inserts_a_space_into_the_field() {
    let (mut ui, id) = editor_ui("a", false);

    ui.dispatch_raw(&[key_at(id, " ", KeyModifiers::default())]);

    assert_eq!(text_of(&ui, id), "a ");
    assert_eq!(caret_of(&ui, id), 2);
    assert_eq!(seen(), ["input:a @2"]);
}

#[test]
pub(super) fn typing_after_an_external_write_uses_the_new_text() {
    let (mut ui, id) = editor_ui("ab", false);
    assert!(ui.tree.borrow_mut().set_input_text(id, "q".into()));

    ui.dispatch_raw(&[key_at(id, "W", KeyModifiers::default())]);

    assert_eq!(text_of(&ui, id), "qW");
    assert_eq!(caret_of(&ui, id), 2);
    assert_eq!(seen(), ["input:qW@2"]);
}

#[test]
pub(super) fn vertical_arrows_move_between_lines() {
    let (mut ui, id) = editor_ui("ab\ncd", true);

    ui.dispatch_raw(&[key_at(id, "ArrowUp", KeyModifiers::default())]);
    ui.dispatch_raw(&[key_at(id, "X", KeyModifiers::default())]);
    assert_eq!(text_of(&ui, id), "abX\ncd");
    assert_eq!(caret_of(&ui, id), 3);
    SEEN.with(|seen| seen.borrow_mut().clear());

    ui.tree
        .borrow_mut()
        .layout(
            &mut TextEngine::new(),
            Size {
                width: 200,
                height: 100,
            },
            1.0,
        )
        .expect("relayout");
    ui.dispatch_raw(&[key_at(id, "ArrowDown", KeyModifiers::default())]);
    assert_eq!(caret_of(&ui, id), 6);
    ui.dispatch_raw(&[key_at(id, "Y", KeyModifiers::default())]);
    assert_eq!(text_of(&ui, id), "abX\ncdY");
    assert_eq!(caret_of(&ui, id), 7);
    assert_eq!(seen(), ["input:abX\ncdY@7"]);
}

#[test]
pub(super) fn home_and_end_stay_on_the_current_line() {
    let (mut ui, id) = editor_ui("ab\ncd", true);

    ui.dispatch_raw(&[key_at(id, "Home", KeyModifiers::default())]);
    assert_eq!(caret_of(&ui, id), 3);
    ui.dispatch_raw(&[key_at(id, "X", KeyModifiers::default())]);
    assert_eq!(text_of(&ui, id), "ab\nXcd");
    assert_eq!(caret_of(&ui, id), 4);
    SEEN.with(|seen| seen.borrow_mut().clear());

    ui.tree
        .borrow_mut()
        .layout(
            &mut TextEngine::new(),
            Size {
                width: 200,
                height: 100,
            },
            1.0,
        )
        .expect("relayout");
    ui.dispatch_raw(&[key_at(id, "End", KeyModifiers::default())]);
    assert_eq!(caret_of(&ui, id), 6);
    ui.dispatch_raw(&[key_at(id, "Y", KeyModifiers::default())]);
    assert_eq!(text_of(&ui, id), "ab\nXcdY");
    assert_eq!(caret_of(&ui, id), 7);
    assert_eq!(seen(), ["input:ab\nXcdY@7"]);
}

#[test]
pub(super) fn a_plain_arrow_collapses_a_shift_selection() {
    let (mut ui, id) = editor_ui("ab", false);

    ui.dispatch_raw(&[key_at(id, "ArrowLeft", shift())]);
    let selection = ui.tree.borrow().edit_buffer(id).expect("input").selection();
    assert_eq!((selection.start.offset, selection.end.offset), (1, 2));

    ui.dispatch_raw(&[key_at(id, "ArrowLeft", KeyModifiers::default())]);
    let selection = ui.tree.borrow().edit_buffer(id).expect("input").selection();
    assert_eq!(selection.start, selection.end);
}

#[test]
pub(super) fn a_click_on_the_second_line_places_the_caret_there() {
    let input = Widget::input("ab\ncd")
        .wrap(true)
        .with_layout(fixed(80, 20));
    let id = input.id();
    let root = Widget::column(vec![input]).with_layout(fixed(100, 40));
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ui = assemble_laid_out(
        root,
        vec![pointer(&component)],
        Size {
            width: 100,
            height: 40,
        },
    );

    ui.dispatch_raw(&[raw(id, RawEvent::press(60, 18, KeyModifiers::default()))]);

    assert_eq!(caret_of(&ui, id), 5);
}
